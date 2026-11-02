#include "decode_spiflash.h"
#include "board_settings.h"
#include <string.h>

static inline bool get_pin(const uint8_t* buf, uint32_t idx, uint8_t pin) {
    return (buf[idx] >> pin) & 1;
}

static inline uint32_t circ_idx(uint32_t offset, uint32_t i, uint32_t size) {
    return (offset + i) % size;
}

void spiflash_decode(const uint8_t* buffer, uint32_t length, uint32_t sample_rate,
                     uint8_t cs_pin, uint8_t sclk_pin, uint8_t mosi_pin, uint8_t miso_pin,
                     uint8_t pin_base, uint32_t buf_offset, DECODER_EVENT_ARENA* arena) {
    if (!buffer || length < 10 || !arena || sample_rate == 0) return;

    uint8_t cs = cs_pin - pin_base;
    uint8_t sclk = sclk_pin - pin_base;
    uint8_t mosi = mosi_pin - pin_base;
    uint8_t miso = (miso_pin != 0xFF) ? (miso_pin - pin_base) : 0xFF;

    bool prev_cs = get_pin(buffer, circ_idx(buf_offset, 0, CAPTURE_BUFFER_SIZE), cs);
    bool prev_sclk = get_pin(buffer, circ_idx(buf_offset, 0, CAPTURE_BUFFER_SIZE), sclk);

    bool cs_active = !prev_cs;
    uint32_t trans_start_sample = 0;
    uint8_t bit_count = 0;
    uint8_t mosi_byte = 0;
    uint8_t miso_byte = 0;
    uint16_t byte_count = 0;

    uint8_t opcode = 0;
    uint32_t address = 0;
    uint8_t payload[4];
    memset(payload, 0, sizeof(payload));

    for (uint32_t i = 1; i < length; i++) {
        uint32_t idx = circ_idx(buf_offset, i, CAPTURE_BUFFER_SIZE);
        bool cur_cs = get_pin(buffer, idx, cs);
        bool cur_sclk = get_pin(buffer, idx, sclk);

        /* CS Assertion (falling edge) */
        if (prev_cs && !cur_cs) {
            cs_active = true;
            trans_start_sample = i;
            bit_count = 0;
            mosi_byte = 0;
            miso_byte = 0;
            byte_count = 0;
            opcode = 0;
            address = 0;
            memset(payload, 0, sizeof(payload));
        }

        /* Clock Rising Edge */
        if (cs_active && !prev_sclk && cur_sclk) {
            bool mosi_bit = get_pin(buffer, idx, mosi);
            bool miso_bit = (miso != 0xFF) ? get_pin(buffer, idx, miso) : 0;

            mosi_byte = (mosi_byte << 1) | mosi_bit;
            miso_byte = (miso_byte << 1) | miso_bit;
            bit_count++;

            if (bit_count == 8) {
                if (byte_count == 0) {
                    opcode = mosi_byte;
                } else if (byte_count >= 1 && byte_count <= 3) {
                    /* Most commands have 24-bit address in bytes 1..3 */
                    if (opcode == 0x03 || opcode == 0x0B || opcode == 0x02 ||
                        opcode == 0x20 || opcode == 0x52 || opcode == 0xD8) {
                        address = (address << 8) | mosi_byte;
                    }
                }

                if (opcode == 0x9F && byte_count >= 1 && byte_count <= 4) {
                    /* JEDEC ID on MISO */
                    payload[byte_count - 1] = miso_byte;
                }

                byte_count++;
                bit_count = 0;
                mosi_byte = 0;
                miso_byte = 0;
            }
        }

        /* CS Deassertion (rising edge) */
        if (cs_active && !prev_cs && cur_cs) {
            cs_active = false;
            float start_ts = (float)trans_start_sample / sample_rate;
            float end_ts = (float)i / sample_rate;

            DECODER_EVENT_V2* ev = decoder_arena_add(arena, DECODE_SPIFLASH, EVT_SPIFLASH_CMD,
                                                     start_ts, end_ts, mosi, 1);
            if (ev) {
                ev->spiflash.opcode = opcode;
                ev->spiflash.address = address;
                ev->spiflash.length = byte_count;
                memcpy(ev->spiflash.data, payload, sizeof(payload));
            }
        }

        prev_cs = cur_cs;
        prev_sclk = cur_sclk;
    }
}

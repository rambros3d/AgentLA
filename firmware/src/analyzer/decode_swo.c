#include "decode_swo.h"
#include "board_settings.h"
#include <string.h>

static inline bool get_pin(const uint8_t* buf, uint32_t idx, uint8_t pin) {
    return (buf[idx] >> pin) & 1;
}

static inline uint32_t circ_idx(uint32_t offset, uint32_t i, uint32_t size) {
    return (offset + i) % size;
}

void swo_decode(const uint8_t* buffer, uint32_t length, uint32_t sample_rate,
                uint8_t swo_pin, uint32_t baud_rate, uint8_t pin_base,
                uint32_t buf_offset, DECODER_EVENT_ARENA* arena) {
    if (!buffer || length < 20 || !arena || sample_rate == 0) return;
    if (baud_rate == 0) baud_rate = 1000000; /* Default 1 Mbps */

    uint8_t pin = swo_pin - pin_base;
    float bit_samples = (float)sample_rate / (float)baud_rate;

    /* Extract raw UART bytes from SWO stream */
    uint32_t i = 0;
    while (i + (uint32_t)(10.0f * bit_samples) < length) {
        /* Find start bit (1 -> 0) */
        if (get_pin(buffer, circ_idx(buf_offset, i, CAPTURE_BUFFER_SIZE), pin) != 0) {
            i++;
            continue;
        }

        uint32_t center = i + (uint32_t)(0.5f * bit_samples);
        if (center >= length) break;
        if (get_pin(buffer, circ_idx(buf_offset, center, CAPTURE_BUFFER_SIZE), pin) != 0) {
            i++;
            continue;
        }

        uint32_t byte_start = i;
        uint8_t header = 0;
        for (int b = 0; b < 8; b++) {
            uint32_t s_idx = byte_start + (uint32_t)((1.5f + b) * bit_samples);
            if (s_idx >= length) goto finish;
            uint8_t bit = get_pin(buffer, circ_idx(buf_offset, s_idx, CAPTURE_BUFFER_SIZE), pin);
            header |= (bit << b);
        }

        i = byte_start + (uint32_t)(10.0f * bit_samples);

        /* Parse ITM Packet */
        if ((header & 0x04) == 0 && (header & 0x03) != 0) {
            /* Instrumentation / ITM packet */
            uint8_t port = (header >> 3) & 0x1F;
            uint8_t size_code = header & 0x03;
            uint8_t payload_bytes = (size_code == 1) ? 1 : ((size_code == 2) ? 2 : 4);

            char text_buf[8];
            memset(text_buf, 0, sizeof(text_buf));

            for (uint8_t p = 0; p < payload_bytes; p++) {
                /* Wait for next UART start bit */
                while (i + (uint32_t)(10.0f * bit_samples) < length &&
                       get_pin(buffer, circ_idx(buf_offset, i, CAPTURE_BUFFER_SIZE), pin) == 1) {
                    i++;
                }
                if (i + (uint32_t)(10.0f * bit_samples) >= length) break;

                uint8_t char_byte = 0;
                for (int b = 0; b < 8; b++) {
                    uint32_t s_idx = i + (uint32_t)((1.5f + b) * bit_samples);
                    if (s_idx >= length) break;
                    uint8_t bit = get_pin(buffer, circ_idx(buf_offset, s_idx, CAPTURE_BUFFER_SIZE), pin);
                    char_byte |= (bit << b);
                }
                if (p < sizeof(text_buf)) {
                    text_buf[p] = (char)char_byte;
                }
                i += (uint32_t)(10.0f * bit_samples);
            }

            float start_ts = (float)byte_start / sample_rate;
            float end_ts = (float)i / sample_rate;

            DECODER_EVENT_V2* ev = decoder_arena_add(arena, DECODE_SWO, EVT_SWO_ITM_PRINTF,
                                                     start_ts, end_ts, pin, 1);
            if (ev) {
                ev->swo.port = port;
                ev->swo.len = payload_bytes;
                strncpy(ev->swo.text, text_buf, sizeof(ev->swo.text) - 1);
            }
        }
    }

finish:
    return;
}

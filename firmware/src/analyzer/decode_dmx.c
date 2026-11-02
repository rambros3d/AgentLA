#include "decode_dmx.h"
#include "board_settings.h"
#include <string.h>

static inline bool get_pin(const uint8_t* buf, uint32_t idx, uint8_t pin) {
    return (buf[idx] >> pin) & 1;
}

static inline uint32_t circ_idx(uint32_t offset, uint32_t i, uint32_t size) {
    return (offset + i) % size;
}

void dmx_decode(const uint8_t* buffer, uint32_t length, uint32_t sample_rate,
                uint8_t rx_pin, uint8_t pin_base, uint32_t buf_offset,
                DECODER_EVENT_ARENA* arena) {
    if (!buffer || length < 100 || !arena || sample_rate == 0) return;

    uint8_t pin = rx_pin - pin_base;
    /* DMX-512 standard: 250 kbps -> 4 us per bit */
    float bit_samples = (float)sample_rate / 250000.0f;
    uint32_t min_break_samples = (uint32_t)(88e-6f * sample_rate);
    uint32_t min_mab_samples = (uint32_t)(8e-6f * sample_rate);

    uint32_t i = 0;
    while (i < length) {
        /* Find Break: Line LOW for >= 88 us */
        uint32_t idx = circ_idx(buf_offset, i, CAPTURE_BUFFER_SIZE);
        if (get_pin(buffer, idx, pin) != 0) {
            i++;
            continue;
        }

        uint32_t low_start = i;
        while (i < length && get_pin(buffer, circ_idx(buf_offset, i, CAPTURE_BUFFER_SIZE), pin) == 0) {
            i++;
        }
        uint32_t break_dur = i - low_start;
        if (break_dur < min_break_samples) {
            continue;
        }

        /* Found valid BREAK! Now verify Mark-After-Break (MAB: HIGH >= 8 us) */
        uint32_t mab_start = i;
        while (i < length && get_pin(buffer, circ_idx(buf_offset, i, CAPTURE_BUFFER_SIZE), pin) == 1) {
            i++;
        }
        uint32_t mab_dur = i - mab_start;
        if (mab_dur < min_mab_samples) {
            continue;
        }

        /* Now at falling edge of Start Code byte (start bit) */
        uint32_t frame_start = low_start;
        uint8_t start_code = 0;
        uint16_t slot_count = 0;
        uint8_t slots[9];
        memset(slots, 0, sizeof(slots));

        /* Read Start Code byte (1 start bit, 8 data bits LSB, 2 stop bits) */
        uint32_t byte_start = i;
        if (byte_start + (uint32_t)(10.0f * bit_samples) >= length) break;

        for (int b = 0; b < 8; b++) {
            uint32_t s_idx = byte_start + (uint32_t)((1.5f + b) * bit_samples);
            if (s_idx >= length) break;
            uint8_t bit = get_pin(buffer, circ_idx(buf_offset, s_idx, CAPTURE_BUFFER_SIZE), pin);
            start_code |= (bit << b);
        }

        byte_start += (uint32_t)(11.0f * bit_samples);

        /* Read channel slots (1..512) */
        while (slot_count < 512 && byte_start + (uint32_t)(10.0f * bit_samples) < length) {
            /* Wait for next start bit (falling edge or low level) */
            while (byte_start < length && get_pin(buffer, circ_idx(buf_offset, byte_start, CAPTURE_BUFFER_SIZE), pin) == 1) {
                byte_start++;
            }
            if (byte_start >= length) break;

            /* Check if a new BREAK is occurring */
            uint32_t test_low = byte_start;
            while (test_low < length && (test_low - byte_start < min_break_samples) &&
                   get_pin(buffer, circ_idx(buf_offset, test_low, CAPTURE_BUFFER_SIZE), pin) == 0) {
                test_low++;
            }
            if ((test_low - byte_start) >= min_break_samples) {
                /* New Break encountered */
                i = byte_start;
                break;
            }

            uint8_t slot_val = 0;
            for (int b = 0; b < 8; b++) {
                uint32_t s_idx = byte_start + (uint32_t)((1.5f + b) * bit_samples);
                if (s_idx >= length) break;
                uint8_t bit = get_pin(buffer, circ_idx(buf_offset, s_idx, CAPTURE_BUFFER_SIZE), pin);
                slot_val |= (bit << b);
            }

            if (slot_count < 9) {
                slots[slot_count] = slot_val;
            }
            slot_count++;

            byte_start += (uint32_t)(11.0f * bit_samples);
        }

        /* Emit DMX_FRAME event */
        float start_ts = (float)frame_start / sample_rate;
        float end_ts = (float)byte_start / sample_rate;
        DECODER_EVENT_V2* ev = decoder_arena_add(arena, DECODE_DMX, EVT_DMX_FRAME,
                                                 start_ts, end_ts, pin, 1);
        if (ev) {
            ev->dmx.start_code = start_code;
            ev->dmx.channel_count = slot_count;
            memcpy(ev->dmx.slots, slots, sizeof(slots));
        }

        i = byte_start;
    }
}

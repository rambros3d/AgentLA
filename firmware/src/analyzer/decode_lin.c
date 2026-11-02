#include "decode_lin.h"
#include "board_settings.h"
#include <string.h>

static inline bool get_pin(const uint8_t* buf, uint32_t idx, uint8_t pin) {
    return (buf[idx] >> pin) & 1;
}

static inline uint32_t circ_idx(uint32_t offset, uint32_t i, uint32_t size) {
    return (offset + i) % size;
}

static uint8_t lin_calc_checksum(uint8_t pid, const uint8_t* data, uint8_t len, bool enhanced) {
    uint16_t sum = 0;
    if (enhanced) {
        sum += pid;
    }
    for (uint8_t i = 0; i < len; i++) {
        sum += data[i];
        if (sum > 0xFF) {
            sum = (sum & 0xFF) + 1;
        }
    }
    return (uint8_t)(~sum & 0xFF);
}

void lin_decode(const uint8_t* buffer, uint32_t length, uint32_t sample_rate,
                uint8_t rx_pin, uint32_t baud_rate, uint8_t pin_base,
                uint32_t buf_offset, DECODER_EVENT_ARENA* arena) {
    if (!buffer || length < 50 || !arena || sample_rate == 0) return;
    if (baud_rate == 0) baud_rate = 19200;

    uint8_t pin = rx_pin - pin_base;
    float bit_samples = (float)sample_rate / (float)baud_rate;
    uint32_t min_break_samples = (uint32_t)(13.0f * bit_samples);

    uint32_t i = 0;
    while (i < length) {
        /* Find Sync Break: LOW for >= 13 bit times */
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

        /* Wait for Delimiter (HIGH >= 1 bit) */
        while (i < length && get_pin(buffer, circ_idx(buf_offset, i, CAPTURE_BUFFER_SIZE), pin) == 1) {
            /* Look for falling edge of Sync byte (0x55) */
            uint32_t next = i + 1;
            if (next < length && get_pin(buffer, circ_idx(buf_offset, next, CAPTURE_BUFFER_SIZE), pin) == 0) {
                i = next;
                break;
            }
            i++;
        }
        if (i >= length) break;

        /* Decode Sync Byte (must be 0x55) */
        uint32_t byte_start = i;
        uint8_t sync_byte = 0;
        for (int b = 0; b < 8; b++) {
            uint32_t s_idx = byte_start + (uint32_t)((1.5f + b) * bit_samples);
            if (s_idx >= length) break;
            uint8_t bit = get_pin(buffer, circ_idx(buf_offset, s_idx, CAPTURE_BUFFER_SIZE), pin);
            sync_byte |= (bit << b);
        }

        if (sync_byte != 0x55) {
            continue; /* Not a valid LIN sync */
        }

        byte_start += (uint32_t)(10.0f * bit_samples);

        /* Wait for PID byte falling edge */
        while (byte_start < length && get_pin(buffer, circ_idx(buf_offset, byte_start, CAPTURE_BUFFER_SIZE), pin) == 1) {
            byte_start++;
        }
        if (byte_start >= length) break;

        /* Decode Protected ID (PID) */
        uint8_t pid = 0;
        for (int b = 0; b < 8; b++) {
            uint32_t s_idx = byte_start + (uint32_t)((1.5f + b) * bit_samples);
            if (s_idx >= length) break;
            uint8_t bit = get_pin(buffer, circ_idx(buf_offset, s_idx, CAPTURE_BUFFER_SIZE), pin);
            pid |= (bit << b);
        }

        uint8_t id = pid & 0x3F;
        uint8_t p0 = ((id >> 0) ^ (id >> 1) ^ (id >> 2) ^ (id >> 4)) & 1;
        uint8_t p1 = (~((id >> 1) ^ (id >> 3) ^ (id >> 4) ^ (id >> 5))) & 1;
        bool pid_ok = (((pid >> 6) & 1) == p0) && (((pid >> 7) & 1) == p1);

        byte_start += (uint32_t)(10.0f * bit_samples);

        /* Read Payload Bytes & Checksum */
        uint8_t data[8];
        memset(data, 0, sizeof(data));
        uint8_t data_len = 0;
        uint8_t rx_checksum = 0;

        while (data_len < 8 && byte_start + (uint32_t)(10.0f * bit_samples) < length) {
            /* Find start bit */
            while (byte_start < length && get_pin(buffer, circ_idx(buf_offset, byte_start, CAPTURE_BUFFER_SIZE), pin) == 1) {
                byte_start++;
            }
            if (byte_start >= length) break;

            /* Check if new BREAK */
            uint32_t t_low = byte_start;
            while (t_low < length && (t_low - byte_start < min_break_samples) &&
                   get_pin(buffer, circ_idx(buf_offset, t_low, CAPTURE_BUFFER_SIZE), pin) == 0) {
                t_low++;
            }
            if ((t_low - byte_start) >= min_break_samples) {
                i = byte_start;
                break;
            }

            uint8_t val = 0;
            for (int b = 0; b < 8; b++) {
                uint32_t s_idx = byte_start + (uint32_t)((1.5f + b) * bit_samples);
                if (s_idx >= length) break;
                uint8_t bit = get_pin(buffer, circ_idx(buf_offset, s_idx, CAPTURE_BUFFER_SIZE), pin);
                val |= (bit << b);
            }

            data[data_len++] = val;
            byte_start += (uint32_t)(10.0f * bit_samples);
        }

        /* The last byte read is the Checksum byte */
        bool checksum_ok = false;
        bool is_enhanced = false;
        if (data_len >= 2) {
            rx_checksum = data[data_len - 1];
            uint8_t payload_len = data_len - 1;

            uint8_t calc_classic = lin_calc_checksum(pid, data, payload_len, false);
            uint8_t calc_enhanced = lin_calc_checksum(pid, data, payload_len, true);

            if (rx_checksum == calc_enhanced) {
                checksum_ok = true;
                is_enhanced = true;
            } else if (rx_checksum == calc_classic) {
                checksum_ok = true;
                is_enhanced = false;
            }

            float start_ts = (float)low_start / sample_rate;
            float end_ts = (float)byte_start / sample_rate;
            uint8_t flags = (pid_ok ? 1 : 0) | (checksum_ok ? 2 : 0) | (is_enhanced ? 4 : 0);

            DECODER_EVENT_V2* ev = decoder_arena_add(arena, DECODE_LIN, EVT_LIN_FRAME,
                                                     start_ts, end_ts, pin, flags);
            if (ev) {
                ev->lin.pid = pid;
                ev->lin.id = id;
                ev->lin.length = payload_len;
                ev->lin.checksum = rx_checksum;
                ev->lin.flags = flags;
                memcpy(ev->lin.data, data, (payload_len < 7) ? payload_len : 7);
            }
        }

        i = byte_start;
    }
}

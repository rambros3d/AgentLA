#include "decode_1wire.h"
#include "board_settings.h"
#include <string.h>

static inline bool get_pin(const uint8_t* buf, uint32_t idx, uint8_t pin) {
    return (buf[idx] >> pin) & 1;
}

static inline uint32_t circ_idx(uint32_t offset, uint32_t i, uint32_t size) {
    return (offset + i) % size;
}

static uint8_t dow_crc8(const uint8_t* data, uint8_t len) {
    uint8_t crc = 0;
    for (uint8_t i = 0; i < len; i++) {
        uint8_t byte = data[i];
        for (uint8_t b = 0; b < 8; b++) {
            uint8_t mix = (crc ^ byte) & 0x01;
            crc >>= 1;
            if (mix) {
                crc ^= 0x8C;
            }
            byte >>= 1;
        }
    }
    return crc;
}

void onewire_decode(const uint8_t* buffer, uint32_t length, uint32_t sample_rate,
                    uint8_t pin, uint8_t pin_base, uint32_t buf_offset,
                    DECODER_EVENT_ARENA* arena) {
    if (!buffer || length < 100 || !arena || sample_rate == 0) return;

    uint8_t p = pin - pin_base;
    uint32_t min_reset_samples = (uint32_t)(400e-6f * sample_rate); /* 480-960 us nominal */
    uint32_t max_reset_samples = (uint32_t)(1200e-6f * sample_rate);
    uint32_t sample_offset_15us = (uint32_t)(15e-6f * sample_rate);
    uint32_t min_slot_samples = (uint32_t)(50e-6f * sample_rate);

    uint8_t last_command = 0;
    uint8_t last_family = 0x28; /* Default DS18B20 */
    uint8_t byte_in_cmd = 0;
    uint8_t scratchpad[9];

    uint32_t i = 0;
    while (i < length) {
        /* Search for falling edge */
        while (i < length && get_pin(buffer, circ_idx(buf_offset, i, CAPTURE_BUFFER_SIZE), p) == 1) {
            i++;
        }
        if (i >= length) break;

        uint32_t low_start = i;
        while (i < length && get_pin(buffer, circ_idx(buf_offset, i, CAPTURE_BUFFER_SIZE), p) == 0) {
            i++;
        }
        uint32_t low_dur = i - low_start;

        /* Check for Reset pulse */
        if (low_dur >= min_reset_samples && low_dur <= max_reset_samples) {
            float r_start = (float)low_start / sample_rate;
            float r_end = (float)i / sample_rate;
            decoder_arena_add(arena, DECODE_1WIRE, EVT_1WIRE_RESET, r_start, r_end, p, 1);

            /* Wait for Presence pulse: line goes high, then low for 60-240 us */
            uint32_t wait_start = i;
            while (i < length && get_pin(buffer, circ_idx(buf_offset, i, CAPTURE_BUFFER_SIZE), p) == 1) {
                if (i - wait_start > (uint32_t)(150e-6f * sample_rate)) break;
                i++;
            }
            if (i < length && get_pin(buffer, circ_idx(buf_offset, i, CAPTURE_BUFFER_SIZE), p) == 0) {
                uint32_t pres_start = i;
                while (i < length && get_pin(buffer, circ_idx(buf_offset, i, CAPTURE_BUFFER_SIZE), p) == 0) {
                    i++;
                }
                uint32_t pres_dur = i - pres_start;
                if (pres_dur >= (uint32_t)(40e-6f * sample_rate) && pres_dur <= (uint32_t)(300e-6f * sample_rate)) {
                    decoder_arena_add(arena, DECODE_1WIRE, EVT_1WIRE_PRESENCE,
                                      (float)pres_start / sample_rate, (float)i / sample_rate, p, 1);
                }
            }

            last_command = 0;
            byte_in_cmd = 0;
            continue;
        }

        /* Bit slot: starts at low_start */
        uint8_t current_byte = 0;
        uint32_t byte_start_sample = low_start;
        bool valid_byte = true;

        for (int b = 0; b < 8; b++) {
            if (b > 0) {
                /* Wait for next falling edge */
                while (i < length && get_pin(buffer, circ_idx(buf_offset, i, CAPTURE_BUFFER_SIZE), p) == 1) {
                    i++;
                }
                if (i >= length) { valid_byte = false; break; }
                low_start = i;
                while (i < length && get_pin(buffer, circ_idx(buf_offset, i, CAPTURE_BUFFER_SIZE), p) == 0) {
                    i++;
                }
            }

            /* Sample at 15 us from slot start */
            uint32_t sample_pos = low_start + sample_offset_15us;
            if (sample_pos >= length) { valid_byte = false; break; }
            uint8_t bit = get_pin(buffer, circ_idx(buf_offset, sample_pos, CAPTURE_BUFFER_SIZE), p);
            current_byte |= (bit << b);

            /* Wait until slot ends (at least 60 us from low_start) */
            uint32_t slot_end = low_start + min_slot_samples;
            if (i < slot_end) i = slot_end;
        }

        if (!valid_byte) break;

        /* Process 1-Wire byte */
        if (last_command == 0) {
            /* This byte is a Command! */
            last_command = current_byte;
            byte_in_cmd = 0;
        } else if (last_command == 0x33) {
            /* Read ROM sequence: 8 bytes */
            scratchpad[byte_in_cmd++] = current_byte;
            if (byte_in_cmd == 8) {
                uint8_t fam = scratchpad[0];
                last_family = fam;
                uint8_t crc = scratchpad[7];
                uint8_t calc_crc = dow_crc8(scratchpad, 7);
                bool crc_ok = (calc_crc == crc);

                float start_ts = (float)byte_start_sample / sample_rate;
                float end_ts = (float)i / sample_rate;
                DECODER_EVENT_V2* ev = decoder_arena_add(arena, DECODE_1WIRE, EVT_1WIRE_ROM,
                                                         start_ts, end_ts, p, crc_ok ? 1 : 0);
                if (ev) {
                    ev->onewire.family = fam;
                    ev->onewire.crc = crc;
                    ev->onewire.flags = crc_ok ? 1 : 0;
                    memcpy(ev->onewire.rom, &scratchpad[1], 5);
                }
                last_command = 0;
                byte_in_cmd = 0;
            }
        } else if (last_command == 0xBE) {
            /* Read Scratchpad: bytes 0 and 1 are temperature */
            scratchpad[byte_in_cmd++] = current_byte;
            if (byte_in_cmd >= 2) {
                int16_t raw_temp = (int16_t)(((uint16_t)scratchpad[1] << 8) | scratchpad[0]);
                float temp_c = (float)raw_temp / 16.0f;

                float start_ts = (float)byte_start_sample / sample_rate;
                float end_ts = (float)i / sample_rate;

                DECODER_EVENT_V2* ev = decoder_arena_add(arena, DECODE_1WIRE, EVT_1WIRE_TEMP,
                                                         start_ts, end_ts, p, 1);
                if (ev) {
                    ev->onewire.temp_c = temp_c;
                    ev->onewire.family = last_family;
                    ev->onewire.flags = 3; /* crc_ok | has_temp */
                }
                if (byte_in_cmd == 9) {
                    last_command = 0;
                    byte_in_cmd = 0;
                }
            }
        } else if (last_command == 0x55) {
            /* Match ROM (8 bytes) */
            byte_in_cmd++;
            if (byte_in_cmd == 8) {
                last_command = 0;
                byte_in_cmd = 0;
            }
        } else if (last_command == 0xCC) {
            /* Skip ROM: next byte will be functional command (e.g. 0xBE or 0x44) */
            last_command = current_byte;
            byte_in_cmd = 0;
        } else {
            byte_in_cmd++;
        }
    }
}

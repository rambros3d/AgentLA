#include "decode_i3c.h"
#include "board_settings.h"
#include <string.h>

static inline bool get_pin(const uint8_t* buf, uint32_t idx, uint8_t pin) {
    return (buf[idx] >> pin) & 1;
}

static inline uint32_t circ_idx(uint32_t offset, uint32_t i, uint32_t size) {
    return (offset + i) % size;
}

static inline uint8_t parity8(uint8_t val) {
    val ^= val >> 4;
    val ^= val >> 2;
    val ^= val >> 1;
    return val & 1;
}

void i3c_decode(const uint8_t* buffer, uint32_t length, uint32_t sample_rate,
                uint8_t sda_pin, uint8_t scl_pin, uint8_t pin_base,
                uint32_t buf_offset, DECODER_EVENT_ARENA* arena) {
    if (!buffer || length < 10 || !arena || sample_rate == 0) return;

    uint8_t sda = sda_pin - pin_base;
    uint8_t scl = scl_pin - pin_base;

    bool prev_sda = get_pin(buffer, circ_idx(buf_offset, 0, CAPTURE_BUFFER_SIZE), sda);
    bool prev_scl = get_pin(buffer, circ_idx(buf_offset, 0, CAPTURE_BUFFER_SIZE), scl);

    bool in_frame = false;
    uint8_t bit_count = 0;
    uint8_t byte_val = 0;
    uint8_t byte_idx = 0;
    uint8_t ccc_code = 0;
    uint8_t dyn_addr = 0;
    uint32_t frame_start = 0;

    for (uint32_t i = 1; i < length; i++) {
        uint32_t idx = circ_idx(buf_offset, i, CAPTURE_BUFFER_SIZE);
        bool cur_sda = get_pin(buffer, idx, sda);
        bool cur_scl = get_pin(buffer, idx, scl);

        /* START condition: SDA falls while SCL is HIGH */
        if (prev_scl && cur_scl && prev_sda && !cur_sda) {
            in_frame = true;
            bit_count = 0;
            byte_val = 0;
            byte_idx = 0;
            ccc_code = 0;
            dyn_addr = 0;
            frame_start = i;
            prev_sda = cur_sda;
            prev_scl = cur_scl;
            continue;
        }

        /* STOP condition: SDA rises while SCL is HIGH */
        if (prev_scl && cur_scl && !prev_sda && cur_sda && in_frame) {
            in_frame = false;
            prev_sda = cur_sda;
            prev_scl = cur_scl;
            continue;
        }

        /* Clock rising edge -> Sample SDA */
        if (!prev_scl && cur_scl && in_frame) {
            if (bit_count < 8) {
                byte_val = (byte_val << 1) | cur_sda;
                bit_count++;
            } else {
                /* 9th bit: ACK in Header or Transition T bit in SDR transfer */
                uint8_t t_bit = cur_sda;
                float ts = (float)i / sample_rate;

                if (byte_idx == 0) {
                    /* Byte 0 is Address / Header: e.g. 0x7E + W (0xFC) */
                    if ((byte_val >> 1) == 0x7E) {
                        decoder_arena_add(arena, DECODE_I3C, EVT_I3C_BROADCAST,
                                          (float)frame_start / sample_rate, ts, sda, 1);
                    }
                } else if (byte_idx == 1) {
                    /* Byte 1 is CCC code */
                    ccc_code = byte_val;
                    uint8_t expected_t = ~parity8(byte_val) & 1;
                    bool parity_ok = (t_bit == expected_t);

                    if (!parity_ok) {
                        decoder_arena_add(arena, DECODE_I3C, EVT_I3C_ERR_PARITY,
                                          ts, ts, sda, 0);
                    }

                    DECODER_EVENT_V2* ev = decoder_arena_add(arena, DECODE_I3C, EVT_I3C_CCC,
                                                             (float)frame_start / sample_rate, ts,
                                                             sda, parity_ok ? 1 : 0);
                    if (ev) {
                        ev->i3c.ccc_code = ccc_code;
                        ev->i3c.dyn_addr = 0;
                        ev->i3c.is_direct = (ccc_code >= 0x80) ? 1 : 0;
                        ev->i3c.flags = parity_ok ? 1 : 0;
                    }
                } else {
                    /* Subsequent bytes in CCC transfer */
                    uint8_t expected_t = ~parity8(byte_val) & 1;
                    bool parity_ok = (t_bit == expected_t);

                    if (!parity_ok) {
                        decoder_arena_add(arena, DECODE_I3C, EVT_I3C_ERR_PARITY,
                                          ts, ts, sda, 0);
                    }

                    /* If ENTDAA (0x07), dynamic address assignment occurs after 8 payload bytes (PID+BCR+DCR) */
                    if (ccc_code == 0x07 && byte_idx >= 9) {
                        dyn_addr = byte_val >> 1;
                        DECODER_EVENT_V2* ev = decoder_arena_add(arena, DECODE_I3C, EVT_I3C_CCC,
                                                                 (float)frame_start / sample_rate, ts,
                                                                 sda, parity_ok ? 1 : 0);
                        if (ev) {
                            ev->i3c.ccc_code = ccc_code;
                            ev->i3c.dyn_addr = dyn_addr;
                            ev->i3c.is_direct = 0;
                            ev->i3c.flags = parity_ok ? 1 : 0;
                        }
                    } else if (ccc_code == 0x86 && byte_idx == 2) {
                        /* SETDASA direct CCC dynamic address */
                        dyn_addr = byte_val >> 1;
                        DECODER_EVENT_V2* ev = decoder_arena_add(arena, DECODE_I3C, EVT_I3C_CCC,
                                                                 (float)frame_start / sample_rate, ts,
                                                                 sda, parity_ok ? 1 : 0);
                        if (ev) {
                            ev->i3c.ccc_code = ccc_code;
                            ev->i3c.dyn_addr = dyn_addr;
                            ev->i3c.is_direct = 1;
                            ev->i3c.flags = parity_ok ? 1 : 0;
                        }
                    }
                }

                byte_idx++;
                bit_count = 0;
                byte_val = 0;
            }
        }

        prev_sda = cur_sda;
        prev_scl = cur_scl;
    }
}

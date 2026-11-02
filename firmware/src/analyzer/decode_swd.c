#include "decode_swd.h"
#include "board_settings.h"
#include <string.h>

static inline bool get_pin(const uint8_t* buf, uint32_t idx, uint8_t pin) {
    return (buf[idx] >> pin) & 1;
}

static inline uint32_t circ_idx(uint32_t offset, uint32_t i, uint32_t size) {
    return (offset + i) % size;
}

static inline uint8_t parity32(uint32_t v) {
    v ^= v >> 16;
    v ^= v >> 8;
    v ^= v >> 4;
    v ^= v >> 2;
    v ^= v >> 1;
    return v & 1;
}

void swd_decode(const uint8_t* buffer, uint32_t length, uint32_t sample_rate,
                uint8_t clk_pin, uint8_t dio_pin, uint8_t pin_base,
                uint32_t buf_offset, DECODER_EVENT_ARENA* arena) {
    if (!buffer || length < 20 || !arena || sample_rate == 0) return;

    uint8_t clk = clk_pin - pin_base;
    uint8_t dio = dio_pin - pin_base;

    bool prev_clk = get_pin(buffer, circ_idx(buf_offset, 0, CAPTURE_BUFFER_SIZE), clk);

    /* Collect clock rising edges */
    uint32_t i = 1;
    while (i < length) {
        uint32_t idx = circ_idx(buf_offset, i, CAPTURE_BUFFER_SIZE);
        bool cur_clk = get_pin(buffer, idx, clk);

        /* Wait for clock rising edge */
        if (!prev_clk && cur_clk) {
            bool dio_bit = get_pin(buffer, idx, dio);
            if (dio_bit == 1) {
                /* Potential Start bit of Request */
                uint32_t req_start_sample = i;
                uint8_t req_bits[8];
                req_bits[0] = 1;

                /* Read remaining 7 bits of request on subsequent rising edges */
                bool req_ok = true;
                for (int b = 1; b < 8; b++) {
                    while (++i < length) {
                        uint32_t c_idx = circ_idx(buf_offset, i, CAPTURE_BUFFER_SIZE);
                        bool clk_val = get_pin(buffer, c_idx, clk);
                        if (!prev_clk && clk_val) {
                            req_bits[b] = get_pin(buffer, c_idx, dio);
                            prev_clk = clk_val;
                            break;
                        }
                        prev_clk = clk_val;
                    }
                    if (i >= length) { req_ok = false; break; }
                }

                if (!req_ok) break;

                /* Parse Request */
                uint8_t ap_ndp = req_bits[1];
                uint8_t r_nw = req_bits[2];
                uint8_t a2 = req_bits[3];
                uint8_t a3 = req_bits[4];
                uint8_t parity = req_bits[5];
                uint8_t stop = req_bits[6];
                uint8_t park = req_bits[7];

                uint8_t expected_parity = (ap_ndp ^ r_nw ^ a2 ^ a3) & 1;

                if (stop != 0 || park != 1 || parity != expected_parity) {
                    /* Not a valid SWD request packet */
                    continue;
                }

                /* 1 Trn clock cycle */
                while (++i < length) {
                    uint32_t c_idx = circ_idx(buf_offset, i, CAPTURE_BUFFER_SIZE);
                    bool clk_val = get_pin(buffer, c_idx, clk);
                    if (!prev_clk && clk_val) {
                        prev_clk = clk_val;
                        break;
                    }
                    prev_clk = clk_val;
                }

                /* Read 3-bit ACK (LSB first) */
                uint8_t ack = 0;
                for (int b = 0; b < 3; b++) {
                    while (++i < length) {
                        uint32_t c_idx = circ_idx(buf_offset, i, CAPTURE_BUFFER_SIZE);
                        bool clk_val = get_pin(buffer, c_idx, clk);
                        if (!prev_clk && clk_val) {
                            ack |= (get_pin(buffer, c_idx, dio) << b);
                            prev_clk = clk_val;
                            break;
                        }
                        prev_clk = clk_val;
                    }
                }

                if (ack == 1) { /* ACK OK (0b001) */
                    uint32_t data_val = 0;
                    uint8_t data_parity = 0;

                    if (r_nw == 1) {
                        /* Read data phase: 32 data bits + 1 parity bit */
                        for (int b = 0; b < 32; b++) {
                            while (++i < length) {
                                uint32_t c_idx = circ_idx(buf_offset, i, CAPTURE_BUFFER_SIZE);
                                bool clk_val = get_pin(buffer, c_idx, clk);
                                if (!prev_clk && clk_val) {
                                    if (get_pin(buffer, c_idx, dio)) data_val |= (1UL << b);
                                    prev_clk = clk_val;
                                    break;
                                }
                                prev_clk = clk_val;
                            }
                        }
                        /* Parity bit */
                        while (++i < length) {
                            uint32_t c_idx = circ_idx(buf_offset, i, CAPTURE_BUFFER_SIZE);
                            bool clk_val = get_pin(buffer, c_idx, clk);
                            if (!prev_clk && clk_val) {
                                data_parity = get_pin(buffer, c_idx, dio);
                                prev_clk = clk_val;
                                break;
                            }
                            prev_clk = clk_val;
                        }

                        bool p_ok = (parity32(data_val) == data_parity);
                        uint8_t reg_addr = (a3 << 3) | (a2 << 2);

                        float start_ts = (float)req_start_sample / sample_rate;
                        float end_ts = (float)i / sample_rate;
                        EVENT_TYPE ev_type = ap_ndp ? EVT_SWD_AP_READ : EVT_SWD_DP_READ;

                        DECODER_EVENT_V2* ev = decoder_arena_add(arena, DECODE_SWD, ev_type,
                                                                 start_ts, end_ts, dio, p_ok ? 1 : 0);
                        if (ev) {
                            ev->swd.data = data_val;
                            ev->swd.reg_addr = reg_addr;
                            ev->swd.ap_ndp = ap_ndp;
                            ev->swd.r_nw = 1;
                            ev->swd.ack = ack;
                            ev->swd.flags = p_ok ? 1 : 0;
                        }
                    } else {
                        /* Write data phase: 1 Trn clock + 32 data bits + 1 parity bit */
                        while (++i < length) {
                            uint32_t c_idx = circ_idx(buf_offset, i, CAPTURE_BUFFER_SIZE);
                            bool clk_val = get_pin(buffer, c_idx, clk);
                            if (!prev_clk && clk_val) {
                                prev_clk = clk_val;
                                break;
                            }
                            prev_clk = clk_val;
                        }

                        for (int b = 0; b < 32; b++) {
                            while (++i < length) {
                                uint32_t c_idx = circ_idx(buf_offset, i, CAPTURE_BUFFER_SIZE);
                                bool clk_val = get_pin(buffer, c_idx, clk);
                                if (!prev_clk && clk_val) {
                                    if (get_pin(buffer, c_idx, dio)) data_val |= (1UL << b);
                                    prev_clk = clk_val;
                                    break;
                                }
                                prev_clk = clk_val;
                            }
                        }
                        /* Parity bit */
                        while (++i < length) {
                            uint32_t c_idx = circ_idx(buf_offset, i, CAPTURE_BUFFER_SIZE);
                            bool clk_val = get_pin(buffer, c_idx, clk);
                            if (!prev_clk && clk_val) {
                                data_parity = get_pin(buffer, c_idx, dio);
                                prev_clk = clk_val;
                                break;
                            }
                            prev_clk = clk_val;
                        }

                        bool p_ok = (parity32(data_val) == data_parity);
                        uint8_t reg_addr = (a3 << 3) | (a2 << 2);

                        float start_ts = (float)req_start_sample / sample_rate;
                        float end_ts = (float)i / sample_rate;
                        EVENT_TYPE ev_type = ap_ndp ? EVT_SWD_AP_WRITE : EVT_SWD_DP_WRITE;

                        DECODER_EVENT_V2* ev = decoder_arena_add(arena, DECODE_SWD, ev_type,
                                                                 start_ts, end_ts, dio, p_ok ? 1 : 0);
                        if (ev) {
                            ev->swd.data = data_val;
                            ev->swd.reg_addr = reg_addr;
                            ev->swd.ap_ndp = ap_ndp;
                            ev->swd.r_nw = 0;
                            ev->swd.ack = ack;
                            ev->swd.flags = p_ok ? 1 : 0;
                        }
                    }
                }
            }
        }
        prev_clk = cur_clk;
        i++;
    }
}

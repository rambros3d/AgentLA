#include "decode_can.h"
#include "board_settings.h"
#include <string.h>

static inline bool get_pin(const uint8_t* buf, uint32_t idx, uint8_t pin) {
    return (buf[idx] >> pin) & 1;
}

static inline uint32_t circ_idx(uint32_t offset, uint32_t i, uint32_t size) {
    return (offset + i) % size;
}

/* CAN-FD DLC to payload length mapping */
static const uint8_t fd_dlc_bytes[] = {
    0, 1, 2, 3, 4, 5, 6, 7, 8, 12, 16, 20, 24, 32, 48, 64
};

static inline uint8_t dlc_to_length(uint8_t dlc, bool is_fd) {
    if (!is_fd) {
        return dlc > 8 ? 8 : dlc;
    }
    return (dlc <= 15) ? fd_dlc_bytes[dlc] : 64;
}

static inline uint16_t crc15_update(uint16_t crc, uint8_t bit) {
    uint8_t msb = (crc >> 14) & 1;
    crc = (crc << 1) & 0x7FFF;
    if (msb ^ bit) {
        crc ^= 0x4599;
    }
    return crc;
}

static inline uint32_t crc17_update(uint32_t crc, uint8_t bit) {
    uint8_t msb = (crc >> 16) & 1;
    crc = (crc << 1) & 0x1FFFF;
    if (msb ^ bit) {
        crc ^= 0x1685B;
    }
    return crc;
}

static inline uint32_t crc21_update(uint32_t crc, uint8_t bit) {
    uint8_t msb = (crc >> 20) & 1;
    crc = (crc << 1) & 0x1FFFFF;
    if (msb ^ bit) {
        crc ^= 0x102899;
    }
    return crc;
}

void can_decode(const uint8_t* buffer, uint32_t length, uint32_t sample_rate,
                uint8_t rx_pin, uint32_t nominal_baud, uint32_t data_baud,
                uint8_t sample_point_percent, uint8_t pin_base, uint32_t buf_offset,
                DECODER_EVENT_ARENA* arena) {
    if (!buffer || length < 10 || !arena || sample_rate == 0) return;
    if (nominal_baud == 0) nominal_baud = 500000;
    if (data_baud == 0) data_baud = nominal_baud;
    if (sample_point_percent == 0 || sample_point_percent > 100) sample_point_percent = 75;

    uint8_t pin = rx_pin - pin_base;
    float nom_bit_samples = (float)sample_rate / (float)nominal_baud;
    float dat_bit_samples = (float)sample_rate / (float)data_baud;
    float sp_fraction = (float)sample_point_percent / 100.0f;

    uint32_t i = 0;
    while (i + 1 < length) {
        /* Wait for idle high followed by falling edge (SOF) */
        uint32_t idx_prev = circ_idx(buf_offset, i, CAPTURE_BUFFER_SIZE);
        uint32_t idx_cur = circ_idx(buf_offset, i + 1, CAPTURE_BUFFER_SIZE);
        bool prev_val = get_pin(buffer, idx_prev, pin);
        bool cur_val = get_pin(buffer, idx_cur, pin);

        if (!prev_val || cur_val) {
            i++;
            continue;
        }

        /* Falling edge found at i + 1 -> candidate SOF */
        uint32_t sof_sample = i + 1;
        float bit_pos = (float)sof_sample;
        float cur_bit_samples = nom_bit_samples;

        uint8_t consecutive = 1;
        uint8_t last_bit = 0; /* SOF is 0 */
        bool stuff_err = false;
        bool destuff_active = true;

        /* Frame fields */
        uint32_t arb_id = 0;
        bool is_extended = false;
        bool is_fd = false;
        bool is_brs = false;
        uint8_t dlc = 0;
        uint8_t payload[64];
        memset(payload, 0, sizeof(payload));
        uint8_t payload_len = 0;
        uint32_t rx_crc = 0;
        uint32_t calc_crc = 0;

        /* CRC initialization */
        uint16_t c15 = 0;
        uint32_t c17 = 0;
        uint32_t c21 = 0;
        c15 = crc15_update(c15, 0); /* Include SOF (0) in CRC */
        c17 = crc17_update(c17, 0);
        c21 = crc21_update(c21, 0);

        /* Helper lambda-like macro to sample next bit with de-stuffing */
        #define SAMPLE_RAW_BIT(out_bit) do { \
            bit_pos += cur_bit_samples; \
            uint32_t s_idx = (uint32_t)(bit_pos + sp_fraction * cur_bit_samples); \
            if (s_idx >= length) goto frame_end; \
            out_bit = get_pin(buffer, circ_idx(buf_offset, s_idx, CAPTURE_BUFFER_SIZE), pin); \
        } while(0)

        #define SAMPLE_DESTUFFED_BIT(out_bit) do { \
            SAMPLE_RAW_BIT(out_bit); \
            if (destuff_active) { \
                if (out_bit == last_bit) { \
                    consecutive++; \
                    if (consecutive == 5) { \
                        /* Next bit must be stuff bit */ \
                        uint8_t s_bit; \
                        SAMPLE_RAW_BIT(s_bit); \
                        if (s_bit == last_bit) { \
                            stuff_err = true; \
                            decoder_arena_add(arena, DECODE_CAN, EVT_CAN_ERR_STUFF, \
                                              (float)(bit_pos) / sample_rate, \
                                              (float)(bit_pos + cur_bit_samples) / sample_rate, \
                                              pin, 0); \
                            goto frame_end; \
                        } \
                        consecutive = 1; \
                        last_bit = s_bit; \
                    } \
                } else { \
                    consecutive = 1; \
                    last_bit = out_bit; \
                } \
            } \
        } while(0)

        /* 1. Standard 11-bit ID */
        for (int b = 10; b >= 0; b--) {
            uint8_t bit;
            SAMPLE_DESTUFFED_BIT(bit);
            arb_id = (arb_id << 1) | bit;
            c15 = crc15_update(c15, bit);
            c17 = crc17_update(c17, bit);
            c21 = crc21_update(c21, bit);
        }

        /* 2. RTR or SRR bit */
        uint8_t rtr_srr;
        SAMPLE_DESTUFFED_BIT(rtr_srr);
        c15 = crc15_update(c15, rtr_srr);
        c17 = crc17_update(c17, rtr_srr);
        c21 = crc21_update(c21, rtr_srr);

        /* 3. IDE bit */
        uint8_t ide;
        SAMPLE_DESTUFFED_BIT(ide);
        c15 = crc15_update(c15, ide);
        c17 = crc17_update(c17, ide);
        c21 = crc21_update(c21, ide);

        if (ide == 1) {
            is_extended = true;
            /* 18-bit Extended ID */
            uint32_t ext_id = 0;
            for (int b = 17; b >= 0; b--) {
                uint8_t bit;
                SAMPLE_DESTUFFED_BIT(bit);
                ext_id = (ext_id << 1) | bit;
                c15 = crc15_update(c15, bit);
                c17 = crc17_update(c17, bit);
                c21 = crc21_update(c21, bit);
            }
            arb_id = (arb_id << 18) | ext_id;

            /* RTR bit for extended */
            uint8_t ext_rtr;
            SAMPLE_DESTUFFED_BIT(ext_rtr);
            c15 = crc15_update(c15, ext_rtr);
            c17 = crc17_update(c17, ext_rtr);
            c21 = crc21_update(c21, ext_rtr);

            /* r1 bit */
            uint8_t r1;
            SAMPLE_DESTUFFED_BIT(r1);
            c15 = crc15_update(c15, r1);
            c17 = crc17_update(c17, r1);
            c21 = crc21_update(c21, r1);
        }

        /* r0 / FDF bit */
        uint8_t fdf;
        SAMPLE_DESTUFFED_BIT(fdf);
        c15 = crc15_update(c15, fdf);
        c17 = crc17_update(c17, fdf);
        c21 = crc21_update(c21, fdf);

        if (fdf == 1) {
            is_fd = true;
            /* Reserved bit res */
            uint8_t res;
            SAMPLE_DESTUFFED_BIT(res);
            c17 = crc17_update(c17, res);
            c21 = crc21_update(c21, res);

            /* BRS bit */
            uint8_t brs;
            SAMPLE_DESTUFFED_BIT(brs);
            if (brs == 1) {
                is_brs = true;
                cur_bit_samples = dat_bit_samples;
            }
            c17 = crc17_update(c17, brs);
            c21 = crc21_update(c21, brs);

            /* ESI bit */
            uint8_t esi;
            SAMPLE_DESTUFFED_BIT(esi);
            c17 = crc17_update(c17, esi);
            c21 = crc21_update(c21, esi);
        }

        /* DLC (4 bits) */
        for (int b = 3; b >= 0; b--) {
            uint8_t bit;
            SAMPLE_DESTUFFED_BIT(bit);
            dlc = (dlc << 1) | bit;
            c15 = crc15_update(c15, bit);
            c17 = crc17_update(c17, bit);
            c21 = crc21_update(c21, bit);
        }

        payload_len = dlc_to_length(dlc, is_fd);

        /* Data payload bytes */
        for (uint8_t p = 0; p < payload_len; p++) {
            uint8_t byte_val = 0;
            for (int b = 7; b >= 0; b--) {
                uint8_t bit;
                SAMPLE_DESTUFFED_BIT(bit);
                byte_val = (byte_val << 1) | bit;
                c15 = crc15_update(c15, bit);
                c17 = crc17_update(c17, bit);
                c21 = crc21_update(c21, bit);
            }
            if (p < 64) payload[p] = byte_val;
        }

        /* CRC Field */
        uint8_t crc_bits = 15;
        if (is_fd) {
            crc_bits = (payload_len <= 16) ? 17 : 21;
        }

        for (int b = crc_bits - 1; b >= 0; b--) {
            uint8_t bit;
            SAMPLE_DESTUFFED_BIT(bit);
            rx_crc = (rx_crc << 1) | bit;
        }

        /* Destuffing ends after CRC field */
        destuff_active = false;

        /* CRC Delimiter (1 bit recessive) */
        uint8_t crc_delim;
        SAMPLE_RAW_BIT(crc_delim);
        if (is_brs) {
            cur_bit_samples = nom_bit_samples; /* Switch back to nominal baud */
        }

        /* Check CRC */
        bool crc_ok = false;
        if (!is_fd) {
            calc_crc = c15;
            crc_ok = (rx_crc == calc_crc);
        } else if (payload_len <= 16) {
            calc_crc = c17;
            crc_ok = (rx_crc == calc_crc);
        } else {
            calc_crc = c21;
            crc_ok = (rx_crc == calc_crc);
        }

        /* Emit Decoded CAN event */
        float start_ts = (float)sof_sample / sample_rate;
        float end_ts = (float)bit_pos / sample_rate;
        uint8_t flags = (is_extended ? 1 : 0) |
                        (is_fd ? 2 : 0) |
                        (is_brs ? 4 : 0) |
                        (crc_ok ? 8 : 0);

        DECODER_EVENT_V2* ev = decoder_arena_add(arena, DECODE_CAN, EVT_CAN_FRAME,
                                                 start_ts, end_ts, pin, flags);
        if (ev) {
            ev->can.id = arb_id;
            ev->can.crc = (uint16_t)(rx_crc & 0xFFFF);
            ev->can.dlc = payload_len;
            ev->can.flags = flags;
            ev->can.data[0] = payload[0];
            ev->can.data[1] = payload[1];
            ev->can.data[2] = payload[2];
            ev->can.data[3] = payload[3];
        }

        #undef SAMPLE_RAW_BIT
        #undef SAMPLE_DESTUFFED_BIT

frame_end:
        /* Fast forward sample index to current bit position */
        if ((uint32_t)bit_pos > i) {
            i = (uint32_t)bit_pos + (uint32_t)nom_bit_samples;
        } else {
            i++;
        }
    }
}

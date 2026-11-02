#include "decode_modbus.h"
#include "board_settings.h"
#include <string.h>

static inline bool get_pin(const uint8_t* buf, uint32_t idx, uint8_t pin) {
    return (buf[idx] >> pin) & 1;
}

static inline uint32_t circ_idx(uint32_t offset, uint32_t i, uint32_t size) {
    return (offset + i) % size;
}

static uint16_t modbus_crc16(const uint8_t* data, uint16_t len) {
    uint16_t crc = 0xFFFF;
    for (uint16_t i = 0; i < len; i++) {
        crc ^= data[i];
        for (int b = 0; b < 8; b++) {
            if (crc & 1) {
                crc = (crc >> 1) ^ 0xA001;
            } else {
                crc >>= 1;
            }
        }
    }
    return crc;
}

static void process_modbus_frame(const uint8_t* frame, uint16_t frame_len,
                                 float start_ts, float end_ts, uint8_t pin,
                                 DECODER_EVENT_ARENA* arena) {
    if (frame_len < 4) return; /* Slave(1) + FC(1) + CRC(2) */

    uint8_t slave = frame[0];
    uint8_t fc = frame[1];
    uint16_t calc_crc = modbus_crc16(frame, frame_len - 2);
    uint16_t rx_crc = frame[frame_len - 2] | ((uint16_t)frame[frame_len - 1] << 8);
    bool crc_ok = (calc_crc == rx_crc);

    if (!crc_ok) {
        DECODER_EVENT_V2* ev = decoder_arena_add(arena, DECODE_MODBUS, EVT_MODBUS_ERR_CRC,
                                                 start_ts, end_ts, pin, 0);
        if (ev) {
            ev->modbus.slave_addr = slave;
            ev->modbus.function_code = fc;
            ev->modbus.crc = rx_crc;
            ev->modbus.flags = 0;
        }
        return;
    }

    /* Check for exception response (FC has MSB set) */
    if (fc & 0x80) {
        uint8_t exc = (frame_len >= 3) ? frame[2] : 0;
        DECODER_EVENT_V2* ev = decoder_arena_add(arena, DECODE_MODBUS, EVT_MODBUS_EXCEPTION,
                                                 start_ts, end_ts, pin, 1);
        if (ev) {
            ev->modbus.slave_addr = slave;
            ev->modbus.function_code = fc;
            ev->modbus.exception_code = exc;
            ev->modbus.crc = rx_crc;
            ev->modbus.flags = 1;
        }
        return;
    }

    /* Standard requests / responses */
    if (frame_len == 8 && (fc == 0x01 || fc == 0x02 || fc == 0x03 || fc == 0x04 ||
                           fc == 0x05 || fc == 0x06)) {
        /* Standard 8-byte request */
        uint16_t addr = ((uint16_t)frame[2] << 8) | frame[3];
        uint16_t count_or_val = ((uint16_t)frame[4] << 8) | frame[5];

        DECODER_EVENT_V2* ev = decoder_arena_add(arena, DECODE_MODBUS, EVT_MODBUS_REQ,
                                                 start_ts, end_ts, pin, 1);
        if (ev) {
            ev->modbus.slave_addr = slave;
            ev->modbus.function_code = fc;
            ev->modbus.reg_addr = addr;
            ev->modbus.reg_count = count_or_val;
            ev->modbus.crc = rx_crc;
            ev->modbus.flags = 1;
        }
    } else {
        /* Generic / Response packet */
        uint16_t byte_count = (frame_len >= 3) ? frame[2] : 0;
        DECODER_EVENT_V2* ev = decoder_arena_add(arena, DECODE_MODBUS, EVT_MODBUS_RESP,
                                                 start_ts, end_ts, pin, 1);
        if (ev) {
            ev->modbus.slave_addr = slave;
            ev->modbus.function_code = fc;
            ev->modbus.reg_count = byte_count;
            ev->modbus.crc = rx_crc;
            ev->modbus.flags = 1;
            if (frame_len >= 5) {
                ev->modbus.data[0] = frame[3];
                ev->modbus.data[1] = frame[4];
            }
        }
    }
}

void modbus_decode(const uint8_t* buffer, uint32_t length, uint32_t sample_rate,
                   uint8_t rx_pin, uint32_t baud_rate, uint8_t pin_base,
                   uint32_t buf_offset, DECODER_EVENT_ARENA* arena) {
    if (!buffer || length < 20 || !arena || sample_rate == 0) return;
    if (baud_rate == 0) baud_rate = 9600;

    uint8_t pin = rx_pin - pin_base;
    float bit_samples = (float)sample_rate / (float)baud_rate;
    /* 3.5 character times in samples (1 char = 11 bit times: 1 start, 8 data, 2 stop/parity) */
    uint32_t gap_threshold_samples = (uint32_t)(3.5f * 11.0f * bit_samples);

    uint8_t frame[256];
    uint16_t frame_len = 0;
    uint32_t frame_start_sample = 0;
    uint32_t last_byte_end_sample = 0;

    uint32_t i = 0;
    while (i < length) {
        /* Detect falling edge (Start bit: 1 -> 0) */
        uint32_t idx = circ_idx(buf_offset, i, CAPTURE_BUFFER_SIZE);
        if (get_pin(buffer, idx, pin) != 0) {
            i++;
            continue;
        }

        /* Check idle gap before this byte */
        if (frame_len > 0 && (i > last_byte_end_sample) &&
            ((i - last_byte_end_sample) >= gap_threshold_samples)) {
            /* 3.5t idle period exceeded -> flush frame! */
            float start_ts = (float)frame_start_sample / sample_rate;
            float end_ts = (float)last_byte_end_sample / sample_rate;
            process_modbus_frame(frame, frame_len, start_ts, end_ts, pin, arena);
            frame_len = 0;
        }

        /* Verify start bit at center (0.5 * bit_samples) */
        uint32_t sample_center = i + (uint32_t)(0.5f * bit_samples);
        if (sample_center >= length) break;
        if (get_pin(buffer, circ_idx(buf_offset, sample_center, CAPTURE_BUFFER_SIZE), pin) != 0) {
            i++;
            continue; /* False start bit */
        }

        if (frame_len == 0) {
            frame_start_sample = i;
        }

        /* Read 8 data bits (LSB first) */
        uint8_t byte_val = 0;
        for (int b = 0; b < 8; b++) {
            uint32_t s_idx = i + (uint32_t)((1.5f + b) * bit_samples);
            if (s_idx >= length) goto stream_end;
            uint8_t bit = get_pin(buffer, circ_idx(buf_offset, s_idx, CAPTURE_BUFFER_SIZE), pin);
            byte_val |= (bit << b);
        }

        if (frame_len < sizeof(frame)) {
            frame[frame_len++] = byte_val;
        }

        /* Stop bit occurs at bit index 9.5 */
        last_byte_end_sample = i + (uint32_t)(10.0f * bit_samples);
        i = last_byte_end_sample;
    }

stream_end:
    if (frame_len > 0) {
        float start_ts = (float)frame_start_sample / sample_rate;
        float end_ts = (float)last_byte_end_sample / sample_rate;
        process_modbus_frame(frame, frame_len, start_ts, end_ts, pin, arena);
    }
}

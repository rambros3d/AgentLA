#include "decode.h"
#include "board_settings.h"
#include <stdio.h>
#include <string.h>

static void add_event(DECODE_RESULT* r, EVENT_TYPE type, float ts, const char* text) {
    if(r->count >= DECODE_MAX_EVENTS) return;
    DECODE_EVENT* e = &r->events[r->count++];
    e->type = type;
    e->timestamp = ts;
    e->pin = 0;
    e->value = 0;
    e->ack = 0;
    e->rw = 0;
    strncpy(e->text, text, DECODE_TEXT_LEN - 1);
    e->text[DECODE_TEXT_LEN - 1] = '\0';
}

static inline bool get_pin(const uint8_t* buf, uint32_t idx, uint8_t pin) {
    return (buf[idx] >> pin) & 1;
}

static inline uint32_t circ_idx(uint32_t offset, uint32_t i, uint32_t size) {
    return (offset + i) % size;
}

/* ─── I2C DECODER ──────────────────────────────────────────────────────────── */

void i2c_decode(const uint8_t* buffer, uint32_t length, uint32_t sample_rate,
                uint8_t sda_pin, uint8_t scl_pin, uint8_t pin_base,
                uint32_t buf_offset, DECODE_RESULT* result) {
    result->count = 0;
    if(length < 2) return;

    uint8_t sda = sda_pin - pin_base;
    uint8_t scl = scl_pin - pin_base;

    bool prev_sda = get_pin(buffer, circ_idx(buf_offset, 0, CAPTURE_BUFFER_SIZE), sda);
    bool prev_scl = get_pin(buffer, circ_idx(buf_offset, 0, CAPTURE_BUFFER_SIZE), scl);

    uint8_t bit_count = 0;
    uint8_t byte_val = 0;
    bool in_transaction = false;
    bool addr_phase = true;
    uint8_t addr = 0;
    bool rw = false;

    for(uint32_t i = 1; i < length; i++) {
        uint32_t idx = circ_idx(buf_offset, i, CAPTURE_BUFFER_SIZE);
        bool cur_sda = get_pin(buffer, idx, sda);
        bool cur_scl = get_pin(buffer, idx, scl);

        float ts = (float)i / sample_rate;

        /* START: SDA falls while SCL is high */
        if(prev_scl && cur_scl && prev_sda && !cur_sda) {
            add_event(result, EVT_I2C_START, ts, "I2C START");
            in_transaction = true;
            bit_count = 0;
            byte_val = 0;
            addr_phase = true;
            continue;
        }

        /* STOP: SDA rises while SCL is high */
        if(prev_scl && cur_scl && !prev_sda && cur_sda && in_transaction) {
            add_event(result, EVT_I2C_STOP, ts, "I2C STOP");
            in_transaction = false;
            continue;
        }

        /* Data bit: SDA stable while SCL rises */
        if(!prev_scl && cur_scl && in_transaction) {
            if(bit_count < 8) {
                byte_val = (byte_val << 1) | cur_sda;
                bit_count++;
            } else {
                /* 9th clock = ACK/NACK */
                if(cur_sda) {
                    add_event(result, EVT_I2C_NACK, ts, "I2C NACK");
                } else {
                    add_event(result, EVT_I2C_ACK, ts, "I2C ACK");
                }

                if(addr_phase) {
                    rw = byte_val & 1;
                    addr = byte_val >> 1;
                    char txt[32];
                    sprintf(txt, "I2C ADDR 0x%02X %c", addr, rw ? 'R' : 'W');
                    add_event(result, EVT_I2C_ADDR, ts, txt);
                    addr_phase = false;
                } else {
                    char txt[32];
                    sprintf(txt, "I2C DATA 0x%02X", byte_val);
                    add_event(result, EVT_I2C_DATA, ts, txt);
                }

                bit_count = 0;
                byte_val = 0;
            }
        }

        /* Clock stretching: SCL held low by slave */
        /* We just wait — the loop continues sampling until SCL rises */

        prev_sda = cur_sda;
        prev_scl = cur_scl;
    }
}

/* ─── SPI DECODER ──────────────────────────────────────────────────────────── */

void spi_decode(const uint8_t* buffer, uint32_t length, uint32_t sample_rate,
                uint8_t cs_pin, uint8_t sclk_pin, uint8_t mosi_pin, uint8_t miso_pin,
                uint8_t pin_base, uint32_t buf_offset, DECODE_RESULT* result) {
    result->count = 0;
    if(length < 2) return;

    uint8_t cs = cs_pin - pin_base;
    uint8_t sclk = sclk_pin - pin_base;
    uint8_t mosi = mosi_pin - pin_base;
    uint8_t miso = miso_pin - pin_base;

    bool prev_cs = get_pin(buffer, circ_idx(buf_offset, 0, CAPTURE_BUFFER_SIZE), cs);
    bool prev_sclk = get_pin(buffer, circ_idx(buf_offset, 0, CAPTURE_BUFFER_SIZE), sclk);
    bool cs_active = !prev_cs;

    uint8_t bit_count = 0;
    uint8_t mosi_byte = 0;
    uint8_t miso_byte = 0;
    uint32_t byte_index = 0;

    for(uint32_t i = 1; i < length; i++) {
        uint32_t idx = circ_idx(buf_offset, i, CAPTURE_BUFFER_SIZE);
        bool cur_cs = get_pin(buffer, idx, cs);
        bool cur_sclk = get_pin(buffer, idx, sclk);
        float ts = (float)i / sample_rate;

        /* CS assertion (falling edge) */
        if(prev_cs && !cur_cs) {
            add_event(result, EVT_SPI_CS_ASSERT, ts, "SPI CS ASSERT");
            cs_active = true;
            bit_count = 0;
            mosi_byte = 0;
            miso_byte = 0;
            byte_index = 0;
        }

        /* CS deassertion (rising edge) */
        if(!prev_cs && cur_cs && cs_active) {
            add_event(result, EVT_SPI_CS_DEASSERT, ts, "SPI CS DEASSERT");
            cs_active = false;
        }

        /* Clock edge: sample on rising (CPOL=0) or falling (CPOL=1) */
        if(cs_active && !prev_sclk && cur_sclk) {
            mosi_byte = (mosi_byte << 1) | get_pin(buffer, idx, mosi);
            miso_byte = (miso_byte << 1) | get_pin(buffer, idx, miso);
            bit_count++;

            if(bit_count == 8) {
                char txt[32];
                sprintf(txt, "SPI MOSI 0x%02X", mosi_byte);
                add_event(result, EVT_SPI_MOSI, ts, txt);
                sprintf(txt, "SPI MISO 0x%02X", miso_byte);
                add_event(result, EVT_SPI_MISO, ts, txt);
                bit_count = 0;
                mosi_byte = 0;
                miso_byte = 0;
                byte_index++;
            }
        }

        prev_cs = cur_cs;
        prev_sclk = cur_sclk;
    }
}

/* ─── UART DECODER ─────────────────────────────────────────────────────────── */

void uart_decode(const uint8_t* buffer, uint32_t length, uint32_t sample_rate,
                 uint8_t rx_pin, uint32_t baud_rate, uint8_t pin_base,
                 uint32_t buf_offset, DECODE_RESULT* result) {
    result->count = 0;
    if(length < 2 || baud_rate == 0) return;

    uint8_t rx = rx_pin - pin_base;
    uint32_t samples_per_bit = sample_rate / baud_rate;
    if(samples_per_bit == 0) samples_per_bit = 1;

    bool prev_rx = get_pin(buffer, circ_idx(buf_offset, 0, CAPTURE_BUFFER_SIZE), rx);
    bool in_frame = false;
    uint8_t bit_count = 0;
    uint8_t byte_val = 0;
    uint32_t sample_pos = 0;
    uint32_t break_start = 0;

    for(uint32_t i = 1; i < length; i++) {
        uint32_t idx = circ_idx(buf_offset, i, CAPTURE_BUFFER_SIZE);
        bool cur_rx = get_pin(buffer, idx, rx);
        float ts = (float)i / sample_rate;

        /* Break condition: RX held low for > 1 frame time */
        if(!cur_rx && !in_frame) {
            if(break_start == 0) break_start = i;
            if((i - break_start) > samples_per_bit * 12) {
                add_event(result, EVT_UART_BREAK, ts, "UART BREAK");
                break_start = 0;
            }
        } else if(cur_rx) {
            break_start = 0;
        }

        /* Start bit: falling edge on RX */
        if(!in_frame && prev_rx && !cur_rx) {
            in_frame = true;
            bit_count = 0;
            byte_val = 0;
            sample_pos = i + samples_per_bit * 3 / 2;
            continue;
        }

        /* Sample data bits */
        if(in_frame && i >= sample_pos) {
            if(bit_count < 8) {
                byte_val |= (cur_rx << bit_count);
                bit_count++;
                sample_pos += samples_per_bit;
            } else if(bit_count == 8) {
                /* Stop bit */
                if(!cur_rx) {
                    add_event(result, EVT_UART_ERR_FRAMING, ts, "UART ERR_FRAMING");
                } else {
                    char txt[32];
                    sprintf(txt, "UART 0x%02X", byte_val);
                    add_event(result, EVT_UART_DATA, ts, txt);
                }
                in_frame = false;
            }
        }

        prev_rx = cur_rx;
    }
}

/* ─── PWM DECODER ──────────────────────────────────────────────────────────── */

void pwm_decode(const uint8_t* buffer, uint32_t length, uint32_t sample_rate,
                uint8_t pin, uint8_t pin_base, uint32_t buf_offset, DECODE_RESULT* result) {
    result->count = 0;
    if(length < 4) return;

    uint8_t p = pin - pin_base;
    uint32_t rising_edge = 0;
    uint32_t falling_edge = 0;
    uint32_t period = 0;
    bool found_first_rising = false;
    bool prev = get_pin(buffer, circ_idx(buf_offset, 0, CAPTURE_BUFFER_SIZE), p);

    for(uint32_t i = 1; i < length; i++) {
        uint32_t idx = circ_idx(buf_offset, i, CAPTURE_BUFFER_SIZE);
        bool cur = get_pin(buffer, idx, p);
        float ts = (float)i / sample_rate;

        /* Rising edge */
        if(!prev && cur) {
            if(found_first_rising && period > 0) {
                uint32_t high_time = falling_edge - rising_edge;
                uint32_t total_time = i - rising_edge;
                float freq = (float)sample_rate / total_time;
                float duty = (float)high_time * 100.0 / total_time;
                char txt[64];
                sprintf(txt, "PWM FREQ %.1f Hz PERIOD %.6fs DUTY %.1f%%",
                        freq, (float)total_time / sample_rate, duty);
                add_event(result, EVT_PWM_SIGNAL, ts, txt);
            }
            rising_edge = i;
            found_first_rising = true;
        }

        /* Falling edge */
        if(prev && !cur) {
            falling_edge = i;
            if(found_first_rising) {
                period = i - rising_edge;
            }
        }

        prev = cur;
    }

    if(!found_first_rising) {
        add_event(result, EVT_PWM_NO_SIGNAL, 0, "PWM NO_SIGNAL");
    }
}

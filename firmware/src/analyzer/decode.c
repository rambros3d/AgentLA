#include "decode.h"
#include "board_settings.h"
#include <stdio.h>
#include <string.h>

void decoder_arena_init(DECODER_EVENT_ARENA* arena) {
    if (!arena) return;
    arena->count = 0;
    arena->overflow_count = 0;
}

DECODER_EVENT_V2* decoder_arena_add(DECODER_EVENT_ARENA* arena, uint8_t protocol, uint8_t type,
                                    float ts, float end_ts, uint8_t channel, uint8_t flags) {
    if (!arena) return NULL;
    if (arena->count >= DECODER_ARENA_CAPACITY) {
        arena->overflow_count++;
        return NULL;
    }
    DECODER_EVENT_V2* ev = &arena->events[arena->count++];
    memset(ev, 0, sizeof(*ev));
    ev->timestamp = ts;
    ev->end_timestamp = end_ts;
    ev->protocol = protocol;
    ev->type = type;
    ev->channel = channel;
    ev->flags = flags;
    return ev;
}

void decoder_event_format(const DECODER_EVENT_V2* ev, char* out, size_t out_len) {
    if (!ev || !out || out_len == 0) return;
    out[0] = '\0';

    switch (ev->protocol) {
        case DECODE_CAN:
            if (ev->type == EVT_CAN_FRAME) {
                bool is_ext = (ev->can.flags & 1) != 0;
                bool is_fd = (ev->can.flags & 2) != 0;
                bool is_brs = (ev->can.flags & 4) != 0;
                bool crc_ok = (ev->can.flags & 8) != 0;
                snprintf(out, out_len, "CAN %sID=0x%lX%s%s DLC=%u DATA=%02X %02X %02X %02X CRC=0x%04X %s",
                         is_ext ? "EXT " : "",
                         (unsigned long)ev->can.id,
                         is_fd ? " FD" : "",
                         is_brs ? " BRS" : "",
                         ev->can.dlc,
                         ev->can.data[0], ev->can.data[1], ev->can.data[2], ev->can.data[3],
                         ev->can.crc,
                         crc_ok ? "OK" : "CRC_ERR");
            } else if (ev->type == EVT_CAN_ERR_STUFF) {
                snprintf(out, out_len, "CAN Stuff Error at ts=%.6f", ev->timestamp);
            } else if (ev->type == EVT_CAN_ERR_CRC) {
                snprintf(out, out_len, "CAN CRC Error at ts=%.6f", ev->timestamp);
            }
            break;

        case DECODE_MODBUS:
            if (ev->type == EVT_MODBUS_REQ) {
                snprintf(out, out_len, "MODBUS REQ slave=%u fc=0x%02X addr=%u count=%u CRC=0x%04X %s",
                         ev->modbus.slave_addr, ev->modbus.function_code,
                         ev->modbus.reg_addr, ev->modbus.reg_count,
                         ev->modbus.crc, (ev->modbus.flags & 1) ? "OK" : "CRC_ERR");
            } else if (ev->type == EVT_MODBUS_RESP) {
                snprintf(out, out_len, "MODBUS RESP slave=%u fc=0x%02X bytes=%u CRC=0x%04X %s",
                         ev->modbus.slave_addr, ev->modbus.function_code,
                         ev->modbus.reg_count,
                         ev->modbus.crc, (ev->modbus.flags & 1) ? "OK" : "CRC_ERR");
            } else if (ev->type == EVT_MODBUS_EXCEPTION) {
                const char* desc = "Unknown";
                if (ev->modbus.exception_code == 0x01) desc = "Illegal Function";
                else if (ev->modbus.exception_code == 0x02) desc = "Illegal Data Address";
                else if (ev->modbus.exception_code == 0x03) desc = "Illegal Data Value";
                else if (ev->modbus.exception_code == 0x04) desc = "Slave Device Failure";
                snprintf(out, out_len, "MODBUS EXCEPTION slave=%u fc=0x%02X code=0x%02X (%s)",
                         ev->modbus.slave_addr, ev->modbus.function_code,
                         ev->modbus.exception_code, desc);
            } else if (ev->type == EVT_MODBUS_ERR_CRC) {
                snprintf(out, out_len, "MODBUS CRC Error slave=%u CRC=0x%04X",
                         ev->modbus.slave_addr, ev->modbus.crc);
            }
            break;

        case DECODE_DMX:
            if (ev->type == EVT_DMX_FRAME) {
                snprintf(out, out_len, "DMX FRAME slots=%u start_code=0x%02X [0]=%u [1]=%u [2]=%u",
                         ev->dmx.channel_count, ev->dmx.start_code,
                         ev->dmx.slots[0], ev->dmx.slots[1], ev->dmx.slots[2]);
            } else if (ev->type == EVT_DMX_BREAK) {
                snprintf(out, out_len, "DMX Break dur=%.2fus", (ev->end_timestamp - ev->timestamp) * 1e6f);
            }
            break;

        case DECODE_LED_1W:
        case DECODE_LED_2W:
            if (ev->type == EVT_LED_PIXEL) {
                if (ev->led.w > 0 || (ev->flags & 2)) {
                    snprintf(out, out_len, "LED #%u #%02X%02X%02X W=%u",
                             ev->led.pixel_index, ev->led.r, ev->led.g, ev->led.b, ev->led.w);
                } else if (ev->protocol == DECODE_LED_2W && ev->led.brightness > 0) {
                    snprintf(out, out_len, "LED #%u #%02X%02X%02X bright=%u/31",
                             ev->led.pixel_index, ev->led.r, ev->led.g, ev->led.b, ev->led.brightness);
                } else {
                    snprintf(out, out_len, "LED #%u #%02X%02X%02X",
                             ev->led.pixel_index, ev->led.r, ev->led.g, ev->led.b);
                }
            } else if (ev->type == EVT_LED_TIMING_VIOLATION) {
                snprintf(out, out_len, "LED_TIMING_VIOLATION pulse=%.2fus", (float)ev->led.pulse_ns / 1000.0f);
            } else if (ev->type == EVT_LED_RESET) {
                snprintf(out, out_len, "LED Reset Gap dur=%.2fus", (ev->end_timestamp - ev->timestamp) * 1e6f);
            }
            break;

        case DECODE_1WIRE:
            if (ev->type == EVT_1WIRE_ROM) {
                snprintf(out, out_len, "1WIRE ROM family=0x%02X serial=0x%02X%02X%02X%02X%02X%02X CRC=0x%02X %s",
                         ev->onewire.family,
                         ev->onewire.rom[0], ev->onewire.rom[1], ev->onewire.rom[2],
                         ev->onewire.rom[3], ev->onewire.rom[4], ev->onewire.rom[4],
                         ev->onewire.crc, (ev->onewire.flags & 1) ? "OK" : "CRC_ERR");
            } else if (ev->type == EVT_1WIRE_TEMP) {
                snprintf(out, out_len, "1WIRE TEMP %.2f C", ev->onewire.temp_c);
            } else if (ev->type == EVT_1WIRE_RESET) {
                snprintf(out, out_len, "1WIRE Reset dur=%.2fus", (ev->end_timestamp - ev->timestamp) * 1e6f);
            } else if (ev->type == EVT_1WIRE_PRESENCE) {
                snprintf(out, out_len, "1WIRE Presence dur=%.2fus", (ev->end_timestamp - ev->timestamp) * 1e6f);
            }
            break;

        case DECODE_LIN:
            if (ev->type == EVT_LIN_FRAME) {
                snprintf(out, out_len, "LIN PID=0x%02X (ID=0x%02X) len=%u CHK=0x%02X %s %s",
                         ev->lin.pid, ev->lin.id, ev->lin.length, ev->lin.checksum,
                         (ev->lin.flags & 1) ? "PID_OK" : "PID_ERR",
                         (ev->lin.flags & 2) ? "CHK_OK" : "CHK_ERR");
            }
            break;

        case DECODE_I3C:
            if (ev->type == EVT_I3C_BROADCAST) {
                snprintf(out, out_len, "I3C Broadcast Header 0x7E+W");
            } else if (ev->type == EVT_I3C_CCC) {
                const char* ccc_name = "CCC";
                if (ev->i3c.ccc_code == 0x07) ccc_name = "ENTDAA";
                else if (ev->i3c.ccc_code == 0x86) ccc_name = "SETDASA";
                else if (ev->i3c.ccc_code == 0x00) ccc_name = "ENEC";
                else if (ev->i3c.ccc_code == 0x01) ccc_name = "DISEC";
                else if (ev->i3c.ccc_code == 0x8D) ccc_name = "GETPID";
                snprintf(out, out_len, "I3C CCC 0x%02X (%s) dyn_addr=0x%02X %s",
                         ev->i3c.ccc_code, ccc_name, ev->i3c.dyn_addr,
                         (ev->i3c.flags & 1) ? "PARITY_OK" : "PARITY_ERR");
            } else if (ev->type == EVT_I3C_ERR_PARITY) {
                snprintf(out, out_len, "I3C Parity Error");
            }
            break;

        case DECODE_SWD:
            if (ev->type == EVT_SWD_DP_READ || ev->type == EVT_SWD_DP_WRITE ||
                ev->type == EVT_SWD_AP_READ || ev->type == EVT_SWD_AP_WRITE) {
                const char* port = ev->swd.ap_ndp ? "AP" : "DP";
                const char* rw = ev->swd.r_nw ? "READ" : "WRITE";
                const char* reg = "UNKNOWN";
                if (!ev->swd.ap_ndp) {
                    if (ev->swd.reg_addr == 0x00) reg = ev->swd.r_nw ? "DPIDR" : "ABORT";
                    else if (ev->swd.reg_addr == 0x04) reg = "CTRL/STAT";
                    else if (ev->swd.reg_addr == 0x08) reg = ev->swd.r_nw ? "RESEND" : "SELECT";
                    else if (ev->swd.reg_addr == 0x0C) reg = "RDBUFF";
                } else {
                    if (ev->swd.reg_addr == 0x00) reg = "CSW";
                    else if (ev->swd.reg_addr == 0x04) reg = "TAR";
                    else if (ev->swd.reg_addr == 0x0C) reg = "DRW";
                    else reg = "BDx";
                }
                const char* ack_str = "OK";
                if (ev->swd.ack == 2) ack_str = "WAIT";
                else if (ev->swd.ack == 4) ack_str = "FAULT";
                snprintf(out, out_len, "SWD %s %s %s=0x%08lX ACK=%s PARITY=%s",
                         port, rw, reg, (unsigned long)ev->swd.data, ack_str,
                         (ev->swd.flags & 1) ? "OK" : "ERR");
            }
            break;

        case DECODE_SWO:
            if (ev->type == EVT_SWO_ITM_PRINTF) {
                snprintf(out, out_len, "SWO ITM port=%u \"%.*s\"",
                         ev->swo.port, ev->swo.len, ev->swo.text);
            }
            break;

        case DECODE_SPIFLASH:
            if (ev->type == EVT_SPIFLASH_CMD) {
                const char* cmd_name = "UNKNOWN";
                if (ev->spiflash.opcode == 0x9F) cmd_name = "JEDEC ID";
                else if (ev->spiflash.opcode == 0x03) cmd_name = "READ";
                else if (ev->spiflash.opcode == 0x0B) cmd_name = "FAST READ";
                else if (ev->spiflash.opcode == 0x02) cmd_name = "PAGE PROGRAM";
                else if (ev->spiflash.opcode == 0x20) cmd_name = "SECTOR ERASE";
                else if (ev->spiflash.opcode == 0x06) cmd_name = "WREN";
                else if (ev->spiflash.opcode == 0x05) cmd_name = "RDSR";
                snprintf(out, out_len, "SPIFLASH CMD=0x%02X (%s) addr=0x%06lX len=%u",
                         ev->spiflash.opcode, cmd_name,
                         (unsigned long)ev->spiflash.address, ev->spiflash.length);
            }
            break;

        default:
            snprintf(out, out_len, "EVT proto=%u type=%u ts=%.6f", ev->protocol, ev->type, ev->timestamp);
            break;
    }
}

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

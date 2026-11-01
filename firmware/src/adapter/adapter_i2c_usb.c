/**
 * @file adapter_i2c_usb.c
 * @brief Native i2c-tiny-usb driverless endpoint handler for AgentLA
 */

#include "adapter_i2c_usb.h"
#include "adapter.h"
#include "board_settings.h"
#include <string.h>

static uint8_t last_i2c_status = STATUS_IDLE;
static uint8_t i2c_ctrl_rx_buffer[64];
static uint8_t i2c_ctrl_tx_buffer[64];

void adapter_i2c_usb_init(void) {
    last_i2c_status = STATUS_IDLE;
    adapter_i2c_usb_standby();
}

void adapter_i2c_usb_standby(void) {
    adapter_i2c_standby();
}

bool adapter_i2c_usb_auto_claim(void) {
    return adapter_i2c_auto_claim();
}

bool adapter_i2c_usb_control_xfer(uint8_t rhport, uint8_t stage, tusb_control_request_t const * request) {
    if (request->bmRequestType_bit.type != TUSB_REQ_TYPE_VENDOR) {
        return false;
    }

    uint8_t req = request->bRequest;
    uint8_t cmd_base = req & ~0x03;

    /* Check if this is an i2c-tiny-usb request */
    if (cmd_base != CMD_I2C_IO && req != CMD_ECHO && req != CMD_GET_FUNC &&
        req != CMD_SET_DELAY && req != CMD_GET_STATUS) {
        return false;
    }

    switch (req) {
        case CMD_ECHO: {
            if (stage == CONTROL_STAGE_SETUP) {
                if (request->bmRequestType_bit.direction == TUSB_DIR_IN) {
                    static uint16_t echo_val;
                    echo_val = request->wValue;
                    uint16_t len = (request->wLength < 2) ? request->wLength : 2;
                    return tud_control_xfer(rhport, (tusb_control_request_t const *)request, &echo_val, len);
                }
                return tud_control_status(rhport, request);
            }
            return true;
        }

        case CMD_GET_FUNC: {
            if (stage == CONTROL_STAGE_SETUP) {
                static const uint32_t func = I2C_FUNC_I2C | I2C_FUNC_SMBUS_EMUL;
                return tud_control_xfer(rhport, (tusb_control_request_t const *)request, (void*)&func, sizeof(func));
            }
            return true;
        }

        case CMD_SET_DELAY: {
            if (stage == CONTROL_STAGE_SETUP) {
                /* request->wValue contains the clock delay in us */
                if (request->wValue > 0) {
                    uint32_t freq = 1000000 / (request->wValue * 2);
                    if (freq < 10000) freq = 10000;
                    if (freq > 400000) freq = 400000;
                    i2c_set_baudrate(i2c1, freq);
                }
                return tud_control_status(rhport, request);
            }
            return true;
        }

        case CMD_GET_STATUS: {
            if (stage == CONTROL_STAGE_SETUP) {
                static uint8_t status_reply;
                status_reply = last_i2c_status;
                return tud_control_xfer(rhport, (tusb_control_request_t const *)request, &status_reply, 1);
            }
            return true;
        }

        default:
            break;
    }

    /* CMD_I2C_IO processing (with CMD_I2C_IO_BEGIN / CMD_I2C_IO_END) */
    if (cmd_base == CMD_I2C_IO || (req & 0x04) != 0) {
        uint8_t flags = req & 0x03;
        bool is_end = (flags & CMD_I2C_IO_END) != 0;
        bool nostop = !is_end;
        uint8_t addr = (uint8_t)(request->wIndex & 0x7F);
        uint16_t len = request->wLength;

        if (request->bmRequestType_bit.direction == TUSB_DIR_IN) {
            /* I2C Read transfer */
            if (stage == CONTROL_STAGE_SETUP) {
                if (len == 0) {
                    int res = adapter_i2c_raw_read(addr, NULL, 0, false, 20000);
                    last_i2c_status = (res >= 0) ? STATUS_ADDRESS_ACK : STATUS_ADDRESS_NAK;
                    return tud_control_status(rhport, request);
                }

                if (len > sizeof(i2c_ctrl_rx_buffer)) len = sizeof(i2c_ctrl_rx_buffer);
                int res = adapter_i2c_raw_read(addr, i2c_ctrl_rx_buffer, len, nostop, 50000);
                last_i2c_status = (res == (int)len) ? STATUS_ADDRESS_ACK : STATUS_ADDRESS_NAK;
                if (res < 0) {
                    memset(i2c_ctrl_rx_buffer, 0xFF, len);
                }
                return tud_control_xfer(rhport, (tusb_control_request_t const *)request, i2c_ctrl_rx_buffer, len);
            }
            return true;
        } else {
            /* I2C Write transfer */
            if (len == 0) {
                if (stage == CONTROL_STAGE_SETUP) {
                    int res = adapter_i2c_raw_write(addr, NULL, 0, false, 20000);
                    last_i2c_status = (res >= 0) ? STATUS_ADDRESS_ACK : STATUS_ADDRESS_NAK;
                    return tud_control_status(rhport, request);
                }
                return true;
            }

            if (stage == CONTROL_STAGE_SETUP) {
                uint16_t rx_len = (len > sizeof(i2c_ctrl_tx_buffer)) ? sizeof(i2c_ctrl_tx_buffer) : len;
                return tud_control_xfer(rhport, (tusb_control_request_t const *)request, i2c_ctrl_tx_buffer, rx_len);
            } else if (stage == CONTROL_STAGE_DATA) {
                uint16_t wr_len = (len > sizeof(i2c_ctrl_tx_buffer)) ? sizeof(i2c_ctrl_tx_buffer) : len;
                int res = adapter_i2c_raw_write(addr, i2c_ctrl_tx_buffer, wr_len, nostop, 50000);
                last_i2c_status = (res == (int)wr_len) ? STATUS_ADDRESS_ACK : STATUS_ADDRESS_NAK;
                return true;
            }
            return true;
        }
    }

    return false;
}

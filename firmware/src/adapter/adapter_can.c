/**
 * @file adapter_can.c
 * @brief SocketCAN gs_usb protocol handler and CAN transceiver management for AgentLA
 */

#include "adapter_can.h"
#include "board_settings.h"
#include "pin_owner.h"
#include <string.h>

static uint32_t can_mode = GS_CAN_MODE_RESET;
static bool can_termination_active = false;
static uint32_t can_claim_handle = 0;

static struct gs_host_config host_cfg;
static struct gs_device_bittiming dev_bittiming;
static struct gs_device_mode dev_mode_buf;
static uint32_t term_req_val = 0;

void adapter_can_init(void) {
    can_mode = GS_CAN_MODE_RESET;
    can_termination_active = false;
    can_claim_handle = 0;
    adapter_can_standby();
}

void adapter_can_standby(void) {
    can_mode = GS_CAN_MODE_RESET;

    if (can_claim_handle != 0) {
        pin_owner_result_t res = PIN_OWNER_OK;
        pin_owner_release_claim(can_claim_handle, &res);
        can_claim_handle = 0;
    }

    /* Tri-state CAN transceiver pins into High-Z */
    gpio_init(CAN_TX_PIN);
    gpio_set_dir(CAN_TX_PIN, GPIO_IN);
    gpio_disable_pulls(CAN_TX_PIN);

    gpio_init(CAN_RX_PIN);
    gpio_set_dir(CAN_RX_PIN, GPIO_IN);
    gpio_disable_pulls(CAN_RX_PIN);

    gpio_init(CAN_TERM_PIN);
    gpio_set_dir(CAN_TERM_PIN, GPIO_IN);
    gpio_disable_pulls(CAN_TERM_PIN);
}

void adapter_can_start(void) {
    can_mode = GS_CAN_MODE_START;

    const uint8_t pins[2] = { CAN_TX_PIN, CAN_RX_PIN };
    if (can_claim_handle == 0) {
        pin_owner_claim_pins(pins, 2, GPIO_FUNC_SIO, 0, &can_claim_handle);
    }

    /* Drive CAN_TX high (recessive state) */
    gpio_init(CAN_TX_PIN);
    gpio_set_dir(CAN_TX_PIN, GPIO_OUT);
    gpio_put(CAN_TX_PIN, 1);

    /* CAN_RX input with weak pull-up */
    gpio_init(CAN_RX_PIN);
    gpio_set_dir(CAN_RX_PIN, GPIO_IN);
    gpio_pull_up(CAN_RX_PIN);

    if (can_termination_active) {
        gpio_init(CAN_TERM_PIN);
        gpio_set_dir(CAN_TERM_PIN, GPIO_OUT);
        gpio_put(CAN_TERM_PIN, 1);
    }
}

bool adapter_can_is_active(void) {
    return can_mode == GS_CAN_MODE_START;
}

void adapter_can_set_termination(bool enable) {
    can_termination_active = enable;
    if (enable) {
        gpio_init(CAN_TERM_PIN);
        gpio_set_dir(CAN_TERM_PIN, GPIO_OUT);
        gpio_put(CAN_TERM_PIN, 1);
    } else {
        gpio_init(CAN_TERM_PIN);
        gpio_set_dir(CAN_TERM_PIN, GPIO_IN);
        gpio_disable_pulls(CAN_TERM_PIN);
    }
}

bool adapter_can_get_termination(void) {
    return can_termination_active;
}

void adapter_can_task(void) {
    if (!adapter_can_is_active()) return;

    /* Loopback / packet echo loop for SocketCAN gs_usb */
    while (tud_vendor_available() >= sizeof(struct gs_host_frame)) {
        struct gs_host_frame rx_frame;
        uint32_t rd = tud_vendor_read(&rx_frame, sizeof(rx_frame));
        if (rd == sizeof(rx_frame)) {
            /* If host requested transmission echo, reply on Bulk IN endpoint */
            if (rx_frame.echo_id != 0xFFFFFFFF) {
                if (tud_vendor_write_available() >= sizeof(rx_frame)) {
                    tud_vendor_write(&rx_frame, sizeof(rx_frame));
                    tud_vendor_write_flush();
                }
            }
        }
    }
}

bool adapter_can_control_xfer(uint8_t rhport, uint8_t stage, tusb_control_request_t const * request) {
    /* Only handle requests targeted at the CAN interface or with valid gs_usb requests */
    if (request->bmRequestType_bit.type != TUSB_REQ_TYPE_VENDOR) {
        return false;
    }

    uint8_t req = request->bRequest;

    /* Interface check: gs_usb sets wIndex to interface number or channel index */
    bool is_can_itf = (request->wIndex == ITF_NUM_CAN || request->wIndex == 0);
    if (!is_can_itf && req != GS_USB_BREQ_HOST_FORMAT && req != GS_USB_BREQ_DEVICE_CONFIG) {
        return false;
    }

    switch (req) {
        case GS_USB_BREQ_HOST_FORMAT: {
            if (stage == CONTROL_STAGE_SETUP) {
                if (request->wLength == sizeof(struct gs_host_config)) {
                    return tud_control_xfer(rhport, (tusb_control_request_t const *)request, &host_cfg, sizeof(host_cfg));
                }
                return tud_control_status(rhport, request);
            }
            return true;
        }

        case GS_USB_BREQ_DEVICE_CONFIG: {
            if (stage == CONTROL_STAGE_SETUP) {
                static const struct gs_device_config dconf = {
                    .reserved1 = 0,
                    .reserved2 = 0,
                    .reserved3 = 0,
                    .icount = 0,      /* 1 channel: icount = count - 1 = 0 */
                    .sw_version = 1,
                    .hw_version = 1,
                };
                return tud_control_xfer(rhport, (tusb_control_request_t const *)request, (void*)&dconf, sizeof(dconf));
            }
            return true;
        }

        case GS_USB_BREQ_BT_CONST: {
            if (stage == CONTROL_STAGE_SETUP) {
                static const struct gs_device_bt_const bt_const = {
                    .feature = 0,
                    .fclk_can = 48000000,
                    .tseg1_min = 1,
                    .tseg1_max = 16,
                    .tseg2_min = 1,
                    .tseg2_max = 8,
                    .sjw_max = 4,
                    .brp_min = 1,
                    .brp_max = 1024,
                    .brp_inc = 1,
                };
                return tud_control_xfer(rhport, (tusb_control_request_t const *)request, (void*)&bt_const, sizeof(bt_const));
            }
            return true;
        }

        case GS_USB_BREQ_BITTIMING: {
            if (stage == CONTROL_STAGE_SETUP) {
                if (request->wLength == sizeof(struct gs_device_bittiming)) {
                    return tud_control_xfer(rhport, (tusb_control_request_t const *)request, &dev_bittiming, sizeof(dev_bittiming));
                }
                return tud_control_status(rhport, request);
            }
            return true;
        }

        case GS_USB_BREQ_MODE: {
            if (stage == CONTROL_STAGE_SETUP) {
                if (request->wLength == sizeof(struct gs_device_mode)) {
                    return tud_control_xfer(rhport, (tusb_control_request_t const *)request, &dev_mode_buf, sizeof(dev_mode_buf));
                }
                return tud_control_status(rhport, request);
            } else if (stage == CONTROL_STAGE_DATA) {
                if (dev_mode_buf.mode == GS_CAN_MODE_START) {
                    adapter_can_start();
                } else if (dev_mode_buf.mode == GS_CAN_MODE_RESET) {
                    adapter_can_standby();
                }
                return true;
            }
            return true;
        }

        case GS_USB_BREQ_SET_TERMINATION: {
            if (stage == CONTROL_STAGE_SETUP) {
                if (request->wLength == sizeof(uint32_t)) {
                    return tud_control_xfer(rhport, (tusb_control_request_t const *)request, &term_req_val, sizeof(term_req_val));
                } else {
                    adapter_can_set_termination(request->wValue != 0);
                    return tud_control_status(rhport, request);
                }
            } else if (stage == CONTROL_STAGE_DATA) {
                adapter_can_set_termination(term_req_val != 0);
                return true;
            }
            return true;
        }

        case GS_USB_BREQ_GET_TERMINATION: {
            if (stage == CONTROL_STAGE_SETUP) {
                static uint32_t term_status;
                term_status = adapter_can_get_termination() ? 1 : 0;
                return tud_control_xfer(rhport, (tusb_control_request_t const *)request, &term_status, sizeof(term_status));
            }
            return true;
        }

        case GS_USB_BREQ_BERR:
        case GS_USB_BREQ_TIMESTAMP:
        case GS_USB_BREQ_IDENTIFY:
            if (stage == CONTROL_STAGE_SETUP) {
                return tud_control_status(rhport, request);
            }
            return true;

        default:
            break;
    }

    return false;
}

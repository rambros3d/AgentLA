/**
 * @file adapter_can.h
 * @brief SocketCAN gs_usb protocol handler and CAN transceiver management for AgentLA
 */

#ifndef __AGENTLA_ADAPTER_CAN__
#define __AGENTLA_ADAPTER_CAN__

#include "pico/stdlib.h"
#include "tusb.h"

#define GS_CAN_MODE_RESET 0
#define GS_CAN_MODE_START 1

enum gs_usb_breq {
    GS_USB_BREQ_HOST_FORMAT = 0,
    GS_USB_BREQ_BITTIMING = 1,
    GS_USB_BREQ_MODE = 2,
    GS_USB_BREQ_BERR = 3,
    GS_USB_BREQ_BT_CONST = 4,
    GS_USB_BREQ_DEVICE_CONFIG = 5,
    GS_USB_BREQ_TIMESTAMP = 6,
    GS_USB_BREQ_IDENTIFY = 7,
    GS_USB_BREQ_GET_USER_ID = 8,
    GS_USB_BREQ_SET_USER_ID = 9,
    GS_USB_BREQ_DATA_BITTIMING = 10,
    GS_USB_BREQ_BT_CONST_EXT = 11,
    GS_USB_BREQ_SET_TERMINATION = 12,
    GS_USB_BREQ_GET_TERMINATION = 13,
    GS_USB_BREQ_GET_BERR_COUNTER = 14,
};

struct gs_host_config {
    uint32_t byte_order;
} __attribute__((packed));

struct gs_device_config {
    uint8_t reserved1;
    uint8_t reserved2;
    uint8_t reserved3;
    uint8_t icount;
    uint32_t sw_version;
    uint32_t hw_version;
} __attribute__((packed));

struct gs_device_bt_const {
    uint32_t feature;
    uint32_t fclk_can;
    uint32_t tseg1_min;
    uint32_t tseg1_max;
    uint32_t tseg2_min;
    uint32_t tseg2_max;
    uint32_t sjw_max;
    uint32_t brp_min;
    uint32_t brp_max;
    uint32_t brp_inc;
} __attribute__((packed));

struct gs_device_bittiming {
    uint32_t prop_seg;
    uint32_t phase_seg1;
    uint32_t phase_seg2;
    uint32_t sjw;
    uint32_t brp;
} __attribute__((packed));

struct gs_device_mode {
    uint32_t mode;
    uint32_t flags;
} __attribute__((packed));

struct gs_host_frame {
    uint32_t echo_id;
    uint32_t can_id;
    uint8_t can_dlc;
    uint8_t channel;
    uint8_t flags;
    uint8_t reserved;
    uint8_t data[8];
} __attribute__((packed));

void adapter_can_init(void);
void adapter_can_standby(void);
void adapter_can_start(void);
bool adapter_can_is_active(void);
void adapter_can_set_termination(bool enable);
bool adapter_can_get_termination(void);
void adapter_can_task(void);
bool adapter_can_control_xfer(uint8_t rhport, uint8_t stage, tusb_control_request_t const * request);

#endif

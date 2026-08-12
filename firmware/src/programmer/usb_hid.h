/**
 * @file usb_hid.h
 * @brief USB composite device: CDC (serial CLI) + HID (CMSIS-DAP)
 */
#ifndef AGENTLA_USB_HID_H
#define AGENTLA_USB_HID_H

#include <stdint.h>
#include <stdbool.h>

#define CMSIS_DAP_VID  0x1209
#define CMSIS_DAP_PID  0x414D

void usb_hid_init(void);
void usb_hid_task(void);
bool usb_hid_is_ready(void);

/* CDC Serial Interface */
bool usb_cdc_available(void);
int usb_cdc_read_char(void);
void usb_cdc_write_str(const char *str);
void usb_cdc_write_char(char c);

/* Debug log interface (CDC1) */
void dbg_printf(const char *fmt, ...);

/* CDC RX diagnostic counters */
extern volatile uint32_t g_cdc_rx_bytes_total;
extern volatile uint32_t g_cdc_rx_bytes_pushed;
extern volatile uint32_t g_cdc_rx_bytes_dropped;
extern volatile uint32_t g_cdc_rx_bytes_popped;

/* Enhanced CDC RX pipeline diagnostics */
extern volatile uint32_t g_cdc_rx_fifo_before_tud_task;
extern volatile uint32_t g_cdc_rx_hw_read_calls;
extern volatile uint32_t g_cdc_rx_hw_read_bytes;
extern volatile uint32_t g_cdc_rx_fifo_after_read;
extern volatile uint32_t g_cdc_rx_ring_head_snap;
extern volatile uint32_t g_cdc_rx_ring_tail_snap;

/* CDC RX pipeline reset (call before test) */
void usb_cdc_diag_reset(void);

/* CDC RX pipeline snapshot (call after tud_task, before drain) */
void usb_cdc_diag_snapshot(void);

#endif

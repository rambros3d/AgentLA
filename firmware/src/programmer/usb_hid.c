/**
 * @file usb_hid.c
 * @brief USB composite device: CDC (serial CLI) + HID (CMSIS-DAP)
 */
#include "usb_hid.h"
#include "cmsis_dap.h"
#include "adapter/adapter_can.h"
#include "adapter/adapter_i2c_usb.h"
#include "pico/stdlib.h"
#include "pico/stdio/driver.h"
#include "pico/bootrom.h"
#include "tusb.h"
#include <string.h>
#include <stdarg.h>

/* String descriptors */
static const char* string_desc_arr[] = {
    (const char[]){ 0x09, 0x04 },  /* Language ID */
    "AgentLA",                       /* 1: Manufacturer */
    "AgentLA CMSIS-DAP Probe",       /* 2: Product */
    "00000001",                      /* 3: Serial */
    "CMSIS-DAP v1 Interface",        /* 4: HID interface */
    "AgentLA CDC Serial",            /* 5: CDC0 CLI interface */
    "AgentLA Debug Log",             /* 6: CDC1 debug interface */
    "0202846A9600",                  /* 7: MAC address (ECM) */
    "AgentLA SocketCAN gs_usb",      /* 8: CAN interface */
};
static uint16_t _desc_str[32];

/* HID report descriptor for CMSIS-DAP */
static const uint8_t hid_report_desc[] = {
    TUD_HID_REPORT_DESC_GENERIC_INOUT(64)
};

/* Response buffer for CMSIS-DAP */
static uint8_t dap_response[64];
static uint16_t dap_response_len = 0;
static bool dap_response_pending = false;

/* ═══════════════════════════════════════════════════════════════════
 * USB Descriptors — Composite Device (CDC + HID + CAN)
 * ═══════════════════════════════════════════════════════════════════ */

/* Interface numbers */
#define ITF_NET          0
#define ITF_NET_DATA     1
#define ITF_NUM_CDC0     (ITF_NET_DATA + 1)      /* 2: CDC0 CLI */
#define ITF_NUM_CDC0_DATA (ITF_NUM_CDC0 + 1)     /* 3: CDC0 data */
#define ITF_NUM_CDC1     (ITF_NUM_CDC0_DATA + 1) /* 4: CDC1 debug */
#define ITF_NUM_CDC1_DATA (ITF_NUM_CDC1 + 1)     /* 5: CDC1 data */
#undef ITF_NUM_HID
#define ITF_NUM_HID      (ITF_NUM_CDC1_DATA + 1) /* 6: HID DAP */
#undef ITF_NUM_CAN
#define ITF_NUM_CAN      (ITF_NUM_HID + 1)       /* 7: CAN gs_usb */
#define ITF_NUM_TOTAL    (ITF_NUM_CAN + 1)       /* 8: Total interfaces */

/* Endpoint numbers */
#define EDPT_NET_NOTIF  0x81
#define EDPT_NET_OUT    0x02 /* net data OUT */
#define EDPT_NET_IN     0x82 /* net data IN */
#define EDPT_CDC0_NOTIF 0x83
#define EDPT_CDC0_OUT   0x03
#define EDPT_CDC0_IN    0x84
#define EDPT_CDC1_NOTIF 0x85
#define EDPT_CDC1_OUT   0x05
#define EDPT_CDC1_IN    0x86
#define EDPT_HID_OUT    0x01
#define EDPT_HID_IN     0x87
#undef EDPT_CAN_OUT
#define EDPT_CAN_OUT    0x04
#undef EDPT_CAN_IN
#define EDPT_CAN_IN     0x88

/* Device descriptor — composite with IAD */
static const tusb_desc_device_t desc_device = {
    .bLength            = sizeof(tusb_desc_device_t),
    .bDescriptorType    = TUSB_DESC_DEVICE,
    .bcdUSB             = 0x0200,
    .bDeviceClass       = TUSB_CLASS_MISC,
    .bDeviceSubClass    = MISC_SUBCLASS_COMMON,
    .bDeviceProtocol    = MISC_PROTOCOL_IAD,
    .bMaxPacketSize0    = 64,
    .idVendor           = CMSIS_DAP_VID,   /* 0x1209 */
    .idProduct          = CMSIS_DAP_PID,   /* 0x414D */
    .bcdDevice          = 0x0100,
    .iManufacturer      = 0x01,
    .iProduct           = 0x02,
    .iSerialNumber      = 0x03,
    .bNumConfigurations = 0x01
};

/* Configuration descriptor — ECM + CDC0 + CDC1 + HID + gs_usb Vendor composite */
static const uint8_t desc_config[] = {
    TUD_CONFIG_DESCRIPTOR(1, ITF_NUM_TOTAL, 0,
        TUD_CONFIG_DESC_LEN + TUD_CDC_ECM_DESC_LEN + TUD_CDC_DESC_LEN + TUD_CDC_DESC_LEN + TUD_HID_INOUT_DESC_LEN + TUD_VENDOR_DESC_LEN,
        0x00, 500),

    /* ECM Network Interface (ITF 0,1) */
    TUD_CDC_ECM_DESCRIPTOR(ITF_NET, 5, 7, EDPT_NET_NOTIF, 64, EDPT_NET_OUT, EDPT_NET_IN, 64, CFG_TUD_NET_MTU),

    /* CDC0 CLI Interface */
    TUD_CDC_DESCRIPTOR(ITF_NUM_CDC0, 5, EDPT_CDC0_NOTIF, 8, EDPT_CDC0_OUT, EDPT_CDC0_IN, 64),

    /* CDC1 Debug Interface */
    TUD_CDC_DESCRIPTOR(ITF_NUM_CDC1, 6, EDPT_CDC1_NOTIF, 8, EDPT_CDC1_OUT, EDPT_CDC1_IN, 64),

    /* HID CMSIS-DAP Interface */
    TUD_HID_INOUT_DESCRIPTOR(ITF_NUM_HID, 4, HID_ITF_PROTOCOL_NONE,
        sizeof(hid_report_desc), EDPT_HID_OUT, EDPT_HID_IN, 64, 1),

    /* SocketCAN gs_usb Vendor Interface */
    TUD_VENDOR_DESCRIPTOR(ITF_NUM_CAN, 8, EDPT_CAN_OUT, EDPT_CAN_IN, 64),
};

/* ═══════════════════════════════════════════════════════════════════
 * CDC0 CLI Output — route printf through TinyUSB CDC0
 * ═══════════════════════════════════════════════════════════════════ */

static void cdc_out_chars(const char *buf, int length) {
    uint32_t written = 0;
    uint32_t retries = 0;
    while (written < length && retries < 1000) {
        uint32_t avail = tud_cdc_write_available();
        if (avail == 0) {
            tud_cdc_write_flush();
            tud_task();
            retries++;
            busy_wait_us(10);
            continue;
        }
        uint32_t chunk = (length - written > avail) ? avail : (length - written);
        tud_cdc_write(buf + written, chunk);
        written += chunk;
        tud_cdc_write_flush();
        tud_task();
        retries = 0;
    }
}

static void cdc_out_flush(void) {
    tud_cdc_write_flush();
}

static int cdc_in_chars(char *buf, int length) {
    (void)buf; (void)length;
    return 0;
}

static stdio_driver_t stdio_cdc = {
    .out_chars    = cdc_out_chars,
    .out_flush    = cdc_out_flush,
    .in_chars     = cdc_in_chars,
    .set_chars_available_callback = NULL,
    .next         = NULL,
};

/* ═══════════════════════════════════════════════════════════════════
 * CDC1 Debug Output — diagnostic printf through TinyUSB CDC1
 * ═══════════════════════════════════════════════════════════════════ */

static void cdc1_out_chars(const char *buf, int length) {
    uint32_t written = 0;
    uint32_t retries = 0;
    while (written < length && retries < 1000) {
        uint32_t avail = tud_cdc_n_write_available(1);
        if (avail == 0) {
            tud_cdc_n_write_flush(1);
            tud_task();
            retries++;
            busy_wait_us(10);
            continue;
        }
        uint32_t chunk = (length - written > avail) ? avail : (length - written);
        tud_cdc_n_write(1, buf + written, chunk);
        written += chunk;
        tud_cdc_n_write_flush(1);
        tud_task();
        retries = 0;
    }
}

static void cdc1_out_flush(void) {
    tud_cdc_n_write_flush(1);
}

static int cdc1_in_chars(char *buf, int length) {
    (void)buf; (void)length;
    return 0;
}

static stdio_driver_t stdio_cdc1 = {
    .out_chars    = cdc1_out_chars,
    .out_flush    = cdc1_out_flush,
    .in_chars     = cdc1_in_chars,
    .set_chars_available_callback = NULL,
    .next         = NULL,
};

/* ═══════════════════════════════════════════════════════════════════
 * Public API
 * ═══════════════════════════════════════════════════════════════════ */

void usb_hid_init(void) {
    tusb_init();
    /* Register CDC0 as stdio output so printf goes through USB CDC0 (CLI) */
    stdio_set_driver_enabled(&stdio_cdc, true);
    adapter_can_init();
    adapter_i2c_usb_init();
}

/* Debug printf — writes to CDC1 (debug log) instead of CDC0 (CLI).
 * Non-blocking: drops output if CDC1 buffer is full. */
void dbg_printf(const char *fmt, ...) {
    char buf[256];
    va_list args;
    va_start(args, fmt);
    int len = vsnprintf(buf, sizeof(buf), fmt, args);
    va_end(args);
    if (len <= 0) return;
    /* Write directly to CDC1 — non-blocking, drop if no space */
    tud_cdc_n_write(1, buf, (uint32_t)len);
    tud_cdc_n_write_flush(1);
}

/* ═══════════════════════════════════════════════════════════════════
 * CDC RX Ring Buffer for CLI
 * ═══════════════════════════════════════════════════════════════════ */

#define CDC_RX_BUF_SIZE 1024
static volatile uint8_t cdc_rx_ring[CDC_RX_BUF_SIZE];
static volatile uint32_t cdc_rx_head = 0;
static volatile uint32_t cdc_rx_tail = 0;

volatile uint32_t g_cdc_rx_bytes_total = 0;
volatile uint32_t g_cdc_rx_bytes_pushed = 0;
volatile uint32_t g_cdc_rx_bytes_dropped = 0;
volatile uint32_t g_cdc_rx_bytes_popped = 0;

volatile uint32_t g_cdc_rx_fifo_before_tud_task = 0;
volatile uint32_t g_cdc_rx_hw_read_calls = 0;
volatile uint32_t g_cdc_rx_hw_read_bytes = 0;
volatile uint32_t g_cdc_rx_fifo_after_read = 0;
volatile uint32_t g_cdc_rx_ring_head_snap = 0;
volatile uint32_t g_cdc_rx_ring_tail_snap = 0;

void usb_cdc_diag_reset(void) {
    g_cdc_rx_bytes_total = 0;
    g_cdc_rx_bytes_pushed = 0;
    g_cdc_rx_bytes_dropped = 0;
    g_cdc_rx_bytes_popped = 0;
    g_cdc_rx_fifo_before_tud_task = 0;
    g_cdc_rx_hw_read_calls = 0;
    g_cdc_rx_hw_read_bytes = 0;
    g_cdc_rx_fifo_after_read = 0;
    g_cdc_rx_ring_head_snap = 0;
    g_cdc_rx_ring_tail_snap = 0;
}

void usb_cdc_diag_snapshot(void) {
    g_cdc_rx_ring_head_snap = cdc_rx_head;
    g_cdc_rx_ring_tail_snap = cdc_rx_tail;
}

bool usb_cdc_available(void) {
    return cdc_rx_head != cdc_rx_tail;
}

int usb_cdc_read_char(void) {
    if (cdc_rx_head == cdc_rx_tail) return -1;
    uint8_t c = cdc_rx_ring[cdc_rx_tail];
    cdc_rx_tail = (cdc_rx_tail + 1) % CDC_RX_BUF_SIZE;
    g_cdc_rx_bytes_popped++;
    return c;
}

void usb_cdc_write_str(const char *str) {
    if (tud_cdc_n_connected(0)) {
        tud_cdc_n_write_str(0, str);
        tud_cdc_n_write_flush(0);
    }
}

void usb_cdc_write_char(char c) {
    if (tud_cdc_n_connected(0)) {
        tud_cdc_n_write_char(0, c);
        tud_cdc_n_write_flush(0);
    }
}

/* ═══════════════════════════════════════════════════════════════════
 * USB Task
 * ═══════════════════════════════════════════════════════════════════ */

void usb_hid_task(void) {
    /* Snapshot FIFO state before tud_task processes USB events */
    g_cdc_rx_fifo_before_tud_task = tud_cdc_n_available(0);

    tud_task();

    if (dap_response_pending && tud_hid_ready()) {
        tud_hid_report(0, dap_response, dap_response_len);
        dap_response_pending = false;
    }

    /* Process CDC0 data into ring buffer for CLI processing */
    while (tud_cdc_n_available(0)) {
        uint8_t buf[256];
        uint32_t count = tud_cdc_n_read(0, buf, sizeof(buf));
        g_cdc_rx_hw_read_calls++;
        g_cdc_rx_hw_read_bytes += count;
        g_cdc_rx_bytes_total += count;
        for (uint32_t i = 0; i < count; i++) {
            uint32_t next = (cdc_rx_head + 1) % CDC_RX_BUF_SIZE;
            if (next != cdc_rx_tail) {
                cdc_rx_ring[cdc_rx_head] = buf[i];
                cdc_rx_head = next;
                g_cdc_rx_bytes_pushed++;
            } else {
                g_cdc_rx_bytes_dropped++;
            }
        }
    }

    g_cdc_rx_ring_head_snap = cdc_rx_head;
    g_cdc_rx_ring_tail_snap = cdc_rx_tail;

    /* Process SocketCAN gs_usb bulk frames */
    adapter_can_task();
}

#include "pico/bootrom.h"

void tud_cdc_line_coding_cb(uint8_t itf, cdc_line_coding_t const* p_line_coding) {
    (void)itf;
    (void)p_line_coding;
}

bool usb_hid_is_ready(void) { return tud_mounted(); }

/* ═══════════════════════════════════════════════════════════════════
 * TinyUSB Descriptor Callbacks
 * ═══════════════════════════════════════════════════════════════════ */

uint8_t const *tud_descriptor_device_cb(void) {
    return (uint8_t const *)&desc_device;
}

uint8_t const *tud_descriptor_configuration_cb(uint8_t i) {
    (void)i;
    return (uint8_t const *)desc_config;
}

uint8_t const *tud_hid_descriptor_report_cb(uint8_t i) {
    (void)i;
    return (uint8_t const *)hid_report_desc;
}

uint16_t const *tud_descriptor_string_cb(uint8_t idx, uint16_t lang) {
    (void)lang;
    uint8_t n;
    if (idx == 0) {
        memcpy(&_desc_str[1], string_desc_arr[0], 2);
        n = 1;
    } else {
        if (idx >= sizeof(string_desc_arr)/sizeof(string_desc_arr[0])) return NULL;
        const char *s = string_desc_arr[idx];
        n = (uint8_t)strlen(s);
        if (n > 31) n = 31;
        for (uint8_t i = 0; i < n; i++) _desc_str[1+i] = s[i];
    }
    _desc_str[0] = (TUSB_DESC_STRING << 8) | (2*n + 2);
    return _desc_str;
}

/* ═══════════════════════════════════════════════════════════════════
 * HID Callbacks — CMSIS-DAP
 * ═══════════════════════════════════════════════════════════════════ */

uint16_t tud_hid_get_report_cb(uint8_t itf, uint8_t rid, hid_report_type_t rt, uint8_t* buf, uint16_t len) {
    (void)itf; (void)rid; (void)rt; (void)buf; (void)len;
    return 0;
}

void tud_hid_set_report_cb(uint8_t itf, uint8_t rid, hid_report_type_t rt, uint8_t const* buf, uint16_t len) {
    (void)itf; (void)rid; (void)rt;
    if (len > 64) len = 64;
    dap_response_len = cmsis_dap_process(buf, dap_response, len);
    if (dap_response_len > 0) {
        dap_response_pending = true;
        if (tud_hid_ready()) {
            tud_hid_report(0, dap_response, dap_response_len);
            dap_response_pending = false;
        }
    }
}

void tud_hid_report_complete_cb(uint8_t itf, uint8_t const* report, uint16_t len) {
    (void)itf; (void)report; (void)len;
    if (dap_response_pending && tud_hid_ready()) {
        tud_hid_report(0, dap_response, dap_response_len);
        dap_response_pending = false;
    }
}

/* ═══════════════════════════════════════════════════════════════════
 * CDC Callbacks
 * ═══════════════════════════════════════════════════════════════════ */

void tud_cdc_rx_cb(uint8_t itf) {
    (void)itf;
    /* Data is processed in usb_hid_task() */
}

void tud_cdc_tx_complete_cb(uint8_t itf) {
    (void)itf;
}

void tud_cdc_line_state_cb(uint8_t itf, bool dtr, bool rts) {
    (void)itf;
    (void)dtr;
    (void)rts;
    /* Line state changed (DTR/RTS) — nothing to do, but the callback
       must exist so TinyUSB completes the SET_CONTROL_LINE_STATE request
       promptly instead of leaving the host CDC driver waiting. */
}

/* ═══════════════════════════════════════════════════════════════════
 * Vendor Callback — picotool BOOTSEL
 * ═══════════════════════════════════════════════════════════════════ */

bool tud_vendor_control_xfer_cb(uint8_t rhport, uint8_t stage, tusb_control_request_t const * request) {
    if (request->bmRequestType_bit.type != TUSB_REQ_TYPE_VENDOR) {
        return false;
    }

    /* 1. Picotool BOOTSEL reset */
    if (stage == CONTROL_STAGE_SETUP && request->bRequest == 0 &&
        request->wLength == 0 && request->wIndex == 0 && request->wValue == 0) {
        tud_control_status(rhport, request);
        busy_wait_ms(100);
        reset_usb_boot(0, 0);
        return true;
    }

    /* 2. SocketCAN gs_usb requests */
    if (adapter_can_control_xfer(rhport, stage, request)) {
        return true;
    }

    /* 3. i2c-tiny-usb requests */
    if (adapter_i2c_usb_control_xfer(rhport, stage, request)) {
        return true;
    }

    return false;
}

/* ═══════════════════════════════════════════════════════════════════
 * USB State Callbacks
 * ═══════════════════════════════════════════════════════════════════ */

void tud_mount_cb(void) {}
void tud_unmount_cb(void) {}
void tud_suspend_cb(bool r) { (void)r; }
void tud_resume_cb(void) {}

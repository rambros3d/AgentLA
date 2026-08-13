/*
 * AgentLA SWD Engine
 *
 * Vendored Raspberry Pi debugprobe PIO transport
 * (debugprobe/swd_probe.pio + probe_pio_transport + ported SWD_Transfer)
 * on RP2350 PIO0 SM0, pins GPIO4 (SWCLK) / GPIO5 (SWDIO).
 */

#ifndef AGENTLA_SWD_H
#define AGENTLA_SWD_H

#include "pico/stdlib.h"
#include "hardware/pio.h"

/* SWD Pin Definitions (from board_settings.h) */
#define SWD_SWCLK_PIN  4   /* PIO1 SWCLK (matches PROG_SWDCLK_PIN) */
#define SWD_SWDIO_PIN  5   /* PIO1 SWDIO (matches PROG_SWDIO_PIN) */
#define SWD_nRESET_PIN 6   /* GPIO nRESET (matches PROG_NRESET_PIN) */

/* SWD PIO configuration */
#define SWD_PIO        pio0
#define SWD_SM         0   /* State machine index */

/* SWD Clock Limits (in kHz) */
#define SWD_FREQ_MIN   100   /* 100 kHz minimum */
#define SWD_FREQ_MAX   4000  /* 4 MHz maximum */
#define SWD_FREQ_DEFAULT 100   /* 100 kHz default for reliable SWJ-DP switching */

/* SWD Protocol Constants */
#define SWD_ACK_OK     0x01
#define SWD_ACK_WAIT   0x02
#define SWD_ACK_FAULT  0x04

/* SWD Request Bit Positions */
#define SWD_REQUEST_START    0  /* Start bit (always 1) */
#define SWD_REQUEST_APnDP    1  /* APnDP: 0=DP, 1=AP */
#define SWD_REQUEST_RnW      2  /* RnW: 0=Write, 1=Read */
#define SWD_REQUEST_A23      3  /* A[2:3] register address */
#define SWD_REQUEST_PARITY   5  /* Parity bit */
#define SWD_REQUEST_STOP     6  /* Stop bit (always 0) */
#define SWD_REQUEST_PARK     7  /* Park bit (always 1) */

/* DP Register Addresses */
#define DP_DPIDR     0x0  /* Device ID (Read-only) */
#define DP_ABORT     0x0  /* Abort (Write-only) */
#define DP_CTRLSTAT  0x4  /* Control/Status (R/W) */
#define DP_SELECT    0x8  /* Bank/APSel (Write-only) */
#define DP_RDBUFF    0xC  /* Read Buffer (Read-only) */

/* AP Register Addresses (AHB-AP, ADIv5 §A3.4.1).
 * NOTE: DRW is at 0x0C — 0x08 is a reserved register. A previous value of
 * 0x8 here made every memory read/write hit the reserved slot and return 0. */
#define AP_CSW       0x0  /* Control/Status Word */
#define AP_TAR       0x4  /* Transfer Address */
#define AP_DRW       0xC  /* Data Read/Write */
#define AP_BD0       0x10 /* Banked Data 0 */
#define AP_BD1       0x14
#define AP_BD2       0x18
#define AP_BD3       0x1C

/* ADIv5 AP CSW Access Size Constants */
#define ADIV5_AP_CSW_SIZE_BYTE     0x00000000
#define ADIV5_AP_CSW_SIZE_HALFWORD 0x00000001
#define ADIV5_AP_CSW_SIZE_WORD     0x00000002
#define ADIV5_AP_CSW_DEFAULT       0x23000012  /* 32-bit word access, AddrInc single */
#define ADIV5_AP_CSW_16BIT         0x23000011  /* 16-bit halfword access, AddrInc single */

/* SWD Result Codes */
typedef enum {
    SWD_OK = 0,
    SWD_ERR_NO_TARGET,
    SWD_ERR_ACK_FAULT,
    SWD_ERR_ACK_WAIT,
    SWD_ERR_PARITY,
    SWD_ERR_PROTOCOL,
    SWD_ERR_TIMEOUT
} swd_result_t;

/* SWD State */
typedef struct {
    pio_sm_config sm_config;
    uint sm;
    pio_hw_t *pio;
    uint offset;
    uint32_t clock_freq;
    bool initialized;
    uint32_t last_idcode;
    uint8_t turnaround_cycles;  /* Configurable turnaround (default 1) */
    uint8_t idle_cycles;        /* Idle cycles after transfer */
    uint8_t ap_select;          /* Current AP bank select (0xFF = unknown/invalid, forces DP_SELECT write) */
} swd_state_t;

/* Global SWD state */
extern swd_state_t swd_state;

/* ============================================================================
 * Initialization / Deinitialization
 * ============================================================================ */

/**
 * Initialize SWD engine on PIO1.
 * Configures PIO state machine, GPIO pins, and default clock.
 * @return SWD_OK on success, error code otherwise
 */
swd_result_t swd_init(void);

/**
 * Deinitialize SWD engine.
 * Releases PIO state machine and resets GPIO pins.
 */
void swd_deinit(void);

/**
 * Check if SWD engine is initialized.
 * @return true if initialized, false otherwise
 */
bool swd_is_initialized(void);

/* ============================================================================
 * Low-Level PIO Operations
 * ============================================================================ */

/**
 * Set SWD clock frequency.
 * @param freq_khz Clock frequency in kHz (100-4000)
 * @return SWD_OK on success
 */
swd_result_t swd_set_clock(uint32_t freq_khz);

/**
 * Configure SWD turnaround cycles (1 or 2).
 * @param cycles Number of turnaround cycles (1 or 2)
 * @return SWD_OK on success, SWD_ERR_PROTOCOL if invalid
 */
swd_result_t swd_set_turnaround_cycles(uint8_t cycles);

swd_result_t swd_write_dp_no_check(uint8_t reg, uint32_t value);
swd_result_t swd_read_dpidr_retry(uint32_t *idcode);

/**
 * Write bits to target via SWDIO.
 * @param bit_count Number of bits to write (1-256)
 * @param data Data to write (LSB first)
 */
void swd_write_bits(uint32_t bit_count, uint32_t data);

/**
 * Read bits from target via SWDIO.
 * @param bit_count Number of bits to read (1-256)
 * @return Data read (LSB first)
 */
uint32_t swd_read_bits(uint32_t bit_count);

/**
 * Clock target with SWDIO in high-impedance (input) mode.
 * Used for turnaround cycles.
 * @param bit_count Number of clock cycles
 */
void swd_hiz_clocks(uint32_t bit_count);

/* ============================================================================
 * SWD Protocol Operations
 * ============================================================================ */

/**
 * Perform SWD line reset sequence.
 * Drives SWDIO high for 50+ clock cycles, then 2 idle cycles.
 */
void swd_line_reset(void);

/**
 * Perform dormant-to-SWD wake sequence (ARM ADIv5 §5.3.4).
 */
void swd_dormant_to_swd(void);

/**
 * Switch from JTAG to SWD mode.
 * Sends the 0xE79E switching sequence.
 */
void swd_jtag_to_swd(void);

/**
 * Read target IDCODE via SWD.
 * @param idcode Pointer to store 32-bit IDCODE
 * @return SWD_OK on success
 */
swd_result_t swd_read_idcode(uint32_t *idcode);

/**
 * Write DP (Debug Port) register.
 * @param reg Register address (0x0, 0x4, 0x8, 0xC)
 * @param value 32-bit value to write
 * @return SWD_OK on success
 */
swd_result_t swd_write_dp(uint8_t reg, uint32_t value);

/**
 * Read DP (Debug Port) register.
 * @param reg Register address (0x0, 0x4, 0x8, 0xC)
 * @param value Pointer to store 32-bit value
 * @return SWD_OK on success
 */
swd_result_t swd_read_dp(uint8_t reg, uint32_t *value);

/**
 * Write AP (Access Port) register.
 * NOTE: Caller must configure CSW (AP_CSW) before first AP access.
 * @param reg Register address (0x0, 0x4, 0xC, etc.)
 * @param value 32-bit value to write
 * @return SWD_OK on success
 */
swd_result_t swd_write_ap(uint8_t reg, uint32_t value);

/**
 * Read AP (Access Port) register.
 * NOTE: Caller must configure CSW (AP_CSW) before first AP access.
 * @param reg Register address (0x0, 0x4, 0xC, etc.)
 * @param value Pointer to store 32-bit value
 * @return SWD_OK on success
 */
swd_result_t swd_read_ap(uint8_t reg, uint32_t *value);

/**
 * Perform SWD transfer (combined request + data).
 * @param request Request byte (APnDP, RnW, A[2:3])
 * @param data Pointer to data (read or write)
 * @return ACK value (0x01=OK, 0x02=WAIT, 0x04=FAULT)
 */
uint8_t swd_transfer(uint8_t request, uint32_t *data);

/* ============================================================================
 * Target Control
 * ============================================================================ */

/**
 * Assert target nRESET (pull low).
 */
void swd_assert_reset(void);

/**
 * Deassert target nRESET (release to high-Z/pull-up).
 */
void swd_deassert_reset(void);

/**
 * Read current SWDIO pin state.
 * @return true if high, false if low
 */
bool swd_read_swdo(void);

/* ============================================================================
 * Utility Functions
 * ============================================================================ */

/**
 * Calculate SWD request parity.
 * @param request Request byte (excluding parity bit)
 * @return Parity bit value (0 or 1)
 */
static inline uint8_t swd_request_parity(uint8_t request) {
    uint8_t parity = 0;
    parity ^= (request >> 1) & 1;  /* APnDP */
    parity ^= (request >> 2) & 1;  /* RnW */
    parity ^= (request >> 3) & 1;  /* A2 */
    parity ^= (request >> 4) & 1;  /* A3 */
    return parity & 1;
}

/**
 * Build SWD request byte.
 * @param apn_dp 0=DP, 1=AP
 * @param rnw 0=Write, 1=Read
 * @param a23 Register address bits [2:3]
 * @return Complete request byte with parity, stop, park
 */
static inline uint8_t swd_build_request(uint8_t apn_dp, uint8_t rnw, uint8_t a23) {
    uint8_t request = 0;
    request |= (1 << SWD_REQUEST_START);   /* Start bit */
    request |= ((apn_dp & 1) << SWD_REQUEST_APnDP);
    request |= ((rnw & 1) << SWD_REQUEST_RnW);
    request |= ((a23 & 3) << SWD_REQUEST_A23);
    /* Bit 5 = parity, bit 6 = stop (0), bit 7 = park (1). The STM32F1 SWJ-DP
     * uses this stop/park layout: wire capture showed 0xA5 (parity=1, bit6=0,
     * bit7=1) accepted for reads while 0xE5 (bits 5,6,7 all = parity) was
     * rejected with a silent all-ones ACK. */
    uint8_t parity = swd_request_parity(request);  /* after addr bits are set */
    request |= (parity << SWD_REQUEST_PARITY);
    request |= (0 << SWD_REQUEST_STOP);    /* Stop bit = 0 */
    request |= (1 << SWD_REQUEST_PARK);    /* Park bit = 1 */
    return request;
}

#endif /* AGENTLA_SWD_H */

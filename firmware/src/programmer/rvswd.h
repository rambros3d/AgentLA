/*
 * rvswd.h - WCH RVSWD (2-wire RISC-V Debug) PIO Transport
 *
 * Implements WCH's 2-wire RVSWD protocol for CH32V20x / CH32V30x
 * (QingKe V4 RISC-V cores). Uses PIO1 SM2 with:
 *   SWCLK = GPIO 4 (sideset)
 *   SWDIO = GPIO 5 (OUT/SET/IN)
 *
 * Wire frame (MSB-first):
 *   START: SWDIO H->L (CLK held H)
 *   HEADER: 9 bits [7-bit reg][1-bit op][1-bit parity]
 *   TRN:    5 bits 0b10101 (host driven)
 *   DATA:   32 bits + 1 even-parity bit (direction depends on op)
 *   TRAILER: 5 bits 0b10111
 *   STOP:   SWDIO L->H then release
 */

#ifndef AGENTLA_RVSWD_H
#define AGENTLA_RVSWD_H

#include <stdint.h>
#include <stdbool.h>
#include "pico/stdlib.h"
#include "hardware/pio.h"   /* for pio1 */

/* Pins and PIO allocation */
#define RVSWD_SWCLK_PIN   4   /* GPIO 4 — shared with SWD SWCLK */
#define RVSWD_SWDIO_PIN   5   /* GPIO 5 — shared with SWD/SWIO data */
#define RVSWD_PIO         pio1
#define RVSWD_SM          2   /* SM2 (SM0=SWD, SM1=SWIO) */

/* DMI register addresses carried in the RVSWD 7-bit address field */
#define RVSWD_REG_DMCONTROL  0x10
#define RVSWD_REG_DMSTATUS   0x11
#define RVSWD_REG_HARTINFO   0x12
#define RVSWD_REG_ABSTRACTCS 0x16
#define RVSWD_REG_COMMAND    0x17
#define RVSWD_REG_SBADDRESS0 0x39
#define RVSWD_REG_SBDATA0    0x3C

/* Result codes */
typedef enum {
    RVSWD_OK = 0,
    RVSWD_ERR_PARITY = 1,
    RVSWD_ERR_TIMEOUT = 2,
    RVSWD_ERR_NOT_INIT = 3,
} rvswd_result_t;

/*
 * Initialize the RVSWD PIO engine.
 * Loads rvswd.pio into PIO1 SM2 and configures GPIO4/5.
 * Deinitializes SWD/SWIO first since pins are shared.
 * Safe to call multiple times — no-op if already initialized.
 */
rvswd_result_t rvswd_init(void);

/*
 * Deinitialize the RVSWD PIO engine and release pins.
 */
void rvswd_deinit(void);

/*
 * Check if the RVSWD engine is initialized.
 */
bool rvswd_is_initialized(void);

/*
 * Send the RVSWD line reset sequence (SWDIO high for 300 clocks).
 */
rvswd_result_t rvswd_reset(void);

/*
 * Write a 32-bit value to a DMI register over RVSWD.
 * @param reg 7-bit DMI register address (e.g. RVSWD_REG_DMCONTROL)
 * @param value 32-bit value to write
 */
rvswd_result_t rvswd_write(uint8_t reg, uint32_t value);

/*
 * Read a 32-bit value from a DMI register over RVSWD.
 * @param reg 7-bit DMI register address (e.g. RVSWD_REG_DMSTATUS)
 * @param value out-param for the 32-bit value read
 */
rvswd_result_t rvswd_read(uint8_t reg, uint32_t *value);

#endif /* AGENTLA_RVSWD_H */

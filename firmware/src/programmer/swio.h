/*
 * AgentLA SWIO (Single-Wire Debug) PIO Engine
 * Ported from PicoRVD (https://github.com/aappleby/picorvd)
 *
 * Implements WCH's single-wire debug protocol using PIO1 SM1.
 * Provides ~2.5 MHz operation vs ~250 KHz with GPIO bit-bang.
 */

#ifndef AGENTLA_SWIO_H
#define AGENTLA_SWIO_H

#include <stdint.h>
#include <stdbool.h>
#include "pico/stdlib.h"
#include "board_settings.h"

/* SWIO Pin and PIO Configuration */
#define SWIO_DATA_PIN    5    /* GPIO 5 — shared with SWDIO */
#define SWIO_PIO         pio1
#define SWIO_SM          1    /* SM1 (SM0 used by SWD engine) */

/*
 * Initialize the SWIO PIO engine.
 * Configures PIO1 SM1, GPIO pin, and resets the target debug module.
 * Safe to call multiple times — no-op if already initialized.
 */
void swio_init(void);

/*
 * Deinitialize the SWIO PIO engine.
 * Releases PIO state machine and resets GPIO pin.
 */
void swio_deinit(void);

/*
 * Check if SWIO engine is initialized.
 * @return true if initialized, false otherwise
 */
bool swio_is_initialized(void);

/*
 * Write a 32-bit value to a debug register via SWIO.
 * @param addr 7-bit debug register address
 * @param data 32-bit value to write
 */
void swio_put(uint32_t addr, uint32_t data);

/*
 * Read a 32-bit value from a debug register via SWIO.
 * @param addr 7-bit debug register address
 * @return 32-bit value read from register
 */
uint32_t swio_get(uint32_t addr);

/*
 * Reset the SWIO interface (deinit + reinit).
 */
void swio_reset(void);

#endif /* AGENTLA_SWIO_H */

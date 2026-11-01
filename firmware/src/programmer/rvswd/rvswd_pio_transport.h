/*
 * rvswd_pio_transport.h - RVSWD PIO Transport Driver for AgentLA
 *
 * Vendored and adapted from RVSWD_pico (https://github.com/Nicolai-Electronics/RVSWD_pico).
 * License: MIT
 */

#ifndef AGENTLA_RVSWD_PIO_TRANSPORT_H
#define AGENTLA_RVSWD_PIO_TRANSPORT_H

#include <stdint.h>
#include <stdbool.h>
#include "pico/stdlib.h"
#include "hardware/pio.h"
#include "programmer/rvswd.h"

/* Pins and PIO configuration (PIO0 SM0 shared dynamically with ARM SWD) */
#define RVSWD_PIO_INSTANCE  pio0
#define RVSWD_PIO_SM        0
#define RVSWD_SWCLK_PIN     4   /* GPIO 4 — SWCLK */
#define RVSWD_SWDIO_PIN     5   /* GPIO 5 — SWDIO */

/* Handle tracking PIO offset and init state */
typedef struct {
    PIO pio;
    uint sm;
    uint pio_offset;
    bool initialized;
} rvswd_transport_handle_t;

/* Initialize RVSWD transport on PIO0 SM0 (unloads ARM SWD if active). */
rvswd_result_t rvswd_transport_init(void);

/* Deinitialize RVSWD transport and remove program from PIO0. */
void rvswd_transport_deinit(void);

/* Check if RVSWD transport is active. */
bool rvswd_transport_is_initialized(void);

/* Set/get the PIO clock divider (default 30.0 = 4.17 MHz SWCLK). Takes
 * effect on the next rvswd_transport_init(). */
void rvswd_transport_set_clkdiv(float div);
float rvswd_transport_get_clkdiv(void);

/* Send line reset (300 cycles of SWDIO=H). */
rvswd_result_t rvswd_transport_reset(void);

/* Read 32-bit DMI register. */
rvswd_result_t rvswd_transport_read(uint8_t reg, uint32_t *value);

/* Write 32-bit DMI register. */
rvswd_result_t rvswd_transport_write(uint8_t reg, uint32_t value);

#endif /* AGENTLA_RVSWD_PIO_TRANSPORT_H */

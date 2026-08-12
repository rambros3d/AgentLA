/*
 * rvswd.c - WCH RVSWD (2-wire RISC-V Debug) PIO Transport
 *
 * Ported from RVSWD_pico (https://github.com/Nicolai-Electronics/RVSWD_pico)
 * and informed by PicoRVD (https://github.com/aappleby/picorvd).
 * License: MIT
 *
 * Wire frame (MSB-first):
 *   START  : SWDIO=H,CLK=H -> SWDIO=L,CLK=H
 *   HEADER : 9 bits [7-bit reg][1-bit op][1-bit parity] (even parity of reg+op)
 *   TRN    : 5 bits 0b10101 (host driven)
 *   WRITE  : 32-bit data + 1 even-parity bit
 *   READ   : 32-bit data (target) + 1 parity bit
 *   TRAILER: 5 bits 0b10111 (host driven)
 *   STOP   : SWDIO=L,CLK=H -> SWDIO=H,CLK=H -> release SWDIO
 *   RESET  : SWDIO=H for 300 CLK cycles, then STOP
 */

#include "rvswd.h"
#include "swd.h"
#include "swio.h"
#include "rvswd.pio.h"
#include "hardware/gpio.h"
#include "hardware/pio.h"
#include <stdio.h>

/* PIO clock divider: 125 MHz / 15 ≈ 8.33 MHz (matches RVSWD_pico reference) */
#define RVSWD_PIO_CLK_DIV 15.0f

static PIO rvswd_pio = RVSWD_PIO;
static uint rvswd_sm = RVSWD_SM;
static uint rvswd_offset = 0;
static bool rvswd_initialized = false;

/*
 * Bounded PIO FIFO put. A stalled SM (e.g. no target on the bus) must NEVER
 * freeze the single-threaded firmware — pio_sm_put_blocking() would spin
 * forever once the TX FIFO fills. Returns false on timeout.
 */
static bool rvswd_put_bounded(uint32_t data) {
    absolute_time_t deadline = make_timeout_time_us(1000);
    while (pio_sm_is_tx_fifo_full(rvswd_pio, rvswd_sm)) {
        if (time_reached(deadline)) {
            pio_sm_clear_fifos(rvswd_pio, rvswd_sm);
            return false;
        }
        tight_loop_contents();
    }
    pio_sm_put(rvswd_pio, rvswd_sm, data);
    return true;
}

/*
 * Formats and sends a command word to the PIO.
 * Command format: | 31:24 count-1 | 23 dir(1=write) | 22:18 cmd addr |
 * Returns false if the FIFO never drained (stalled SM).
 */
static bool rvswd_send_cmd(uint cmd_offset, uint bit_count, bool out_en) {
    uint count_val = (bit_count == 0) ? 0 : (bit_count - 1);
    cmd_offset += rvswd_offset;
    uint32_t cmd = ((uint32_t)count_val << 24) |
                   ((uint32_t)out_en << 23) |
                   ((uint32_t)cmd_offset << 18);
    return rvswd_put_bounded(cmd);
}

/* Write N bits (max 32) — data left-aligned, MSB-first on the wire. */
static inline bool rvswd_write_bits(uint32_t data, uint8_t bit_count) {
    if (bit_count < 32)
        data <<= (32 - bit_count);
    if (!rvswd_send_cmd(rvswd_io_offset_write_bits, bit_count, true))
        return false;
    return rvswd_put_bounded(data);
}

/* Read N bits (max 32) — first sampled bit lands in bit 31. */
static inline uint32_t rvswd_read_bits(uint8_t bit_count) {
    rvswd_send_cmd(rvswd_io_offset_read_bits, bit_count, false);
    /* Bounded wait — a stalled SM (e.g. no target) must never freeze the
     * single-threaded firmware. Mirror the swio_get() 1ms timeout pattern. */
    absolute_time_t deadline = make_timeout_time_us(1000);
    while (pio_sm_is_rx_fifo_empty(rvswd_pio, rvswd_sm)) {
        if (time_reached(deadline)) {
            pio_sm_clear_fifos(rvswd_pio, rvswd_sm);
            return 0xFFFFFFFF;
        }
        tight_loop_contents();
    }
    return pio_sm_get(rvswd_pio, rvswd_sm);
}

static inline bool rvswd_start(void) {
    return rvswd_send_cmd(rvswd_io_offset_start, 0, true);
}

static inline bool rvswd_stop(void) {
    return rvswd_send_cmd(rvswd_io_offset_stop, 0, true);
}

rvswd_result_t rvswd_init(void) {
    if (rvswd_initialized) return RVSWD_OK;

    /* Release pins from any other transport (SWD/SWIO share GPIO4/5). */
    swd_deinit();
    swio_deinit();

    /* Configure GPIOs for PIO */
    gpio_init(RVSWD_SWCLK_PIN);
    gpio_set_function(RVSWD_SWCLK_PIN, GPIO_FUNC_PIO1);
    gpio_set_drive_strength(RVSWD_SWCLK_PIN, GPIO_DRIVE_STRENGTH_2MA);
    gpio_set_slew_rate(RVSWD_SWCLK_PIN, GPIO_SLEW_RATE_SLOW);

    gpio_init(RVSWD_SWDIO_PIN);
    gpio_set_function(RVSWD_SWDIO_PIN, GPIO_FUNC_PIO1);
    gpio_set_pulls(RVSWD_SWDIO_PIN, true, false);  /* pull-up */
    gpio_set_drive_strength(RVSWD_SWDIO_PIN, GPIO_DRIVE_STRENGTH_2MA);
    gpio_set_slew_rate(RVSWD_SWDIO_PIN, GPIO_SLEW_RATE_SLOW);

    /* Load PIO program */
    pio_clear_instruction_memory(rvswd_pio);
    rvswd_offset = pio_add_program(rvswd_pio, &rvswd_io_program);

    /* Configure the state machine */
    pio_sm_config c = rvswd_io_program_get_default_config(rvswd_offset);
    sm_config_set_out_pins(&c, RVSWD_SWDIO_PIN, 1);
    sm_config_set_set_pins(&c, RVSWD_SWDIO_PIN, 1);
    sm_config_set_in_pins(&c, RVSWD_SWDIO_PIN);
    sm_config_set_sideset_pins(&c, RVSWD_SWCLK_PIN);
    sm_config_set_clkdiv(&c, RVSWD_PIO_CLK_DIV);
    sm_config_set_out_shift(&c, false, false, 32);  /* MSB-first */
    sm_config_set_in_shift(&c, false, false, 32);

    /* Initialize GPIOs for PIO */
    pio_gpio_init(rvswd_pio, RVSWD_SWCLK_PIN);
    pio_gpio_init(rvswd_pio, RVSWD_SWDIO_PIN);
    gpio_set_pulls(RVSWD_SWDIO_PIN, true, false);  /* keep pull-up */
    pio_sm_set_consecutive_pindirs(rvswd_pio, rvswd_sm, RVSWD_SWDIO_PIN, 1, true);
    pio_sm_set_consecutive_pindirs(rvswd_pio, rvswd_sm, RVSWD_SWCLK_PIN, 1, true);

    /* Load config, jump to dispatcher, and start */
    pio_sm_init(rvswd_pio, rvswd_sm, rvswd_offset, &c);
    uint entry_point = rvswd_offset + rvswd_io_offset_get_next_cmd;
    pio_sm_exec(rvswd_pio, rvswd_sm, entry_point);
    pio_sm_set_enabled(rvswd_pio, rvswd_sm, true);

    rvswd_initialized = true;
    return RVSWD_OK;
}

void rvswd_deinit(void) {
    if (!rvswd_initialized) return;
    pio_sm_set_enabled(rvswd_pio, rvswd_sm, false);
    pio_remove_program(rvswd_pio, &rvswd_io_program, rvswd_offset);
    gpio_init(RVSWD_SWCLK_PIN);
    gpio_init(RVSWD_SWDIO_PIN);
    gpio_set_dir(RVSWD_SWDIO_PIN, GPIO_IN);
    rvswd_initialized = false;
}

bool rvswd_is_initialized(void) {
    return rvswd_initialized;
}

rvswd_result_t rvswd_reset(void) {
    if (!rvswd_initialized) return RVSWD_ERR_NOT_INIT;
    /* NOTE: the PIO command count field is 8 bits (count-1 in bits 31:24).
     * 300 would truncate (299 & 0xFF = 43), so use 256 which encodes
     * count_val=255 cleanly — 256 clocks of SWDIO=H is a valid line reset. */
    if (!rvswd_send_cmd(rvswd_io_offset_reset, 256, true))
        return RVSWD_ERR_TIMEOUT;
    return RVSWD_OK;
}

/* Build the 9-bit header word: [reg7][op][parity], even parity of reg+op. */
static uint32_t rvswd_build_header(uint8_t reg, bool is_write) {
    uint32_t packet = (uint32_t)reg & 0x7F;
    bool parity = false;
    for (uint8_t i = 0; i < 7; i++) {
        if ((packet >> (6 - i)) & 1) parity = !parity;
    }
    packet = (packet << 1) | (is_write ? 1 : 0);
    if (is_write) parity = !parity;
    packet = (packet << 1) | (parity ? 1 : 0);
    return packet;
}

rvswd_result_t rvswd_write(uint8_t reg, uint32_t value) {
    if (!rvswd_initialized) return RVSWD_ERR_NOT_INIT;

    if (!rvswd_start()) return RVSWD_ERR_TIMEOUT;
    if (!rvswd_write_bits(rvswd_build_header(reg, true), 9)) return RVSWD_ERR_TIMEOUT;
    if (!rvswd_write_bits(0b10101, 5)) return RVSWD_ERR_TIMEOUT;  /* turnaround */

    bool parity = false;
    for (uint8_t i = 0; i < 32; i++) {
        if ((value >> (31 - i)) & 1) parity = !parity;
    }
    if (!rvswd_write_bits(value, 32)) return RVSWD_ERR_TIMEOUT;
    if (!rvswd_write_bits(parity ? 1 : 0, 1)) return RVSWD_ERR_TIMEOUT;
    if (!rvswd_write_bits(0b10111, 5)) return RVSWD_ERR_TIMEOUT;  /* trailer */
    if (!rvswd_stop()) return RVSWD_ERR_TIMEOUT;

    return RVSWD_OK;
}

rvswd_result_t rvswd_read(uint8_t reg, uint32_t *value) {
    if (!rvswd_initialized) return RVSWD_ERR_NOT_INIT;
    if (!value) return RVSWD_ERR_TIMEOUT;

    if (!rvswd_start()) return RVSWD_ERR_TIMEOUT;
    if (!rvswd_write_bits(rvswd_build_header(reg, false), 9)) return RVSWD_ERR_TIMEOUT;
    if (!rvswd_write_bits(0b10101, 5)) return RVSWD_ERR_TIMEOUT;  /* turnaround */

    *value = rvswd_read_bits(32);
    bool parity_read = (rvswd_read_bits(1) & 1);

    if (!rvswd_write_bits(0b10111, 5)) return RVSWD_ERR_TIMEOUT;  /* trailer */
    if (!rvswd_stop()) return RVSWD_ERR_TIMEOUT;

    /* Verify data parity */
    bool data_parity = false;
    for (uint8_t i = 0; i < 32; i++) {
        if ((*value >> (31 - i)) & 1) data_parity = !data_parity;
    }
    return (data_parity == parity_read) ? RVSWD_OK : RVSWD_ERR_PARITY;
}

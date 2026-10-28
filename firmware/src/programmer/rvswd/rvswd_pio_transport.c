/*
 * rvswd_pio_transport.c - RVSWD PIO Transport Driver for AgentLA
 *
 * Vendored and adapted from RVSWD_pico (https://github.com/Nicolai-Electronics/RVSWD_pico).
 * License: MIT
 *
 * Local adaptations (AgentLA):
 *   - PIO0 SM0 shared dynamically with ARM SWD (debugprobe); swd_deinit()
 *     unloads probe.pio before rvswd.pio is loaded, and rvswd_transport_deinit()
 *     removes rvswd.pio via pio_remove_program() so PIO0 instruction memory is
 *     managed cleanly (no pio_clear_instruction_memory()).
 *   - All FIFO puts and gets are BOUNDED by a 1ms deadline. A stalled state
 *     machine (e.g. no target on the bus) can never freeze the single-threaded
 *     firmware. On timeout the FIFOs are cleared and an error is propagated.
 *   - MSB-first, left-aligned data shifts (matches RVSWD_pico reference).
 */

#include "rvswd_pio_transport.h"
#include "programmer/swd.h"
#include "programmer/swio.h"
#include "programmer/programmer.h"
#include "rvswd.pio.h"
#include "hardware/gpio.h"
#include "hardware/clocks.h"
#include <stdio.h>

#define RVSWD_PIO_CLK_DIV 30.0f
#define RVSWD_TIMEOUT_US 1000

static rvswd_transport_handle_t g_rvswd = {
    .pio = RVSWD_PIO_INSTANCE,
    .sm = RVSWD_PIO_SM,
    .pio_offset = 0,
    .initialized = false,
};

/*
 * Full state-machine recovery. After a bounded-FIFO timeout the SM may be
 * stuck mid-routine (blocked on pull/push, or jumped to a garbage address
 * via a desynced command word) — pio_sm_clear_fifos() alone leaves it
 * desynced and the next transactions fail, wedging the probe for minutes
 * (F9). Return the SM to the command dispatcher in a known state, mirroring
 * the proven tail of rvswd_transport_init().
 */
static void rvswd_transport_recover(void) {
    pio_sm_set_enabled(g_rvswd.pio, g_rvswd.sm, false);
    pio_sm_clear_fifos(g_rvswd.pio, g_rvswd.sm);
    pio_sm_restart(g_rvswd.pio, g_rvswd.sm);
    uint entry_point = g_rvswd.pio_offset + rvswd_io_offset_get_next_cmd;
    pio_sm_exec(g_rvswd.pio, g_rvswd.sm, entry_point);
    pio_sm_set_enabled(g_rvswd.pio, g_rvswd.sm, true);
}

/*
 * Bounded PIO FIFO put. A stalled SM (e.g. no target on the bus) must NEVER
 * freeze the single-threaded firmware — pio_sm_put_blocking() would spin
 * forever once the TX FIFO fills. Returns false on timeout, after recovering
 * the SM so the next transaction starts clean.
 */
static bool rvswd_put_bounded(uint32_t data) {
    absolute_time_t deadline = make_timeout_time_us(RVSWD_TIMEOUT_US);
    while (pio_sm_is_tx_fifo_full(g_rvswd.pio, g_rvswd.sm)) {
        if (time_reached(deadline)) {
            rvswd_transport_recover();
            return false;
        }
        tight_loop_contents();
    }
    pio_sm_put(g_rvswd.pio, g_rvswd.sm, data);
    return true;
}

/*
 * Bounded PIO FIFO get. Returns false on timeout (RX FIFO never filled),
 * leaving *data untouched, after recovering the SM (F9). Mirror of the
 * swio_get() 1ms timeout pattern.
 */
static bool rvswd_get_bounded(uint32_t *data) {
    absolute_time_t deadline = make_timeout_time_us(RVSWD_TIMEOUT_US);
    while (pio_sm_is_rx_fifo_empty(g_rvswd.pio, g_rvswd.sm)) {
        if (time_reached(deadline)) {
            rvswd_transport_recover();
            return false;
        }
        tight_loop_contents();
    }
    *data = pio_sm_get(g_rvswd.pio, g_rvswd.sm);
    return true;
}

/*
 * Formats and sends a command word to the PIO.
 * Command format: | 31:24 count-1 | 23 dir(1=write) | 22:18 cmd addr |
 * Returns false if the FIFO never drained (stalled SM).
 */
static bool rvswd_send_cmd(uint cmd_offset, uint bit_count, bool out_en) {
    uint count_val = (bit_count == 0) ? 0 : (bit_count - 1);
    cmd_offset += g_rvswd.pio_offset;
    uint32_t cmd = ((uint32_t)count_val << 24) |
                   ((uint32_t)out_en << 23) |
                   ((uint32_t)cmd_offset << 18);
    return rvswd_put_bounded(cmd);
}

/* Write N bits (max 32) — data left-aligned, MSB-first on the wire. */
static bool rvswd_write_bits(uint32_t data, uint8_t bit_count) {
    if (bit_count < 32)
        data <<= (32 - bit_count);
    if (!rvswd_send_cmd(rvswd_io_offset_write_bits, bit_count, true))
        return false;
    return rvswd_put_bounded(data);
}

/* Read N bits (max 32) — first sampled bit lands in bit 31. */
static bool rvswd_read_bits(uint8_t bit_count, uint32_t *out) {
    if (!rvswd_send_cmd(rvswd_io_offset_read_bits, bit_count, false))
        return false;
    return rvswd_get_bounded(out);
}

static bool rvswd_start_seq(void) {
    return rvswd_send_cmd(rvswd_io_offset_start, 0, true);
}

static bool rvswd_stop_seq(void) {
    return rvswd_send_cmd(rvswd_io_offset_stop, 0, true);
}

rvswd_result_t rvswd_transport_init(void) {
    if (g_rvswd.initialized) return RVSWD_OK;

    /* Release UART adapter mode if active on the shared GPIO4-5 pins.
     * Mirrors swd_init(): the programmer owns these pins, so leaving UART
     * mode active would let the UART peripheral fight PIO for the pads. */
    extern volatile bool programmer_mode_active;
    if (programmer_mode_active) {
        prog_exit_mode();
    }

    /* Safely release any other active transport (SWD on PIO0, SWIO on PIO1).
     * swd_deinit() -> probe_deinit() unloads probe.pio via pio_remove_program(),
     * freeing PIO0 instruction memory for rvswd.pio. */
    swd_deinit();
    swio_deinit();

    /* Ensure nRESET (GPIO 6) is released high */
    gpio_init(6);
    gpio_set_dir(6, GPIO_IN);
    gpio_pull_up(6);

    /* Configure GPIOs for PIO0 */
    gpio_init(RVSWD_SWCLK_PIN);
    gpio_set_function(RVSWD_SWCLK_PIN, GPIO_FUNC_PIO0);
    gpio_set_drive_strength(RVSWD_SWCLK_PIN, GPIO_DRIVE_STRENGTH_2MA);
    gpio_set_slew_rate(RVSWD_SWCLK_PIN, GPIO_SLEW_RATE_SLOW);

    gpio_init(RVSWD_SWDIO_PIN);
    gpio_set_function(RVSWD_SWDIO_PIN, GPIO_FUNC_PIO0);
    gpio_set_pulls(RVSWD_SWDIO_PIN, true, false);  /* pull-up */
    gpio_set_drive_strength(RVSWD_SWDIO_PIN, GPIO_DRIVE_STRENGTH_2MA);
    gpio_set_slew_rate(RVSWD_SWDIO_PIN, GPIO_SLEW_RATE_SLOW);

    /* Load PIO program dynamically into PIO0 (probe.pio was removed above,
     * so 25 instructions fit within the 32-instruction block). */
    g_rvswd.pio_offset = pio_add_program(g_rvswd.pio, &rvswd_io_program);

    /* Configure the state machine */
    pio_sm_config c = rvswd_io_program_get_default_config(g_rvswd.pio_offset);
    sm_config_set_out_pins(&c, RVSWD_SWDIO_PIN, 1);
    sm_config_set_set_pins(&c, RVSWD_SWDIO_PIN, 1);
    sm_config_set_in_pins(&c, RVSWD_SWDIO_PIN);
    sm_config_set_sideset_pins(&c, RVSWD_SWCLK_PIN);
    sm_config_set_clkdiv(&c, RVSWD_PIO_CLK_DIV);
    sm_config_set_out_shift(&c, false, false, 32);  /* MSB-first */
    sm_config_set_in_shift(&c, false, false, 32);

    /* Initialize GPIOs for PIO */
    pio_gpio_init(g_rvswd.pio, RVSWD_SWCLK_PIN);
    pio_gpio_init(g_rvswd.pio, RVSWD_SWDIO_PIN);
    gpio_set_pulls(RVSWD_SWDIO_PIN, true, false);
    pio_sm_set_consecutive_pindirs(g_rvswd.pio, g_rvswd.sm, RVSWD_SWCLK_PIN, 2, true);

    /* Clear FIFOs and reset debug flags */
    pio_sm_clear_fifos(g_rvswd.pio, g_rvswd.sm);
    g_rvswd.pio->fdebug = 0xffffffff;

    /* Load config, jump to dispatcher, and start */
    pio_sm_init(g_rvswd.pio, g_rvswd.sm, g_rvswd.pio_offset, &c);
    uint entry_point = g_rvswd.pio_offset + rvswd_io_offset_get_next_cmd;
    pio_sm_exec(g_rvswd.pio, g_rvswd.sm, entry_point);
    pio_sm_set_enabled(g_rvswd.pio, g_rvswd.sm, true);

    g_rvswd.initialized = true;
    rvswd_transport_reset();
    return RVSWD_OK;
}

void rvswd_transport_deinit(void) {
    if (!g_rvswd.initialized) return;

    pio_sm_set_enabled(g_rvswd.pio, g_rvswd.sm, false);
    pio_remove_program(g_rvswd.pio, &rvswd_io_program, g_rvswd.pio_offset);

    /* Restore GPIOs 4 and 5 to safe float mode */
    gpio_init(RVSWD_SWCLK_PIN);
    gpio_set_dir(RVSWD_SWCLK_PIN, GPIO_IN);
    gpio_disable_pulls(RVSWD_SWCLK_PIN);

    gpio_init(RVSWD_SWDIO_PIN);
    gpio_set_dir(RVSWD_SWDIO_PIN, GPIO_IN);
    gpio_disable_pulls(RVSWD_SWDIO_PIN);

    g_rvswd.initialized = false;
}

bool rvswd_transport_is_initialized(void) {
    return g_rvswd.initialized;
}

rvswd_result_t rvswd_transport_reset(void) {
    if (!g_rvswd.initialized) return RVSWD_ERR_NOT_INIT;

    /* Self-heal first: a fresh reset (e.g. CH32 DETECT retry after a wedge)
     * must start from a known SM state (F9). */
    rvswd_transport_recover();

    /* NOTE: the PIO command count field is 8 bits (count-1 in bits 31:24).
     * 300 would truncate (299 & 0xFF = 43), so use 256 which encodes
     * count_val=255 cleanly — 256 clocks of SWDIO=H is a valid line reset. */
    if (!rvswd_send_cmd(rvswd_io_offset_reset, 256, true))
        return RVSWD_ERR_TIMEOUT;
    sleep_us(500);

    /* Awaken WCH Debug Module: set dmactive (0x80000001 -> 0x00000001) */
    rvswd_transport_write(0x10, 0x80000001);
    rvswd_transport_write(0x10, 0x80000001);
    rvswd_transport_write(0x10, 0x00000001);
    sleep_ms(5);

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

rvswd_result_t rvswd_transport_write(uint8_t reg, uint32_t value) {
    if (!g_rvswd.initialized) return RVSWD_ERR_NOT_INIT;

    if (!rvswd_start_seq()) return RVSWD_ERR_TIMEOUT;
    if (!rvswd_write_bits(rvswd_build_header(reg, true), 9)) return RVSWD_ERR_TIMEOUT;
    if (!rvswd_write_bits(0b10101, 5)) return RVSWD_ERR_TIMEOUT;  /* turnaround */

    bool parity = false;
    for (uint8_t i = 0; i < 32; i++) {
        if ((value >> (31 - i)) & 1) parity = !parity;
    }
    if (!rvswd_write_bits(value, 32)) return RVSWD_ERR_TIMEOUT;
    if (!rvswd_write_bits(parity ? 1 : 0, 1)) return RVSWD_ERR_TIMEOUT;
    if (!rvswd_write_bits(0b10111, 5)) return RVSWD_ERR_TIMEOUT;  /* trailer */
    if (!rvswd_stop_seq()) return RVSWD_ERR_TIMEOUT;

    return RVSWD_OK;
}

rvswd_result_t rvswd_transport_read(uint8_t reg, uint32_t *value) {
    if (!g_rvswd.initialized) return RVSWD_ERR_NOT_INIT;
    if (!value) return RVSWD_ERR_TIMEOUT;

    if (!rvswd_start_seq()) return RVSWD_ERR_TIMEOUT;
    if (!rvswd_write_bits(rvswd_build_header(reg, false), 9)) return RVSWD_ERR_TIMEOUT;
    if (!rvswd_write_bits(0b10101, 5)) return RVSWD_ERR_TIMEOUT;  /* turnaround */

    uint32_t data = 0;
    if (!rvswd_read_bits(32, &data)) return RVSWD_ERR_TIMEOUT;
    uint32_t parity_read = 0;
    if (!rvswd_read_bits(1, &parity_read)) return RVSWD_ERR_TIMEOUT;
    *value = data;

    if (!rvswd_write_bits(0b10111, 5)) return RVSWD_ERR_TIMEOUT;  /* trailer */
    if (!rvswd_stop_seq()) return RVSWD_ERR_TIMEOUT;

    /* Verify data parity. NOTE: a read of 0xFFFFFFFF is NOT treated as an
     * error here — erased flash reads legitimately return all-ones. The
     * callers (ch32_probe_target / riscv_debug_init) detect a floating bus
     * by checking for 0xFFFFFFFF at the DMI level. */
    bool data_parity = false;
    for (uint8_t i = 0; i < 32; i++) {
        if ((data >> (31 - i)) & 1) data_parity = !data_parity;
    }
    return (data_parity == (parity_read & 1)) ? RVSWD_OK : RVSWD_ERR_PARITY;
}

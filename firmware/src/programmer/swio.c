/*
 * AgentLA SWIO (Single-Wire Debug) PIO Engine
 * Ported from PicoRVD (https://github.com/aappleby/picorvd)
 *
 * Implements WCH's single-wire debug protocol using PIO1 SM1.
 * Provides ~2.5 MHz operation vs ~250 KHz with GPIO bit-bang.
 *
 * Protocol: Start bit + 7-bit inverted address + R/W bit → data (32 bits)
 * Timing: Normal mode, PIO clock = 125MHz / 12 ≈ 10.4MHz
 */

#include "swio.h"
#include "board_settings.h"
#include "swio.pio.h"
#include "hardware/gpio.h"
#include "hardware/pio.h"
#include <stdio.h>

/* WCH-specific debug interface config registers */
#define WCH_DM_CPBR       0x7C
#define WCH_DM_CFGR       0x7D
#define WCH_DM_SHDWCFGR   0x7E
#define WCH_DM_PART       0x7F

/* DM register addresses (from RISC-V Debug Spec) */
#define DM_DMCONTROL      0x10
#define DM_DMSTATUS       0x11

/* SWIO state */
static PIO swio_pio = SWIO_PIO;
static uint swio_sm = SWIO_SM;
static uint swio_offset = 0;
static bool swio_initialized = false;

/* Helper: busy wait using cycle counting */
static inline void swio_busy_wait(int count) {
    volatile int c = count;
    while (c) c = c - 1;
}

void swio_init(void) {
    if (swio_initialized) return;

    /* Configure GPIO pin for PIO */
    gpio_set_drive_strength(SWIO_DATA_PIN, GPIO_DRIVE_STRENGTH_2MA);
    gpio_set_slew_rate(SWIO_DATA_PIN, GPIO_SLEW_RATE_SLOW);
    gpio_set_function(SWIO_DATA_PIN, GPIO_FUNC_PIO1);

    /* Reset PIO state machine */
    pio_sm_set_enabled(swio_pio, swio_sm, false);

    /* Upload PIO program */
    pio_clear_instruction_memory(swio_pio);
    swio_offset = pio_add_program(swio_pio, &swio_program);

    /* Configure PIO state machine */
    pio_sm_config c = swio_program_get_default_config(swio_offset);

    /* Pin mapping: all on SWIO_DATA_PIN */
    sm_config_set_out_pins(&c, SWIO_DATA_PIN, 1);
    sm_config_set_in_pins(&c, SWIO_DATA_PIN);
    sm_config_set_set_pins(&c, SWIO_DATA_PIN, 1);
    sm_config_set_sideset_pins(&c, SWIO_DATA_PIN);

    /* Shift registers: MSB-first */
    sm_config_set_out_shift(&c, false, false, 32);
    sm_config_set_in_shift(&c, false, true, 32);  /* autopush enabled */

    /* Clock divider: 125 MHz / 12 ≈ 10.4 MHz (96ns per tick) */
    sm_config_set_clkdiv(&c, 12.0f);

    /* Initialize PIO */
    pio_sm_init(swio_pio, swio_sm, swio_offset, &c);
    pio_sm_set_pins(swio_pio, swio_sm, 0);
    pio_sm_set_enabled(swio_pio, swio_sm, true);

    /* Send 8µs low pulse to reset debug module on target */
    /* Must use raw SIO access to avoid PIO jitter */
    sio_hw->gpio_clr = (1 << SWIO_DATA_PIN);
    sio_hw->gpio_oe_set = (1 << SWIO_DATA_PIN);
    io_bank0_hw->io[SWIO_DATA_PIN].ctrl = GPIO_FUNC_SIO << IO_BANK0_GPIO0_CTRL_FUNCSEL_LSB;
    swio_busy_wait(100);  /* ~8µs at 125MHz */
    sio_hw->gpio_oe_clr = (1 << SWIO_DATA_PIN);
    io_bank0_hw->io[SWIO_DATA_PIN].ctrl = GPIO_FUNC_PIO1 << IO_BANK0_GPIO0_CTRL_FUNCSEL_LSB;

    /* Enable debug output pin on target */
    swio_put(WCH_DM_SHDWCFGR, 0x5AA50400);
    swio_put(WCH_DM_CFGR,     0x5AA50400);

    /* Reset debug module on target */
    swio_put(DM_DMCONTROL, 0x00000000);
    swio_put(DM_DMCONTROL, 0x00000001);

    swio_initialized = true;
}

void swio_deinit(void) {
    if (!swio_initialized) return;
    pio_sm_set_enabled(swio_pio, swio_sm, false);
    pio_remove_program(swio_pio, &swio_program, swio_offset);
    gpio_init(SWIO_DATA_PIN);
    gpio_set_dir(SWIO_DATA_PIN, GPIO_IN);
    swio_initialized = false;
}

bool swio_is_initialized(void) {
    return swio_initialized;
}

/*
 * Write a 32-bit value to a debug register via SWIO.
 *
 * PIO command word format:
 *   [24:1] = inverted address (24 bits, MSB first)
 *   [0]    = direction (0 = write)
 *
 * For write: after address phase, push inverted 32-bit data word.
 */
void swio_put(uint32_t addr, uint32_t data) {
    /* Encode: inverted address << 1 | direction(0=write) */
    uint32_t cmd = ((~addr) << 1) | 0;
    /* Wait for TX FIFO with timeout */
    absolute_time_t deadline = make_timeout_time_us(1000);
    while (pio_sm_is_tx_fifo_full(swio_pio, swio_sm)) {
        if (time_reached(deadline)) return;
    }
    pio_sm_put(swio_pio, swio_sm, cmd);
    /* Push inverted data word (PIO transmits LSB first, target expects inverted) */
    deadline = make_timeout_time_us(1000);
    while (pio_sm_is_tx_fifo_full(swio_pio, swio_sm)) {
        if (time_reached(deadline)) return;
    }
    pio_sm_put(swio_pio, swio_sm, ~data);
}

/*
 * Read a 32-bit value from a debug register via SWIO.
 *
 * PIO command word format:
 *   [24:1] = inverted address (24 bits, MSB first)
 *   [0]    = direction (1 = read)
 *
 * For read: PIO receives 32 bits into RX FIFO.
 */
uint32_t swio_get(uint32_t addr) {
    /* Encode: inverted address << 1 | direction(1=read) */
    uint32_t cmd = ((~addr) << 1) | 1;
    pio_sm_put_blocking(swio_pio, swio_sm, cmd);
    /* Read 32-bit response from RX FIFO with timeout */
    absolute_time_t deadline = make_timeout_time_us(1000); /* 1ms timeout */
    while (pio_sm_is_rx_fifo_empty(swio_pio, swio_sm)) {
        if (time_reached(deadline)) {
            /* Timeout - clear FIFO and return error */
            pio_sm_clear_fifos(swio_pio, swio_sm);
            return 0xFFFFFFFF;
        }
        tight_loop_contents();
    }
    return pio_sm_get(swio_pio, swio_sm);
}

void swio_reset(void) {
    if (!swio_initialized) return;

    /* Deinitialize and reinitialize to reset state */
    swio_deinit();
    swio_init();
}

/*
 * AgentLA SWD Engine
 * Based on MioLink & Black Magic Debug
 *
 * PIO-based SWD implementation for RP2350 PIO1 with modular origin sub-programs,
 * hardware ACK checking, DMA transfers, and ADIv5 dormant-to-SWD wake sequence.
 */

#include "swd.h"
#include "tap_pio.h"
#include "board_settings.h"
#include "programmer.h"
#include "usb_hid.h"
#include <string.h>

/* --- LED diagnostic toggle (compile-time gated) --- */
#ifdef SWD_DIAG_LED
#define DIAG_LED_PIN 25
#include "hardware/gpio.h"
static inline void diag_led_init(void) { gpio_init(DIAG_LED_PIN); gpio_set_dir(DIAG_LED_PIN, GPIO_OUT); }
static inline void diag_led_on(void)  { gpio_put(DIAG_LED_PIN, 1); }
static inline void diag_led_off(void) { gpio_put(DIAG_LED_PIN, 0); }
#else
static inline void diag_led_init(void) {}
static inline void diag_led_on(void)  {}
static inline void diag_led_off(void) {}
#endif


/* Track SWDIO direction state */
typedef enum {
    SWDIO_DIR_FLOAT = 0,
    SWDIO_DIR_DRIVE
} swdio_dir_t;

volatile swdio_dir_t current_swdio_dir = SWDIO_DIR_FLOAT;

/* Debug: last raw ACK and FIFO state */
volatile uint32_t swd_debug_raw_ack = 0;
volatile uint32_t swd_debug_rx_count = 0;
volatile uint32_t swd_debug_tx_stall = 0;
volatile uint32_t swd_debug_first_ack = 0;
volatile uint32_t swd_debug_first_idcode = 0;
/* Bitbang transport diagnostics */
volatile uint32_t swd_debug_bb_ack_bits = 0;   /* raw 3-bit ACK sampled */
volatile uint32_t swd_debug_bb_dir = 0;        /* SWDIO dir at transfer start */
volatile uint32_t swd_debug_bb_last_err = 0;   /* 0=none 1=ack!=OK 2=parity 3=timeout */
volatile uint32_t swd_debug_bb_request = 0;    /* last request byte sent */
volatile uint32_t swd_debug_bb_rpar = 0;       /* parity bit read on wire */
volatile uint32_t swd_debug_bb_cpar = 0;       /* parity calculated on host */
volatile uint32_t swd_debug_bb_wack = 0;       /* write-path ACK */

/* Global SWD state */
swd_state_t swd_state = {
    .pio = SWD_PIO,
    .sm = SWD_SM,
    .clock_freq = SWD_FREQ_DEFAULT,
    .initialized = false,
    .last_idcode = 0,
    .turnaround_cycles = 1,
    .idle_cycles = 8,  /* MioLink uses 8; SWD spec requires idle cycles after writes */
    .ap_select = 0
};

/* ADIv5 Selection Alert Sequence (128 bits, LSB first) */
#define ADIV5_SELECTION_ALERT_SEQUENCE_0 0x6209f392U
#define ADIV5_SELECTION_ALERT_SEQUENCE_1 0x86852d95U
#define ADIV5_SELECTION_ALERT_SEQUENCE_2 0xe3ddafe9U
#define ADIV5_SELECTION_ALERT_SEQUENCE_3 0x19bc0ea2U
#define ADIV5_ACTIVATION_CODE_ARM_SWD_DP  0x1aU

/* ============================================================================
 * Initialization / Deinitialization
 * ============================================================================ */

swd_result_t swd_init(void) {
    if (swd_state.initialized) {
        return SWD_OK;
    }

    /* Check if UART is active on shared pins */
    extern volatile bool programmer_mode_active;
    if (programmer_mode_active) {
        return SWD_ERR_PROTOCOL;
    }

    /* Initialize diagnostic LED */
    diag_led_init();

    if (prog_driver == PROG_DRIVER_BITBANG) {
        /* Bitbang driver: raw GPIO ownership of SWCLK/SWDIO/nRESET.
         * No PIO, no DMA — just configure the pins. */
        swd_bb_gpio_init();
        current_swdio_dir = SWDIO_DIR_FLOAT;
        swd_state.ap_select = 0xFF;
        swd_state.initialized = true;
        return SWD_OK;
    }

    /* Initialize PIO state machine and DMA channels */
    tap_pio_init(swd_state.pio, swd_state.sm);
    dbg_printf("[SWD] PIO init done\r\n");

    /* Set default clock frequency (1 MHz) */
    swd_set_clock(SWD_FREQ_DEFAULT);
    dbg_printf("[SWD] Clock set\r\n");

    current_swdio_dir = SWDIO_DIR_FLOAT;
    /* Force first AP access to write DP_SELECT (DP state is unknown after init) */
    swd_state.ap_select = 0xFF;
    swd_state.initialized = true;

    return SWD_OK;
}

void swd_deinit(void) {
    if (!swd_state.initialized) {
        return;
    }

    pio_sm_set_enabled(swd_state.pio, swd_state.sm, false);
    swd_gpio_deinit();
    swd_state.initialized = false;
    swd_state.ap_select = 0xFF;
    current_swdio_dir = SWDIO_DIR_FLOAT;
}

bool swd_is_initialized(void) {
    return swd_state.initialized;
}

/* ============================================================================
 * Low-Level PIO Operations
 * ============================================================================ */

swd_result_t swd_set_clock(uint32_t freq_khz) {
    if (freq_khz < SWD_FREQ_MIN || freq_khz > SWD_FREQ_MAX) {
        return SWD_ERR_PROTOCOL;
    }

    /* System clock: 125 MHz. Max SWD interface frequency = sys_clk / 8 = 15.625 MHz */
    uint32_t actual_freq = tap_pio_set_sm_freq(swd_state.pio, swd_state.sm, freq_khz * 1000, 15625000);
    swd_state.clock_freq = actual_freq / 1000;

    return SWD_OK;
}

swd_result_t swd_set_turnaround_cycles(uint8_t cycles) {
    if (cycles != 1 && cycles != 2) {
        return SWD_ERR_PROTOCOL;
    }
    swd_state.turnaround_cycles = cycles;
    return SWD_OK;
}

/* ============================================================================
 * Bitbang SWD Transport (default driver)
 * Timing mirrors swd_bitbang_read_idcode(): 5us half-period SWCLK, ACK/data
 * sampled ~2us after the rising edge. ~100 kbit/s, plenty for a debugger.
 * ============================================================================ */

#define SWD_BB_HALF_PERIOD_US  5   /* SWCLK high/low hold */
#define SWD_BB_SAMPLE_US       2   /* post-rising-edge sample delay */

/* Number of full bit periods of SWCLK-low held after the ACK before the
 * write data phase (turnaround). Tuned to the PIO's proven shape (~1-3).
 * Exposed via SWD WRALIGN for host-side alignment sweep. */
uint8_t swd_bb_write_turn_clocks = 1;

void swd_bb_gpio_init(void) {
    gpio_init(SWD_SWCLK_PIN);
    gpio_set_dir(SWD_SWCLK_PIN, GPIO_OUT);
    gpio_set_drive_strength(SWD_SWCLK_PIN, GPIO_DRIVE_STRENGTH_12MA);
    gpio_set_slew_rate(SWD_SWCLK_PIN, GPIO_SLEW_RATE_FAST);
    gpio_put(SWD_SWCLK_PIN, 0);

    gpio_init(SWD_SWDIO_PIN);
    gpio_set_dir(SWD_SWDIO_PIN, GPIO_IN);
    gpio_pull_up(SWD_SWDIO_PIN);
    gpio_set_drive_strength(SWD_SWDIO_PIN, GPIO_DRIVE_STRENGTH_12MA);
    gpio_set_slew_rate(SWD_SWDIO_PIN, GPIO_SLEW_RATE_FAST);

    gpio_init(SWD_nRESET_PIN);
    gpio_set_dir(SWD_nRESET_PIN, GPIO_IN);
}

static void swd_bb_out_bits(uint32_t bit_count, uint32_t data) {
    if (bit_count == 0 || bit_count > 32) return;

    /* Drive SWDIO (turnaround entry) if it is currently released. */
    if (current_swdio_dir == SWDIO_DIR_FLOAT) {
        gpio_set_dir(SWD_SWDIO_PIN, GPIO_OUT);
        current_swdio_dir = SWDIO_DIR_DRIVE;
    }

    for (uint32_t i = 0; i < bit_count; i++) {
        gpio_put(SWD_SWDIO_PIN, (data >> i) & 1);
        gpio_put(SWD_SWCLK_PIN, 1);
        sleep_us(SWD_BB_HALF_PERIOD_US);
        gpio_put(SWD_SWCLK_PIN, 0);
        sleep_us(SWD_BB_HALF_PERIOD_US);
    }
}

static uint32_t swd_bb_in_bits(uint32_t bit_count) {
    if (bit_count == 0 || bit_count > 32) return 0;

    /* Release SWDIO (turnaround) if currently driven. */
    if (current_swdio_dir == SWDIO_DIR_DRIVE) {
        gpio_set_dir(SWD_SWDIO_PIN, GPIO_IN);
        current_swdio_dir = SWDIO_DIR_FLOAT;
    }

    uint32_t result = 0;
    for (uint32_t i = 0; i < bit_count; i++) {
        gpio_put(SWD_SWCLK_PIN, 1);
        sleep_us(SWD_BB_SAMPLE_US);
        result |= (uint32_t)(gpio_get(SWD_SWDIO_PIN) ? 1 : 0) << i;
        sleep_us(SWD_BB_HALF_PERIOD_US - SWD_BB_SAMPLE_US);
        gpio_put(SWD_SWCLK_PIN, 0);
        sleep_us(SWD_BB_HALF_PERIOD_US);
    }
    return result;
}

void swd_bb_line_reset_ext(bool idle_cycles) {
    /* Same shape as swd_line_reset_ext(): 60 HIGH cycles, optionally +4 LOW
     * idle cycles (64 total). */
    swd_bb_out_bits(32, 0xFFFFFFFF);
    if (idle_cycles) {
        swd_bb_out_bits(32, 0x0FFFFFFF);  /* 28 HIGH + 4 LOW idle */
    } else {
        swd_bb_out_bits(28, 0x0FFFFFFF);
    }
    swd_state.ap_select = 0xFF;
}

void swd_bb_jtag_to_swd(void) {
    /* Start from a known released state. */
    current_swdio_dir = SWDIO_DIR_FLOAT;

    /* 1. Line reset: 56 HIGH cycles */
    swd_bb_out_bits(32, 0xFFFFFFFF);
    swd_bb_out_bits(24, 0x00FFFFFF);

    /* 2. JTAG-to-SWD switch code 0xE79E (16 bits, LSB-first) */
    swd_bb_out_bits(16, 0xE79E);

    /* 3. Post-switch line reset: 60 HIGH cycles */
    swd_bb_out_bits(32, 0xFFFFFFFF);
    swd_bb_out_bits(28, 0x0FFFFFFF);

    /* 4. Idle cycles: 8 LOW bits */
    swd_bb_out_bits(8, 0x00);

    swd_state.ap_select = 0xFF;
}

uint8_t swd_bb_transfer(uint8_t request, uint32_t *data) {
    bool is_read = (request & (1 << SWD_REQUEST_RnW)) != 0;

    /* 1. Request phase: 8 bits out, LSB-first. Ends with SWCLK LOW. */
    swd_debug_bb_request = request;
    swd_bb_out_bits(8, request);

    /* 2. Turnaround: release SWDIO during the LOW phase of SWCLK.
     * NO extra clock period — the ACK is sampled on the very next rising
     * edge (matches the bitbanging reference driver and the PIO
     * seq_in_turnaround sub-program: `set pindirs, 0 side 0 [3]` +
     * `nop side 0 [3]` hold SWCLK low, then seq_in clocks it). */
    gpio_set_dir(SWD_SWDIO_PIN, GPIO_IN);
    current_swdio_dir = SWDIO_DIR_FLOAT;
    sleep_us(SWD_BB_HALF_PERIOD_US);  /* hold the low phase for the turnaround */

    /* 3. ACK: 3 bits in, sampled mid-high-phase (2us after rising edge). */
    uint8_t ack = (uint8_t)(swd_bb_in_bits(3) & 0x07);
    swd_debug_raw_ack = ack;
    swd_debug_bb_ack_bits = ack;

    if (ack != SWD_ACK_OK) {
        swd_debug_bb_last_err = 1;
        /* No data phase on WAIT/FAULT/protocol error, but DO return the line
         * to a clean driven-low idle state — otherwise the target stays
         * mid-transaction and ignores the next request (silent all-ones). */
        gpio_set_dir(SWD_SWDIO_PIN, GPIO_OUT);
        current_swdio_dir = SWDIO_DIR_DRIVE;
        gpio_put(SWD_SWDIO_PIN, 0);
        for (int i = 0; i < 8; i++) {
            gpio_put(SWD_SWCLK_PIN, 1);
            sleep_us(SWD_BB_HALF_PERIOD_US);
            gpio_put(SWD_SWCLK_PIN, 0);
            sleep_us(SWD_BB_HALF_PERIOD_US);
        }
        return ack;
    }

    if (is_read) {
        /* 4a. Read 32 data bits + 1 parity. */
        uint32_t read_val = swd_bb_in_bits(32);
        uint8_t read_parity = (uint8_t)(swd_bb_in_bits(1) & 1);
        uint8_t calc_parity = __builtin_popcount(read_val) & 1;
        swd_debug_bb_rpar = read_parity;
        swd_debug_bb_cpar = calc_parity;
        if (read_parity != calc_parity) {
            swd_debug_bb_last_err = 2;
            current_swdio_dir = SWDIO_DIR_FLOAT;
            return 0x07;  /* parity error */
        }
        if (data) *data = read_val;
    } else {
        /* 4b. Write 32 data bits + 1 parity, then idle. */
        uint32_t val = data ? *data : 0;
        uint8_t parity = __builtin_popcount(val) & 1;
        swd_debug_bb_wack = ack;

        /* Turnaround after ACK: the bus direction changes (target -> host).
         * Drive SWDIO and hold SWCLK LOW for swd_bb_write_turn_clocks full
         * bit periods with NO rising edges — matching the PIO's proven
         * seq_out_turnaround (2 instructions of SWCLK-low ≈ 1 bit period)
         * plus the seq_out head. The target samples write data on the first
         * rising edge after this quiet low stretch; without enough low time
         * the data phase lands shifted and the target sees a parity error
         * (STICKYERR -> subsequent transactions ACK FAULT). */
        gpio_set_dir(SWD_SWDIO_PIN, GPIO_OUT);
        current_swdio_dir = SWDIO_DIR_DRIVE;
        gpio_put(SWD_SWDIO_PIN, 0);
        for (uint8_t tc = 0; tc < swd_bb_write_turn_clocks; tc++) {
            sleep_us(SWD_BB_HALF_PERIOD_US * 2);
        }

        swd_bb_out_bits(32, val);
        swd_bb_out_bits(1, parity);
    }

    /* 5. Drive SWDIO and send 8 idle cycles (LOW). */
    gpio_set_dir(SWD_SWDIO_PIN, GPIO_OUT);
    current_swdio_dir = SWDIO_DIR_DRIVE;
    gpio_put(SWD_SWDIO_PIN, 0);
    for (int i = 0; i < 8; i++) {
        gpio_put(SWD_SWCLK_PIN, 1);
        sleep_us(SWD_BB_HALF_PERIOD_US);
        gpio_put(SWD_SWCLK_PIN, 0);
        sleep_us(SWD_BB_HALF_PERIOD_US);
    }

    return ack;
}

void swd_write_bits(uint32_t bit_count, uint32_t data) {
    if (prog_driver == PROG_DRIVER_BITBANG) {
        swd_bb_out_bits(bit_count, data);
        return;
    }
    if (bit_count == 0 || bit_count > 32) {
        return;
    }

    uint32_t pio_buf[6];
    uint8_t pos = 0;

    if (current_swdio_dir == SWDIO_DIR_FLOAT) {
        pio_buf[pos++] = SWD_ORIGIN_SEQ_OUT_TURNAROUND;
        current_swdio_dir = SWDIO_DIR_DRIVE;
    } else {
        pio_buf[pos++] = SWD_ORIGIN_SEQ_OUT;
    }

    pio_buf[pos++] = bit_count - 1;
    if (bit_count < 32) {
        pio_buf[pos++] = data & ((1UL << bit_count) - 1);
    } else {
        pio_buf[pos++] = data;
        /* When bit_count == 32, autopull (threshold=32) consumes the next
         * FIFO word after the 32-bit loop. Add a filler so pull block at
         * the end of seq_out can read the actual next origin. */
        pio_buf[pos++] = 0;
    }
    pio_buf[pos++] = SWD_ORIGIN_START;
    /* Trailing dummy: out pc,32 at end of each sub-program triggers autopull
     * which consumes one extra word. Supply a dummy so the TX FIFO is not
     * empty when autopull fires at the last out pc,32. */
    pio_buf[pos++] = 0;

    diag_led_on();
    tap_pio_dma_send_uint32(swd_state.pio, swd_state.sm, pio_buf, pos);
    diag_led_off();
    tap_pio_wait_for_tx_stall(swd_state.pio, swd_state.sm);
    pio_sm_clear_fifos(swd_state.pio, swd_state.sm);
}

uint32_t swd_read_bits(uint32_t bit_count) {
    if (prog_driver == PROG_DRIVER_BITBANG) {
        return swd_bb_in_bits(bit_count);
    }
    if (bit_count == 0 || bit_count > 32) {
        return 0;
    }

    uint32_t pio_buf[5];
    uint8_t pos = 0;

    if (current_swdio_dir == SWDIO_DIR_DRIVE) {
        pio_buf[pos++] = SWD_ORIGIN_SEQ_IN_TURNAROUND;
        current_swdio_dir = SWDIO_DIR_FLOAT;
    } else {
        pio_buf[pos++] = SWD_ORIGIN_SEQ_IN;
    }

    pio_buf[pos++] = bit_count - 1;
    pio_buf[pos++] = SWD_ORIGIN_START;
    pio_buf[pos++] = 0;  /* trailing dummy for autopull */

    diag_led_on();
    tap_pio_dma_send_uint32(swd_state.pio, swd_state.sm, pio_buf, pos);
    diag_led_off();
    uint32_t raw_val;
    if (!tap_pio_get_blocking_timeout(swd_state.pio, swd_state.sm, &raw_val)) {
        tap_pio_wait_for_tx_stall(swd_state.pio, swd_state.sm);
        pio_sm_clear_fifos(swd_state.pio, swd_state.sm);
        return 0;
    }
    uint32_t result = raw_val >> (32U - bit_count);

    tap_pio_wait_for_tx_stall(swd_state.pio, swd_state.sm);
    pio_sm_clear_fifos(swd_state.pio, swd_state.sm);

    return result;
}

void swd_hiz_clocks(uint32_t bit_count) {
    if (bit_count == 0) return;
    swd_read_bits(bit_count);
}

/* ============================================================================
 * SWD Protocol Operations
 * ============================================================================ */

void swd_line_reset_ext(bool idle_cycles) {
    if (prog_driver == PROG_DRIVER_BITBANG) {
        swd_bb_line_reset_ext(idle_cycles);
        return;
    }
    /* Same self-contained reset as swd_jtag_to_swd(): always drive SWDIO from
     * a known state instead of trusting a potentially stale direction cache. */
    current_swdio_dir = SWDIO_DIR_FLOAT;

    uint32_t pio_buf[8];
    uint8_t pos = 0;

    if (current_swdio_dir == SWDIO_DIR_FLOAT) {
        pio_buf[pos++] = SWD_ORIGIN_SEQ_OUT_TURNAROUND;
        current_swdio_dir = SWDIO_DIR_DRIVE;
    } else {
        pio_buf[pos++] = SWD_ORIGIN_SEQ_OUT;
    }

    uint8_t total_bits = idle_cycles ? 64 : 60;
    pio_buf[pos++] = total_bits - 1;
    pio_buf[pos++] = 0xFFFFFFFF;       /* 32 HIGH bits */
    
    if (idle_cycles) {
        /* 28 HIGH bits (bits 0..27) + 4 LOW idle bits (bits 28..31) */
        pio_buf[pos++] = 0x0FFFFFFF;
    } else {
        /* 28 HIGH bits */
        pio_buf[pos++] = 0x0FFFFFFF;
    }

    pio_buf[pos++] = SWD_ORIGIN_START;
    pio_buf[pos++] = 0;

    tap_pio_dma_send_uint32(swd_state.pio, swd_state.sm, pio_buf, pos);
    tap_pio_wait_for_tx_stall(swd_state.pio, swd_state.sm);
    pio_sm_clear_fifos(swd_state.pio, swd_state.sm);
    /* Line reset clears DP_SELECT in the target DP — invalidate our cache */
    swd_state.ap_select = 0xFF;
}

void swd_line_reset(void) {
    swd_line_reset_ext(true);
}

void swd_jtag_to_swd(void) {
    if (prog_driver == PROG_DRIVER_BITBANG) {
        swd_bb_jtag_to_swd();
        return;
    }
    /* Start from a known FLOAT state so the first swd_write_bits() uses
     * SEQ_OUT_TURNAROUND and actually drives SWDIO. current_swdio_dir can be
     * stale relative to the real pin state (e.g. after the SWD CAPTURE handler
     * reconfigures pins, or after a previous op left it DRIVE); relying on it
     * here previously left SWDIO undriven (floating high via pull-up) so the
     * JTAG-to-SWD switch bits never reached the target. */
    current_swdio_dir = SWDIO_DIR_FLOAT;

    /* 1. Line reset (56 HIGH cycles) */
    swd_write_bits(32, 0xFFFFFFFF);
    swd_write_bits(24, 0x00FFFFFF);

    /* 2. JTAG-to-SWD switch code 0xE79E (16 bits, LSB-first) */
    swd_write_bits(16, 0xE79E);

    /* 3. Post-switch line reset (60 HIGH cycles) */
    swd_write_bits(32, 0xFFFFFFFF);
    swd_write_bits(28, 0x0FFFFFFF);

    /* 4. Idle cycles (8 LOW bits) */
    swd_write_bits(8, 0x00);

    swd_state.ap_select = 0xFF;
}

void swd_dormant_to_swd(void) {
    /* 1. Initial line reset (no idle cycles before transition) */
    swd_line_reset_ext(false);

    /* 2. Send 44-bit JTAG-to-Dormant transition sequence (BMP adiv5_swd.c L86-88)
     * Required for targets powering up in JTAG mode (e.g. STM32F1 SWJ-DP) */
    swd_write_bits(5, 0x1F);
    swd_write_bits(31, 0x33BBBBBA);
    swd_write_bits(8, 0xFF);

    /* 3. Send 128-bit Selection Alert sequence (LSB-first word order) */
    uint32_t alert_buf[4] = {
        ADIV5_SELECTION_ALERT_SEQUENCE_3,  /* 0x19bc0ea2U */
        ADIV5_SELECTION_ALERT_SEQUENCE_2,  /* 0xe3ddafe9U */
        ADIV5_SELECTION_ALERT_SEQUENCE_1,  /* 0x86852d95U */
        ADIV5_SELECTION_ALERT_SEQUENCE_0   /* 0x6209f392U */
    };
    for (int i = 0; i < 4; i++) {
        swd_write_bits(32, alert_buf[i]);
    }

    /* 4. Send 4 LOW bits + 8-bit ARM SWD activation code (0x1A) */
    swd_write_bits(12, (ADIV5_ACTIVATION_CODE_ARM_SWD_DP << 4));

    /* 5. Line reset to bring interface to reset state */
    swd_line_reset_ext(true);
}

swd_result_t swd_write_dp_no_check(uint8_t reg, uint32_t value) {
    uint8_t request = swd_build_request(0, 0, (reg >> 2) & 3);

    if (prog_driver == PROG_DRIVER_BITBANG) {
        /* Bitbang write: full transaction via swd_bb_transfer(). The PIO path
         * below is only valid when the PIO engine is actually running — in
         * bitbang mode those raw FIFO writes would go to a dead engine and
         * silently corrupt the line state (this previously broke every
         * transaction after swd_read_idcode's ABORT/CTRLSTAT writes). */
        uint32_t val = value;
        uint8_t ack = swd_bb_transfer(request, &val);
        return (ack == SWD_ACK_OK) ? SWD_OK : SWD_ERR_ACK_WAIT;
    }

    uint32_t parity = __builtin_popcount(value) & 1;
    uint32_t pio_buf[16];
    uint8_t pos = 0;

    /* 1. Request phase (8 bits out) */
    if (current_swdio_dir == SWDIO_DIR_FLOAT) {
        pio_buf[pos++] = SWD_ORIGIN_SEQ_OUT_TURNAROUND;
        current_swdio_dir = SWDIO_DIR_DRIVE;
    } else {
        pio_buf[pos++] = SWD_ORIGIN_SEQ_OUT;
    }
    pio_buf[pos++] = 8 - 1;
    pio_buf[pos++] = request;

    /* 2. Turnaround to input + Read ACK (3 bits in) */
    pio_buf[pos++] = SWD_ORIGIN_SEQ_IN_TURNAROUND;
    current_swdio_dir = SWDIO_DIR_FLOAT;
    pio_buf[pos++] = 3 - 1;

    /* 3. Turnaround to output + Write Data (32 bits) + Parity (1 bit) + Idle (8 bits) */
    pio_buf[pos++] = SWD_ORIGIN_SEQ_OUT_TURNAROUND;
    current_swdio_dir = SWDIO_DIR_DRIVE;
    pio_buf[pos++] = 32 + 1 + 8 - 1;  /* 41 bits total */
    pio_buf[pos++] = value;
    pio_buf[pos++] = parity;          /* parity bit 0, rest 0 = idle */

    /* 4. Return to start (origin 0) */
    pio_buf[pos++] = SWD_ORIGIN_START;
    pio_buf[pos++] = 0;

    tap_pio_dma_send_uint32(swd_state.pio, swd_state.sm, pio_buf, pos);

    uint32_t raw_ack = 0;
    tap_pio_get_blocking_timeout(swd_state.pio, swd_state.sm, &raw_ack);
    tap_pio_wait_for_tx_stall(swd_state.pio, swd_state.sm);
    pio_sm_clear_fifos(swd_state.pio, swd_state.sm);

    uint8_t ack = (raw_ack >> 29) & 0x07;
    return (ack == SWD_ACK_OK) ? SWD_OK : SWD_ERR_ACK_WAIT;
}

swd_result_t swd_read_dpidr_retry(uint32_t *idcode) {
    swd_result_t res = swd_read_dp(DP_DPIDR, idcode);
    uint32_t freqs[] = { 100, 200, 500, 1000 };
    int freq_idx = 0;

    for (int retry = 0; retry < 30 && (res == SWD_ERR_ACK_WAIT || res == SWD_ERR_PROTOCOL); retry++) {
        /* Every 5 retries on WAIT, step down SWD clock frequency for target bus sync */
        if (retry > 0 && (retry % 5 == 0) && freq_idx < 3) {
            freq_idx++;
            swd_set_clock(freqs[freq_idx]);
        }
        /* Write ABORT register (0x0) to clear WAIT / STICKYERR flags (ARM ADIv5 §A3.2) */
        swd_write_dp_no_check(DP_ABORT, 0x0000001E);
        /* Request DBG and SYS power-up via CTRL/STAT (0x50000000 = CDBGPWRUPREQ | CSYSPWRUPREQ) */
        swd_write_dp_no_check(DP_CTRLSTAT, 0x50000000);
        sleep_ms(2);
        res = swd_read_dp(DP_DPIDR, idcode);
    }
    return res;
}

swd_result_t swd_read_idcode(uint32_t *idcode) {
    if (!idcode) {
        return SWD_ERR_PROTOCOL;
    }

    /* Reset SWDIO direction to a known state before starting */
    current_swdio_dir = SWDIO_DIR_FLOAT;

    swd_result_t result;

    if (prog_driver == PROG_DRIVER_BITBANG) {
        /* Bitbang fast connect: line reset -> JTAG-to-SWD switch -> DPIDR read
         * FIRST (the ADIv5 SW-DP ignores non-DPIDR accesses in its post-reset
         * state) -> ABORT/CTRLSTAT writes only after the read succeeds.
         * No 30-retry loop: the bitbang driver is deterministic, so IDCODE
         * reads complete in milliseconds. */
        swd_jtag_to_swd();
        result = swd_read_dp(DP_DPIDR, idcode);
        if (result == SWD_OK && *idcode != 0 && *idcode != 0xFFFFFFFF) {
            swd_write_dp_no_check(DP_ABORT, 0x0000001E);
            swd_write_dp_no_check(DP_CTRLSTAT, 0x50000000);
            swd_state.last_idcode = *idcode;
            return SWD_OK;
        }
        /* No target at normal levels: pulse nRESET and retry once. */
        swd_assert_reset();
        sleep_ms(10);
        swd_deassert_reset();
        sleep_ms(100);
        swd_jtag_to_swd();
        result = swd_read_dp(DP_DPIDR, idcode);
        if (result == SWD_OK && *idcode != 0 && *idcode != 0xFFFFFFFF) {
            swd_write_dp_no_check(DP_ABORT, 0x0000001E);
            swd_write_dp_no_check(DP_CTRLSTAT, 0x50000000);
            swd_state.last_idcode = *idcode;
            return SWD_OK;
        }
        return result;
    }

    /* 1. Send JTAG-to-SWD switch first (required for STM32F103 SWJ-DP targets).
     * IMPORTANT: the first SWD transaction after the switch MUST be a DPIDR read.
     * Per ADIv5, the SW-DP ignores all non-DPIDR accesses while in its post-reset
     * state (they go unanswered with a silent all-ones ACK), so ABORT/CTRLSTAT
     * writes issued before the first read are never acknowledged. BMP and
     * debugprobe/CMSIS-DAP both read DPIDR first. swd_read_dpidr_retry() already
     * performs the read as its first action; only after it succeeds do we issue
     * the ABORT (clear sticky errors) and CTRLSTAT (power-up request) writes. */
    swd_jtag_to_swd();
    result = swd_read_dpidr_retry(idcode);
    if (result == SWD_OK && *idcode != 0 && *idcode != 0xFFFFFFFF) {
        swd_write_dp_no_check(DP_ABORT, 0x0000001E);
        swd_write_dp_no_check(DP_CTRLSTAT, 0x50000000);
        swd_state.last_idcode = *idcode;
        return SWD_OK;
    }

    /* 3. If no response, pulse nRESET and wait 100ms for RC filter capacitor to charge */
    swd_assert_reset();
    sleep_ms(10);
    swd_deassert_reset();
    sleep_ms(100);

    swd_line_reset_ext(true);
    result = swd_read_dpidr_retry(idcode);
    if (result == SWD_OK && *idcode != 0 && *idcode != 0xFFFFFFFF) {
        swd_write_dp_no_check(DP_ABORT, 0x0000001E);
        swd_write_dp_no_check(DP_CTRLSTAT, 0x50000000);
        swd_state.last_idcode = *idcode;
        return SWD_OK;
    }

    /* 4. Try Dormant-to-SWD wake (for WCH CH32V203 and ADIv5.2+ dormant targets) */
    swd_dormant_to_swd();
    result = swd_read_dpidr_retry(idcode);
    /* 5. Fallback to direct bitbanging SWD driver */
    uint32_t bb_res = swd_bitbang_read_idcode();
    uint8_t ack = (bb_res >> 28) & 0x0F;
    if (ack == 1) {
        *idcode = (bb_res & 0x0FFFFFFF) | 0x10000000;
        swd_state.last_idcode = *idcode;
        return SWD_OK;
    }

    return result;
}

swd_result_t swd_write_dp(uint8_t reg, uint32_t value) {
    uint8_t request = swd_build_request(0, 0, (reg >> 2) & 3);  /* A[2:3] from bits [3:2] */
    uint8_t ack = swd_transfer(request, &value);

    if (ack == SWD_ACK_OK) return SWD_OK;
    if (ack == SWD_ACK_WAIT) return SWD_ERR_ACK_WAIT;
    if (ack == SWD_ACK_FAULT) return SWD_ERR_ACK_FAULT;
    return SWD_ERR_PROTOCOL;
}

swd_result_t swd_read_dp(uint8_t reg, uint32_t *value) {
    if (!value) return SWD_ERR_PROTOCOL;

    uint8_t request = swd_build_request(0, 1, (reg >> 2) & 3);  /* A[2:3] from bits [3:2] */
    uint8_t ack = swd_transfer(request, value);

    if (ack == SWD_ACK_OK) return SWD_OK;
    if (ack == SWD_ACK_WAIT) return SWD_ERR_ACK_WAIT;
    if (ack == SWD_ACK_FAULT) return SWD_ERR_ACK_FAULT;
    return SWD_ERR_PROTOCOL;
}

swd_result_t swd_write_ap(uint8_t reg, uint32_t value) {
    uint32_t select = reg & 0xF0;  /* APBANKSEL from upper nibble */
    swd_result_t res;

    /* Only write DP_SELECT if the AP bank actually changed (task 2.2) */
    if (swd_state.ap_select != select) {
        res = swd_write_dp(DP_SELECT, select);
        if (res != SWD_OK) return res;
        swd_state.ap_select = select;
    }

    uint8_t request = swd_build_request(1, 0, (reg >> 2) & 3);  /* A[2:3] from bits [3:2] */
    uint8_t ack = swd_transfer(request, &value);

    if (ack == SWD_ACK_OK) {
        /* Dummy DP read acts as a completion barrier (ADIv5 §5.3.3) —
         * ensures the AP write has fully completed before we return. */
        uint32_t dummy = 0;
        swd_read_dp(DP_RDBUFF, &dummy);
        return SWD_OK;
    }
    if (ack == SWD_ACK_WAIT) return SWD_ERR_ACK_WAIT;
    if (ack == SWD_ACK_FAULT) return SWD_ERR_ACK_FAULT;
    return SWD_ERR_PROTOCOL;
}

swd_result_t swd_read_ap(uint8_t reg, uint32_t *value) {
    if (!value) return SWD_ERR_PROTOCOL;

    uint32_t select = reg & 0xF0;  /* APBANKSEL from upper nibble */
    swd_result_t res;

    /* Only write DP_SELECT if the AP bank actually changed (task 2.2) */
    if (swd_state.ap_select != select) {
        res = swd_write_dp(DP_SELECT, select);
        if (res != SWD_OK) return res;
        swd_state.ap_select = select;
    }

    uint8_t request = swd_build_request(1, 1, (reg >> 2) & 3);  /* A[2:3] from bits [3:2] */
    uint32_t ap_val = 0;
    uint8_t ack = swd_transfer(request, &ap_val);

    if (ack == SWD_ACK_OK) {
        /* AP reads are posted: the data returned in the request transaction is
         * the read buffer from a *previous* AP access. The real value must be
         * fetched via DP_RDBUFF (ADIv5 §5.3.2). */
        uint32_t rd_buff = 0;
        res = swd_read_dp(DP_RDBUFF, &rd_buff);
        if (res != SWD_OK) return res;
        *value = rd_buff;
        return SWD_OK;
    }
    if (ack == SWD_ACK_WAIT) return SWD_ERR_ACK_WAIT;
    if (ack == SWD_ACK_FAULT) return SWD_ERR_ACK_FAULT;
    return SWD_ERR_PROTOCOL;
}

uint8_t swd_transfer(uint8_t request, uint32_t *data) {
    if (prog_driver == PROG_DRIVER_BITBANG) {
        return swd_bb_transfer(request, data);
    }
    bool is_read = (request & (1 << SWD_REQUEST_RnW)) != 0;
    uint32_t pio_buf[16];
    uint8_t pos = 0;

    /*
     * FIFO buffer layout — matches MioLink's swdtap_adiv5_prepare_pio_seq().
     *
     * Each PIO sub-program reads [origin] from TX FIFO first (via `out pc, 32`
     * at `start`, or via `pull block` + `out pc, 32` at end of previous program),
     * then reads [bit_count] via `out x, 32`, then reads [data...] words.
     * With autopull ON (threshold 32), after the data loop the OSR is empty.
     * `pull block` loads the NEXT origin via autopull, `out pc, 32` jumps there.
     */

    /* 1. Request phase (8 bits out) */
    if (current_swdio_dir == SWDIO_DIR_FLOAT) {
        pio_buf[pos++] = SWD_ORIGIN_SEQ_OUT_TURNAROUND;  /* origin 1 */
        current_swdio_dir = SWDIO_DIR_DRIVE;
    } else {
        pio_buf[pos++] = SWD_ORIGIN_SEQ_OUT;  /* origin 2 */
    }
    pio_buf[pos++] = 8 - 1;    /* bit count for request */
    pio_buf[pos++] = request;  /* 8-bit request data */

    /* 2. Turnaround to input + Read ACK (3 bits in) */
    pio_buf[pos++] = SWD_ORIGIN_SEQ_IN_TURNAROUND;  /* origin 7 */
    current_swdio_dir = SWDIO_DIR_FLOAT;
    pio_buf[pos++] = 3 - 1;  /* bit count for ACK */

    /* 3. Check ACK via PIO hardware (origin 14).
     * FIFO entries: [origin][expected_ACK][discard_count]
     * If ACK matches: reads expected_ACK and discard_count, then falls through.
     * If ACK mismatch: reads discard_count words from TX FIFO and jumps to
     * turnaround_float_to_drive (skipping data phase). */
    pio_buf[pos++] = SWD_ORIGIN_CHECK_ACK;  /* origin 14 */
    pio_buf[pos++] = (SWD_ACK_OK << 29);    /* expected ACK value */
    if (is_read) {
        pio_buf[pos++] = 7 - 1;  /* words to discard on mismatch: seq_in origin + bit_count + seq_out_turnaround origin + idle count + idle data + park origin + trailing dummy */
    } else {
        pio_buf[pos++] = 6 - 1;  /* words to discard on mismatch: seq_out_turnaround origin + bit_count + data + parity + park origin + trailing dummy */
    }

    if (is_read) {
        /* 4. Read data+parity (33 bits) via seq_in.
         * Bit count = 32: loop runs 33 times. Autopush threshold=32 auto-pushes
         * 32 data bits to RX FIFO at iteration 32. The 33rd iteration shifts in
         * parity, explicit push sends it as a second RX FIFO entry. */
        pio_buf[pos++] = SWD_ORIGIN_SEQ_IN;  /* origin 8 */
        pio_buf[pos++] = 32 + 1 - 1;  /* 32 data + 1 parity = 33 bits */

        /* 5. Turnaround to output + idle cycles */
        pio_buf[pos++] = SWD_ORIGIN_SEQ_OUT_TURNAROUND;  /* origin 1 */
        current_swdio_dir = SWDIO_DIR_DRIVE;
        pio_buf[pos++] = swd_state.idle_cycles - 1;  /* bit count for idle */
        pio_buf[pos++] = 0;  /* idle data: all zeros */

        /* 6. Return to start (origin 0) to park SM */
        pio_buf[pos++] = SWD_ORIGIN_START;
        pio_buf[pos++] = 0;  /* trailing dummy for autopull at out pc,32 */
    } else {
        /* 4. Write data (32 bits) + parity (1 bit) = 33 bits via seq_out */
        uint32_t val = data ? *data : 0;
        uint32_t parity = __builtin_popcount(val) & 1;

        pio_buf[pos++] = SWD_ORIGIN_SEQ_OUT_TURNAROUND;  /* origin 1 */
        current_swdio_dir = SWDIO_DIR_DRIVE;
        pio_buf[pos++] = 32 + 1 + swd_state.idle_cycles - 1;  /* 32 data + 1 parity + idle */
        pio_buf[pos++] = val;      /* 32-bit data word */
        pio_buf[pos++] = parity;   /* parity in bit[0], rest zeros = idle */

        /* 5. Return to start (origin 0) to park SM */
        pio_buf[pos++] = SWD_ORIGIN_START;
        pio_buf[pos++] = 0;  /* trailing dummy for autopull at out pc,32 */
    }

    diag_led_on();
    tap_pio_dma_send_uint32(swd_state.pio, swd_state.sm, pio_buf, pos);
    diag_led_off();

    /* Read ACK response from RX FIFO (from step 2: 3-bit seq_in push) */
    uint32_t raw_ack;
    if (!tap_pio_get_blocking_timeout(swd_state.pio, swd_state.sm, &raw_ack)) {
        tap_pio_wait_for_tx_stall(swd_state.pio, swd_state.sm);
        pio_sm_clear_fifos(swd_state.pio, swd_state.sm);
        return 0x07;  /* Timeout = parity error code */
    }
    uint8_t ack = (raw_ack >> 29) & 0x07;
    swd_debug_raw_ack = raw_ack;

    if (ack == SWD_ACK_OK && is_read && data) {
        /* Read 32-bit data from RX FIFO (seq_in push after 32-bit loop) */
        uint32_t raw_data;
        if (!tap_pio_get_blocking_timeout(swd_state.pio, swd_state.sm, &raw_data)) {
            tap_pio_wait_for_tx_stall(swd_state.pio, swd_state.sm);
            pio_sm_clear_fifos(swd_state.pio, swd_state.sm);
            return 0x07;
        }
        /* Read parity from RX FIFO (seq_in's 33rd in pins + push) */
        uint32_t raw_parity;
        if (!tap_pio_get_blocking_timeout(swd_state.pio, swd_state.sm, &raw_parity)) {
            tap_pio_wait_for_tx_stall(swd_state.pio, swd_state.sm);
            pio_sm_clear_fifos(swd_state.pio, swd_state.sm);
            return 0x07;
        }

        uint32_t read_val = raw_data >> (32U - 32);  /* data in bits[31:0] after 32 shifts */
        uint32_t read_parity = (raw_parity != 0) ? 1 : 0;
        uint32_t calc_parity = __builtin_popcount(read_val) & 1;

        if (read_parity != calc_parity) {
            ack = 0x07; /* Parity error */
        } else {
            *data = read_val;
        }
    }

    if (ack != SWD_ACK_OK) {
        pio_sm_restart(swd_state.pio, swd_state.sm);
        pio_sm_clear_fifos(swd_state.pio, swd_state.sm);
        pio_sm_exec(swd_state.pio, swd_state.sm, pio_encode_jmp(0));
        current_swdio_dir = SWDIO_DIR_FLOAT;
    } else {
        tap_pio_wait_for_tx_stall(swd_state.pio, swd_state.sm);
        pio_sm_clear_fifos(swd_state.pio, swd_state.sm);
    }

    return ack;
}

/* ============================================================================
 * Target Control
 * ============================================================================ */

void swd_assert_reset(void) {
#if defined(SWD_nRESET_PIN)
    gpio_init(SWD_nRESET_PIN);
    gpio_set_dir(SWD_nRESET_PIN, GPIO_OUT);
    gpio_put(SWD_nRESET_PIN, 0);
#endif
}

void swd_deassert_reset(void) {
#if defined(SWD_nRESET_PIN)
    /* Open-drain release: float the pin, let external/internal pull-up raise nRST */
    gpio_init(SWD_nRESET_PIN);
    gpio_set_dir(SWD_nRESET_PIN, GPIO_IN);
#endif
}

bool swd_read_swdo(void) {
    return gpio_get(SWD_SWDIO_PIN);
}

uint32_t swd_bitbang_read_idcode(void) {
    /* 1. Deinit PIO so we have full raw GPIO control */
    if (swd_state.initialized) {
        swd_deinit();
    }

    gpio_init(SWD_SWCLK_PIN);
    gpio_set_dir(SWD_SWCLK_PIN, GPIO_OUT);
    gpio_put(SWD_SWCLK_PIN, 0);

    gpio_init(SWD_SWDIO_PIN);
    gpio_set_dir(SWD_SWDIO_PIN, GPIO_OUT);
    gpio_pull_up(SWD_SWDIO_PIN);
    gpio_put(SWD_SWDIO_PIN, 1);

    /* 2. Line Reset: 60 HIGH cycles */
    for (int i = 0; i < 60; i++) {
        gpio_put(SWD_SWCLK_PIN, 1);
        sleep_us(5);
        gpio_put(SWD_SWCLK_PIN, 0);
        sleep_us(5);
    }

    /* 3. JTAG-to-SWD Select Sequence: 0xE79E (16 bits, LSB-first = 0x79E7) */
    uint16_t seq = 0xE79E;
    for (int i = 0; i < 16; i++) {
        gpio_put(SWD_SWDIO_PIN, (seq >> i) & 1);
        gpio_put(SWD_SWCLK_PIN, 1);
        sleep_us(5);
        gpio_put(SWD_SWCLK_PIN, 0);
        sleep_us(5);
    }

    /* 4. Post-Select Line Reset: 60 HIGH cycles + 4 LOW idle cycles */
    gpio_put(SWD_SWDIO_PIN, 1);
    for (int i = 0; i < 60; i++) {
        gpio_put(SWD_SWCLK_PIN, 1);
        sleep_us(5);
        gpio_put(SWD_SWCLK_PIN, 0);
        sleep_us(5);
    }
    gpio_put(SWD_SWDIO_PIN, 0);
    for (int i = 0; i < 4; i++) {
        gpio_put(SWD_SWCLK_PIN, 1);
        sleep_us(5);
        gpio_put(SWD_SWCLK_PIN, 0);
        sleep_us(5);
    }

    /* 5. Request DPIDR Read (0xA5 = 1 0 1 0 0 1 0 1 LSB-first) */
    uint8_t req = 0xA5;
    for (int i = 0; i < 8; i++) {
        gpio_put(SWD_SWDIO_PIN, (req >> i) & 1);
        gpio_put(SWD_SWCLK_PIN, 1);
        sleep_us(5);
        gpio_put(SWD_SWCLK_PIN, 0);
        sleep_us(5);
    }

    /* 6. Turnaround (0 cycles / immediate input switch on LOW SWCLK) */
    gpio_set_dir(SWD_SWDIO_PIN, GPIO_IN);
    sleep_us(5);

    /* 7. Read ACK (3 bits) sampled on SWCLK RISING EDGE */
    uint8_t ack = 0;
    uint8_t ack_bits[3];
    for (int i = 0; i < 3; i++) {
        gpio_put(SWD_SWCLK_PIN, 1);
        sleep_us(2);
        ack_bits[i] = gpio_get(SWD_SWDIO_PIN);
        ack |= (ack_bits[i] << i);
        sleep_us(3);
        gpio_put(SWD_SWCLK_PIN, 0);
        sleep_us(5);
    }

    /* 8. Read 32-bit IDCODE + 1-bit Parity unconditionally for diagnostics */
    uint32_t idcode = 0;
    uint8_t raw_data[33];
    for (int i = 0; i < 32; i++) {
        gpio_put(SWD_SWCLK_PIN, 1);
        sleep_us(2);
        raw_data[i] = gpio_get(SWD_SWDIO_PIN);
        idcode |= ((uint32_t)raw_data[i] << i);
        sleep_us(3);
        gpio_put(SWD_SWCLK_PIN, 0);
        sleep_us(5);
    }
    /* Parity bit */
    gpio_put(SWD_SWCLK_PIN, 1);
    sleep_us(2);
    raw_data[32] = gpio_get(SWD_SWDIO_PIN);
    sleep_us(3);
    gpio_put(SWD_SWCLK_PIN, 0);
    sleep_us(5);

    /* 9. Turnaround + 8 idle cycles */
    gpio_set_dir(SWD_SWDIO_PIN, GPIO_OUT);
    gpio_put(SWD_SWDIO_PIN, 0);
    for (int i = 0; i < 8; i++) {
        gpio_put(SWD_SWCLK_PIN, 1);
        sleep_us(5);
        gpio_put(SWD_SWCLK_PIN, 0);
        sleep_us(5);
    }

    /* Re-initialize the SWD engine before returning. This driver de-inits
     * the engine at step 1 to gain raw GPIO control; leaving it de-initialized
     * breaks every subsequent transfer. The engine comes back in the currently
     * selected driver mode (bitbang or PIO) — swd_init() is driver-aware. */
    swd_init();

    /* Return encoded result: ACK in top 4 bits, IDCODE in low 28 bits */
    return ((uint32_t)ack << 28) | (idcode & 0x0FFFFFFF);
}

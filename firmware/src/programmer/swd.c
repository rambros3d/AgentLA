/*
 * AgentLA SWD Engine
 * Based on MioLink & Black Magic Debug
 *
 * PIO-based SWD implementation for RP2350 PIO1 with modular origin sub-programs,
 * hardware ACK checking, DMA transfers, and ADIv5 dormant-to-SWD wake sequence.
 */

#include "swd.h"
#include "board_settings.h"
#include "programmer.h"
#include "usb_hid.h"
#include <stdio.h>
#include "debugprobe/probe_pio_transport.h"
#include "debugprobe/swd_transfer_pio.h"
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


/* Global SWD state */
swd_state_t swd_state = {
    .pio = SWD_PIO,
    .sm = SWD_SM,
    .clock_freq = SWD_FREQ_DEFAULT,
    .initialized = false,
    .last_idcode = 0,
    .turnaround_cycles = 1,
    .idle_cycles = 8,  /* MioLink uses 8; SWD spec requires idle cycles after writes */
    .ap_select = 0xFF
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

    /* Release UART mode if active on shared pins */
    extern volatile bool programmer_mode_active;
    if (programmer_mode_active) {
        prog_exit_mode();
    }

    /* Initialize diagnostic LED */
    diag_led_init();

    /* Initialize the vendored debugprobe PIO transport */
    probe_init();
    dbg_printf("[SWD] PIO init done\r\n");

    /* Set default clock frequency (100 kHz — safe for SWJ-DP switching) */
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

    probe_deinit();
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

    probe_set_swclk_freq(freq_khz);
    swd_state.clock_freq = freq_khz;

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

void swd_write_bits(uint32_t bit_count, uint32_t data) {
    if (bit_count == 0 || bit_count > 32) return;
    probe_write_bits(bit_count, data);
}

uint32_t swd_read_bits(uint32_t bit_count) {
    if (bit_count == 0 || bit_count > 32) return 0;
    return probe_read_bits(bit_count);
}

void swd_hiz_clocks(uint32_t bit_count) {
    if (bit_count == 0) return;
    probe_hiz_clocks(bit_count);
}

/* ============================================================================
 * SWD Protocol Operations
 * ============================================================================ */

void swd_line_reset_ext(bool idle_cycles) {
    /* 60 HIGH cycles (+4 LOW idle cycles with idle_cycles): 32 + 28/32 bits.
     * Direction is set in-band by probe_write_bits — no C-side tracking. */
    swd_write_bits(32, 0xFFFFFFFF);
    swd_write_bits(idle_cycles ? 32 : 28, 0x0FFFFFFF);
    /* Line reset clears DP_SELECT in the target DP — invalidate our cache */
    swd_state.ap_select = 0xFF;
}

void swd_line_reset(void) {
    swd_line_reset_ext(true);
}

void swd_jtag_to_swd(void) {
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

static uint8_t swd_pio_transfer_wire(uint8_t request, uint32_t *data);

swd_result_t swd_write_dp_no_check(uint8_t reg, uint32_t value) {
    uint8_t request = swd_build_request(0, 0, (reg >> 2) & 3);
    uint8_t ack = swd_pio_transfer_wire(request, &value);
    if (ack == SWD_ACK_OK) return SWD_OK;
    if (ack == SWD_ACK_WAIT) return SWD_ERR_ACK_WAIT;
    if (ack == SWD_ACK_FAULT) return SWD_ERR_ACK_FAULT;
    return SWD_ERR_PROTOCOL;
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
        /* Request DBG and SYS power-up via CTRL/STAT (0x50000000 = CDBGPWRUPREQ | CSYSPWRUPREQ,
         * the value verified working by pyOCD on this target) */
        swd_write_dp_no_check(DP_CTRLSTAT, 0x50000000);
        sleep_ms(2);
        res = swd_read_dp(DP_DPIDR, idcode);
    }
    return res;
}

static void swd_powerup_poll(void) {
    uint32_t ctrlstat = 0;
    for (int i = 0; i < 100; i++) {
        if (swd_read_dp(DP_CTRLSTAT, &ctrlstat) == SWD_OK) {
            if ((ctrlstat & 0xA0000000) == 0xA0000000) break;
        }
        sleep_us(100);
    }
}

/* Full-link recovery after a target ACK FAULT (or protocol desync).
 * Verified on hardware: after an AP FAULT the SWJ-DP refuses further
 * transactions and a BARE LINE RESET does NOT resync it — only re-sending
 * the JTAG-to-SWD switch sequence does (see design.md "AP FAULT recovery").
 * Mirrors the recovery used in swd_read_idcode(). DP-level only, so this
 * cannot recurse through the AP access hooks that call it. */
static void swd_recover_from_ack_error(void) {
    swd_jtag_to_swd();
    uint32_t idcode = 0;
    if (swd_read_dpidr_retry(&idcode) != SWD_OK) {
        return;  /* still broken — caller reports the original error */
    }
    swd_write_dp_no_check(DP_ABORT, 0x0000001E);
    swd_write_dp_no_check(DP_CTRLSTAT, 0x50000000);
    swd_powerup_poll();
}

swd_result_t swd_read_idcode(uint32_t *idcode) {
    if (!idcode) {
        return SWD_ERR_PROTOCOL;
    }

    current_swdio_dir = SWDIO_DIR_FLOAT;

    /* 1. Send JTAG-to-SWD switch first (required for STM32F103 SWJ-DP targets). */
    swd_jtag_to_swd();
    swd_result_t result = swd_read_dpidr_retry(idcode);
    if (result == SWD_OK && *idcode != 0 && *idcode != 0xFFFFFFFF) {
        swd_write_dp_no_check(DP_ABORT, 0x0000001E);
        swd_write_dp_no_check(DP_CTRLSTAT, 0x50000000);
        swd_powerup_poll();
        swd_state.last_idcode = *idcode;
        return SWD_OK;
    }

    /* 2. Pulse nRESET and retry if initial connection failed */
    swd_assert_reset();
    sleep_ms(10);
    swd_deassert_reset();
    sleep_ms(100);

    swd_line_reset_ext(true);
    result = swd_read_dpidr_retry(idcode);
    if (result == SWD_OK && *idcode != 0 && *idcode != 0xFFFFFFFF) {
        swd_write_dp_no_check(DP_ABORT, 0x0000001E);
        swd_write_dp_no_check(DP_CTRLSTAT, 0x50000000);
        swd_powerup_poll();
        swd_state.last_idcode = *idcode;
        return SWD_OK;
    }

    /* 3. Try Dormant-to-SWD wake (for WCH CH32V203 and ADIv5.2+ dormant targets) */
    swd_dormant_to_swd();
    result = swd_read_dpidr_retry(idcode);
    if (result == SWD_OK && *idcode != 0 && *idcode != 0xFFFFFFFF) {
        swd_write_dp_no_check(DP_ABORT, 0x0000001E);
        swd_write_dp_no_check(DP_CTRLSTAT, 0x50000000);
        swd_powerup_poll();
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
    /* Retry once after a FAULT + link recovery: the target's DP latches the
     * first AP error and keeps returning FAULT until ABORT clears it (the
     * recovery below does that), so the immediate retry normally succeeds. */
    for (int attempt = 0; attempt < 2; attempt++) {
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

        if (ack == SWD_ACK_OK) return SWD_OK;
        if (ack == SWD_ACK_WAIT) return SWD_ERR_ACK_WAIT;
        if (ack == SWD_ACK_FAULT) {
            swd_recover_from_ack_error();
            continue;  /* retry once after recovery */
        }
        swd_recover_from_ack_error();
        return SWD_ERR_PROTOCOL;
    }
    return SWD_ERR_ACK_FAULT;
}

swd_result_t swd_read_ap(uint8_t reg, uint32_t *value) {
    if (!value) return SWD_ERR_PROTOCOL;

    /* ADIv5 AP reads are POSTED: the data phase of an AP read returns the
     * result of the PREVIOUS AP read, never the current one. A single raw
     * AP read therefore always returns stale data. Post the read, then read
     * DP RDBUFF (0x0C, DPBANKSEL=0) — it holds the just-posted result until
     * the next AP read. This is exactly how debugprobe's CMSIS-DAP hosts
     * (pyOCD/OpenOCD) retrieve AP data; we do it in the firmware because our
     * built-in debugger owns the AP access itself. */
    /* Retry once after a FAULT + link recovery (see swd_write_ap). */
    for (int attempt = 0; attempt < 2; attempt++) {
        uint32_t select = reg & 0xF0;  /* APBANKSEL from upper nibble */
        swd_result_t res;

        /* Only write DP_SELECT if the AP bank actually changed (task 2.2).
         * Recovery invalidates the cache (swd_jtag_to_swd sets it to 0xFF),
         * so this also re-writes DP_SELECT after a FAULT retry. */
        if (swd_state.ap_select != select) {
            res = swd_write_dp(DP_SELECT, select);
            if (res != SWD_OK) return res;
            swd_state.ap_select = select;
        }

        uint32_t posted;
        uint8_t request = swd_build_request(1, 1, (reg >> 2) & 3);  /* A[2:3] from bits [3:2] */
        uint8_t ack = swd_transfer(request, &posted);

        if (ack == SWD_ACK_WAIT) return SWD_ERR_ACK_WAIT;
        if (ack == SWD_ACK_FAULT) {
            swd_recover_from_ack_error();
            continue;  /* retry once after recovery */
        }
        if (ack != SWD_ACK_OK) {
            swd_recover_from_ack_error();
            return SWD_ERR_PROTOCOL;
        }

        return swd_read_dp(DP_RDBUFF, value);
    }
    return SWD_ERR_ACK_FAULT;
}

/* Decode our wire request byte (start,APnDP,RnW,A2,A3,parity,stop,park) into
 * the debugprobe SWD_Transfer request layout (bit0=APnDP bit1=RnW bit2=A2
 * bit3=A3) and run the ported transaction engine. Parity-error ACK (0x05) is
 * normalized to the protocol-error code (0x07) used by the bitbang driver. */
static uint8_t swd_pio_transfer_wire(uint8_t request, uint32_t *data) {
    uint32_t dap_req = ((request >> 1) & 0x1)
                     | (((request >> 2) & 0x1) << 1)
                     | (((request >> 3) & 0x1) << 2)
                     | (((request >> 4) & 0x1) << 3);
    swd_pio_conf_t conf = {
        .turnaround = swd_state.turnaround_cycles,
        .idle_cycles = swd_state.idle_cycles,
        /* ADIv5.0 DPs (STM32F1 SWJ-DP) do NOT drive a data phase on
         * WAIT/FAULT — consuming a dummy 33-bit data phase feeds the target's
         * line state machine arbitrary high-Z bits which it can misparse as a
         * new request, desyncing the link. debugprobe leaves this to the host
         * (DAP_SWD_Configure); our firmware hardcoded it to true, which is why
         * an AP FAULT (e.g. a bus-decode error at an unimplemented address)
         * corrupted every subsequent transaction. */
        .data_phase = false,
    };
    uint8_t ack = swd_pio_transfer(&conf, dap_req, data);
    swd_debug_raw_ack = ack;
    return (ack == 0x05) ? 0x07 : ack;
}

uint8_t swd_transfer(uint8_t request, uint32_t *data) {
    return swd_pio_transfer_wire(request, data);
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

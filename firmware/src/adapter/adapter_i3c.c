#include "adapter_i3c.h"
#include "board_settings.h"
#include "pin_owner.h"

#include <stdio.h>
#include <string.h>
#include "pico/stdlib.h"
#include "hardware/clocks.h"
#include "hardware/gpio.h"
#include "hardware/pio.h"
#include "hardware/sync.h"
#include "hardware/regs/io_bank0.h"
#include "hardware/structs/io_bank0.h"

#include "i3c.pio.h"

#ifndef inline
#define inline __inline__ __attribute__((always_inline))
#endif

#define I3C_PIO       I3C_PIO_INSTANCE
#define I3C_SM        I3C_PIO_SM

static uint32_t i3c_wdata_table[512];
static uint8_t i3c_hl_arbcode = 0xfc;
static uint8_t i3c_sda_pin = ADAPTER_I3C_SDA_PIN;
static uint8_t i3c_scl_pin = ADAPTER_I3C_SCL_PIN;
static uint32_t i3c_active_claim_handle = 0;
static bool i3c_initialized = false;
static uint32_t i3c_current_freq_khz = 12500;

static uint32_t i3c_hl_pio_program_sdr[32];
static uint32_t i3c_hl_pio_program_sdr_overlay[32];
static uint32_t i3c_pio_shiftctrl_autopointer_tab[32] = { 0ul };

static inline void __not_in_flash_func(i3c_pio_wait_tx_empty)(void) {
    while ((I3C_PIO->fstat & (1u << (PIO_FSTAT_TXEMPTY_LSB + I3C_SM))) == 0) {
    }
}

static inline void __not_in_flash_func(i3c_pio_wait_idle)(void) {
    while (I3C_PIO->sm[I3C_SM].addr != 0) {
    }
}

static inline void __not_in_flash_func(i3c_pio_put32)(uint32_t data) {
    while ((I3C_PIO->fstat & (1u << (PIO_FSTAT_TXFULL_LSB + I3C_SM))) != 0) {
    }
    I3C_PIO->txf[I3C_SM] = data;
}

static inline void __not_in_flash_func(i3c_pio_put32_no_check)(uint32_t data) {
    I3C_PIO->txf[I3C_SM] = data;
}

static inline uint32_t __not_in_flash_func(i3c_pio_get32)(void) {
    while ((I3C_PIO->fstat & (1u << (PIO_FSTAT_RXEMPTY_LSB + I3C_SM))) != 0) {
    }
    return I3C_PIO->rxf[I3C_SM];
}

static inline void __not_in_flash_func(i3c_pio_set_autopush)(uint8_t bitcount) {
    I3C_PIO->sm[I3C_SM].shiftctrl = i3c_pio_shiftctrl_autopointer_tab[bitcount & 0x1f];
}

static inline void __not_in_flash_func(s_i3c_program_sdr_overlay_sm)(bool tbit_overlay) {
    const void *datasrc;
    i3c_pio_wait_tx_empty();
    i3c_pio_wait_idle();
    datasrc = (const void*)&i3c_hl_pio_program_sdr[i3c_overlay_offset_cmd_restart];
    if (tbit_overlay) {
        datasrc = (const void*)&i3c_hl_pio_program_sdr_overlay[i3c_overlay_offset_cmd_restart];
    }
    memcpy((void*)(&I3C_PIO->instr_mem[i3c_overlay_offset_cmd_restart]),
           datasrc,
           (i3c_overlay_program.length - i3c_overlay_offset_cmd_restart) * sizeof(uint32_t));
}

static inline void __not_in_flash_func(s_i3c_program_hdr_sdr_sm)(void) {
    memcpy((void*)(I3C_PIO->instr_mem), (const void*)i3c_hl_pio_program_sdr, i3c_program.length * sizeof(uint32_t));
    I3C_PIO->sm[I3C_SM].execctrl = (i3c_wrap << PIO_SM0_EXECCTRL_WRAP_TOP_LSB) |
                                   (i3c_wrap_target << PIO_SM0_EXECCTRL_WRAP_BOTTOM_LSB) | 
                                   (1 << PIO_SM0_EXECCTRL_SIDE_EN_LSB) |
                                   (i3c_sda_pin << PIO_SM0_EXECCTRL_JMP_PIN_LSB);
    I3C_PIO->sm[I3C_SM].shiftctrl = (0 << PIO_SM0_SHIFTCTRL_PULL_THRESH_LSB) | 
                                    (1 << PIO_SM0_SHIFTCTRL_OUT_SHIFTDIR_LSB) |
                                    (0 << PIO_SM0_SHIFTCTRL_IN_SHIFTDIR_LSB) |
                                    (0 << PIO_SM0_SHIFTCTRL_PUSH_THRESH_LSB) |
                                    (1 << PIO_SM0_SHIFTCTRL_AUTOPUSH_LSB) |
                                    (0 << PIO_SM0_SHIFTCTRL_AUTOPULL_LSB);
}

const char *adapter_i3c_get_errorstring(i3c_status_t errcode) {
    switch (errcode) {
        case I3C_STATUS_OK: return "OK";
        case I3C_STATUS_IBI: return "IBI received";
        case I3C_STATUS_NAK_DURING_ARBHDR: return "NACK during arbitration header";
        case I3C_STATUS_NAK_DURING_SDRADDR: return "NACK during SDR address phase";
        case I3C_STATUS_PARAM_OUTOFRANGE: return "Parameter out of range";
        case I3C_STATUS_NO_IBI: return "No IBI detected";
        case I3C_STATUS_NAK_DDR: return "NACK during DDR transfer";
        case I3C_STATUS_DDR_EARLY_TERMINATION: return "DDR early termination";
        case I3C_STATUS_DDR_INVALID_PREAMBLE: return "Invalid DDR preamble";
        case I3C_STATUS_DDR_PARITY_WRONG: return "DDR parity error";
        case I3C_STATUS_DDR_CRC_WRONG: return "DDR CRC error";
        case I3C_STATUS_I2C_XFERERROR: return "I2C transfer error";
        case I3C_STATUS_CLAIM_ERROR: return "Pin claim error";
        case I3C_STATUS_NOT_INITIALIZED: return "I3C not initialized";
        default: return "Unknown error";
    }
}

i3c_status_t adapter_i3c_set_clkrate(uint32_t targetfreq_khz) {
    if (targetfreq_khz > 12500u || targetfreq_khz < 40u) {
        return I3C_STATUS_PARAM_OUTOFRANGE;
    }
    float sys_hz = (float)clock_get_hz(clk_sys);
    float bit_hz = (float)targetfreq_khz * 1000.0f;
    /* In i3c.pio, each bit takes 10 PIO instructions */
    float pio_target_hz = bit_hz * 10.0f;
    float div = sys_hz / pio_target_hz;
    if (div < 1.0f) div = 1.0f;
    pio_sm_set_clkdiv(I3C_PIO, I3C_SM, div);
    i3c_current_freq_khz = targetfreq_khz;
    return I3C_STATUS_OK;
}

i3c_status_t adapter_i3c_set_drivestrength(uint8_t drivestrength_mA) {
    enum gpio_drive_strength ds;
    switch (drivestrength_mA) {
        case 2: ds = GPIO_DRIVE_STRENGTH_2MA; break;
        case 4: ds = GPIO_DRIVE_STRENGTH_4MA; break;
        case 8: ds = GPIO_DRIVE_STRENGTH_8MA; break;
        case 12: ds = GPIO_DRIVE_STRENGTH_12MA; break;
        default: return I3C_STATUS_PARAM_OUTOFRANGE;
    }
    gpio_set_drive_strength(i3c_sda_pin, ds);
    gpio_set_drive_strength(i3c_scl_pin, ds);
    return I3C_STATUS_OK;
}

bool adapter_i3c_bind_claim(uint32_t handle) {
    const pin_owner_claim_t *claim = pin_owner_get_claim(handle);
    if (claim == NULL || claim->observer_only != 0 || claim->pin_count != 2) {
        return false;
    }
    if (!pin_owner_is_flexible(claim->pins[0]) || !pin_owner_is_flexible(claim->pins[1])) {
        return false;
    }
    i3c_sda_pin = claim->pins[0];
    i3c_scl_pin = claim->pins[1];
    i3c_active_claim_handle = handle;
    return true;
}

bool adapter_i3c_release_claim(void) {
    if (i3c_active_claim_handle != 0) {
        adapter_i3c_deinit();
        pin_owner_result_t res;
        pin_owner_release_claim(i3c_active_claim_handle, &res);
        i3c_active_claim_handle = 0;
        return true;
    }
    return false;
}

bool adapter_i3c_is_claimed(void) {
    return i3c_active_claim_handle != 0;
}

static bool __not_in_flash_func(i3c_ibi_type1_check)(void) {
    return ((io_bank0_hw->io[i3c_sda_pin].status & IO_BANK0_GPIO0_STATUS_INFROMPAD_BITS) >> IO_BANK0_GPIO0_STATUS_INFROMPAD_LSB) == 0;
}

static inline void __not_in_flash_func(i3c_start)(void) {
    i3c_pio_put32(I3CPIO_OPCODE_START);
}

static inline void __not_in_flash_func(i3c_restart)(void) {
    i3c_pio_put32(I3CPIO_OPCODE_RESTART);
    i3c_pio_put32(I3CPIO_OPCODE_SCL0);
}

static inline void __not_in_flash_func(i3c_stop)(void) {
    i3c_pio_put32(I3CPIO_OPCODE_STOP);
}

static void __not_in_flash_func(i3c_sdr_write)(uint8_t value) {
    i3c_pio_wait_tx_empty();
    i3c_pio_set_autopush(9);
    i3c_pio_put32_no_check(i3c_wdata_table[(uint32_t)value * 2 + 0]);
    i3c_pio_put32_no_check(i3c_wdata_table[(uint32_t)value * 2 + 1]);
    i3c_pio_put32_no_check(I3CPIO_OPCODE_SCL0);
    i3c_pio_get32(); /* dump read word */
}

static uint32_t __not_in_flash_func(i3c_sdr_read)(uint8_t continuetransfer) {
    uint32_t data;
    i3c_pio_wait_tx_empty();
    i3c_pio_set_autopush(9);
    i3c_pio_put32_no_check(I3CPIO_OPCODE_XFER(6, SDR_RBIT(0), SDR_RBIT(0), SDR_RBIT(0), SDR_RBIT(0), SDR_RBIT(0), SDR_RBIT(0)));
    i3c_pio_put32_no_check(I3CPIO_OPCODE_XFER(2, SDR_RBIT(0), SDR_RBIT(0), 0, 0, 0, 0));
    i3c_pio_put32_no_check(I3CPIO_OPCODE_T_BIT(continuetransfer));
    data = i3c_pio_get32();
    return data;
}

static uint8_t __not_in_flash_func(i3c_od_read8)(void) {
    uint32_t cmdword0, cmdword1;
    i3c_pio_wait_tx_empty();
    i3c_pio_set_autopush(8);
    cmdword0 = I3CPIO_OPCODE_XFER(6, OD_RBIT, OD_RBIT, OD_RBIT, OD_RBIT, OD_RBIT, OD_RBIT);
    cmdword1 = I3CPIO_OPCODE_XFER(2, OD_RBIT, OD_RBIT, 0, 0, 0, 0);
    i3c_pio_put32_no_check(cmdword0);
    i3c_pio_put32_no_check(cmdword1);
    i3c_pio_put32_no_check(I3CPIO_OPCODE_SCL0);
    return (uint8_t)i3c_pio_get32();
}

static inline uint8_t __not_in_flash_func(i3c_od_read)(uint8_t ack) {
    uint32_t cmdword0, cmdword1, data;
    i3c_pio_wait_tx_empty();
    i3c_pio_set_autopush(9);
    cmdword0 = I3CPIO_OPCODE_XFER(6, OD_RBIT, OD_RBIT, OD_RBIT, OD_RBIT, OD_RBIT, OD_RBIT);
    cmdword1 = I3CPIO_OPCODE_XFER(3, OD_RBIT, OD_RBIT, OD_WBIT((ack ^ 1) & 1), 0, 0, 0);
    i3c_pio_put32_no_check(cmdword0);
    i3c_pio_put32_no_check(cmdword1);
    i3c_pio_put32_no_check(I3CPIO_OPCODE_SCL0);
    data = i3c_pio_get32();
    return ((uint8_t)(data >> 1));
}

static i3c_status_t __not_in_flash_func(i3c_sdr_write_addr)(uint8_t value) {
    uint32_t cmdword0, cmdword1, resp;
    i3c_status_t retcode = I3C_STATUS_OK;

    i3c_pio_wait_tx_empty();
    i3c_pio_set_autopush(9);
    cmdword0 = I3CPIO_OPCODE_XFER(6, SDR_WBIT((value >> 7) & 1), SDR_WBIT((value >> 6) & 1), SDR_WBIT((value >> 5) & 1),
                                  SDR_WBIT((value >> 4) & 1), SDR_WBIT((value >> 3) & 1), SDR_WBIT((value >> 2) & 1));
    cmdword1 = I3CPIO_OPCODE_XFER(3, SDR_WBIT((value >> 1) & 1), SDR_WBIT((value >> 0) & 1), OD_RACKBIT, 0, 0, 0);

    i3c_pio_put32_no_check(cmdword0);
    i3c_pio_put32_no_check(cmdword1);
    i3c_pio_put32_no_check(I3CPIO_OPCODE_SCL0);
    resp = i3c_pio_get32();
    if (resp & 1) {
        retcode = I3C_STATUS_NAK_DURING_SDRADDR;
    }
    return retcode;
}

static i3c_status_t __not_in_flash_func(i3c_arbhdr)(uint8_t *parbdata) {
    uint32_t cmdword0, cmdword1, cmdword2;
    uint32_t data0, data1;
    uint8_t arbdata;
    i3c_status_t retcode = I3C_STATUS_OK;

    i3c_pio_wait_tx_empty();
    i3c_pio_set_autopush(6);
    cmdword0 = I3CPIO_OPCODE_XFER(6, OD_WBIT(1), OD_WBIT(1), OD_WBIT(1), OD_WBIT(1), OD_WBIT(1), OD_WBIT(1));
    cmdword1 = I3CPIO_OPCODE_XFER(3, OD_WBIT(0), OD_WBIT(0), OD_RACKBIT, 0, 0, 0);
    cmdword2 = I3CPIO_OPCODE_XFER(3, OD_WBIT(1), OD_WBIT(1), OD_WBIT(0), 0, 0, 0);

    i3c_pio_put32_no_check(cmdword0);
    i3c_pio_put32_no_check(I3CPIO_OPCODE_SCL0);
    data0 = i3c_pio_get32();
    i3c_pio_set_autopush(3);

    if (data0 == 0x3ful) {
        i3c_pio_put32_no_check(cmdword1);
        i3c_pio_put32_no_check(I3CPIO_OPCODE_SCL0);
        data1 = i3c_pio_get32();
        if (data1 & 1) {
            retcode = I3C_STATUS_NAK_DURING_ARBHDR;
        }
    } else {
        i3c_pio_put32_no_check(cmdword2);
        i3c_pio_put32_no_check(I3CPIO_OPCODE_SCL0);
        data1 = i3c_pio_get32();
        retcode = I3C_STATUS_IBI;
    }

    arbdata = (data0 << 2) | (data1 >> 1);
    if (parbdata) *parbdata = arbdata;
    i3c_hl_arbcode = arbdata;
    return retcode;
}

i3c_status_t adapter_i3c_init(uint8_t sda_pin, uint8_t scl_pin, uint32_t freq_khz) {
    if (freq_khz == 0) freq_khz = 12500;
    if (freq_khz > 12500 || freq_khz < 40) return I3C_STATUS_PARAM_OUTOFRANGE;

    if (i3c_active_claim_handle == 0) {
        uint8_t pins[2] = { sda_pin, scl_pin };
        uint8_t func = GPIO_FUNC_PIO2;
#if NUM_PIOS < 3
        func = GPIO_FUNC_PIO0;
#endif
        if (!pin_owner_claim_pins(pins, 2, func, 0, &i3c_active_claim_handle)) {
            return I3C_STATUS_CLAIM_ERROR;
        }
    }

    i3c_sda_pin = sda_pin;
    i3c_scl_pin = scl_pin;

    /* Build RAM copies of SDR and overlay programs */
    for (uint32_t i = 0; i < sizeof(i3c_hl_pio_program_sdr) / sizeof(uint32_t); i++) {
        if (i < (sizeof(i3c_program_instructions) / sizeof(uint16_t))) {
            i3c_hl_pio_program_sdr[i] = i3c_program_instructions[i];
        } else {
            i3c_hl_pio_program_sdr[i] = 0x0ul;
        }
        if (i < (sizeof(i3c_overlay_program_instructions) / sizeof(uint16_t))) {
            i3c_hl_pio_program_sdr_overlay[i] = i3c_overlay_program_instructions[i];
        } else {
            i3c_hl_pio_program_sdr_overlay[i] = 0x0ul;
        }
    }

    s_i3c_program_hdr_sdr_sm();

    I3C_PIO->sm[I3C_SM].pinctrl =
        (1 << PIO_SM0_PINCTRL_SET_COUNT_LSB) |
        (scl_pin << PIO_SM0_PINCTRL_SET_BASE_LSB) |
        (2 << PIO_SM0_PINCTRL_OUT_COUNT_LSB) |
        (sda_pin << PIO_SM0_PINCTRL_OUT_BASE_LSB) |
        (sda_pin << PIO_SM0_PINCTRL_IN_BASE_LSB) |
        (2 << PIO_SM0_PINCTRL_SIDESET_COUNT_LSB) |
        (scl_pin << PIO_SM0_PINCTRL_SIDESET_BASE_LSB);

    uint func_pio = GPIO_FUNC_PIO2;
#if NUM_PIOS < 3
    func_pio = GPIO_FUNC_PIO0;
#endif
    gpio_set_function(sda_pin, func_pio);
    gpio_set_function(scl_pin, func_pio);
    gpio_set_drive_strength(sda_pin, GPIO_DRIVE_STRENGTH_12MA);
    gpio_set_drive_strength(scl_pin, GPIO_DRIVE_STRENGTH_12MA);
    gpio_set_slew_rate(sda_pin, GPIO_SLEW_RATE_FAST);
    gpio_set_slew_rate(scl_pin, GPIO_SLEW_RATE_FAST);

    I3C_PIO->sm[I3C_SM].execctrl = (i3c_wrap << PIO_SM0_EXECCTRL_WRAP_TOP_LSB) |
                                   (i3c_wrap_target << PIO_SM0_EXECCTRL_WRAP_BOTTOM_LSB) | 
                                   (1 << PIO_SM0_EXECCTRL_SIDE_EN_LSB) | 
                                   (sda_pin << PIO_SM0_EXECCTRL_JMP_PIN_LSB);

    I3C_PIO->sm[I3C_SM].shiftctrl = (0 << PIO_SM0_SHIFTCTRL_PULL_THRESH_LSB) | 
                                    (1 << PIO_SM0_SHIFTCTRL_OUT_SHIFTDIR_LSB) |
                                    (0 << PIO_SM0_SHIFTCTRL_IN_SHIFTDIR_LSB) |
                                    (0 << PIO_SM0_SHIFTCTRL_PUSH_THRESH_LSB) |
                                    (1 << PIO_SM0_SHIFTCTRL_AUTOPUSH_LSB) |
                                    (0 << PIO_SM0_SHIFTCTRL_AUTOPULL_LSB);

    for (uint8_t bitcount = 0; bitcount < 32; bitcount++) {
        i3c_pio_shiftctrl_autopointer_tab[bitcount] = 
            I3C_PIO->sm[I3C_SM].shiftctrl | (((uint32_t)(bitcount & 0x1f)) << PIO_SM0_SHIFTCTRL_PUSH_THRESH_LSB);
    }

    hw_set_bits(&I3C_PIO->ctrl, 1u << (PIO_CTRL_SM_ENABLE_LSB + I3C_SM));
    hw_set_bits(&I3C_PIO->input_sync_bypass, (1u << sda_pin) | (1u << scl_pin));

    i3c_pio_put32(I3CPIO_OPCODE_SETDIR_SCLSDA_SCLHIGH(1, 0));

    I3C_PIO->sm[I3C_SM].pinctrl =
        (1 << PIO_SM0_PINCTRL_SET_COUNT_LSB) |
        (sda_pin << PIO_SM0_PINCTRL_SET_BASE_LSB) |
        (1 << PIO_SM0_PINCTRL_OUT_COUNT_LSB) |
        (sda_pin << PIO_SM0_PINCTRL_OUT_BASE_LSB) |
        (sda_pin << PIO_SM0_PINCTRL_IN_BASE_LSB) |
        (2 << PIO_SM0_PINCTRL_SIDESET_COUNT_LSB) |
        (scl_pin << PIO_SM0_PINCTRL_SIDESET_BASE_LSB);

    /* Precalculate write table for fast sdr_write execution */
    for (uint32_t value = 0; value < 256; value++) {
        uint8_t tbit = __builtin_parity(value) ^ 1;
        i3c_wdata_table[value * 2u + 0u] = I3CPIO_OPCODE_XFER(6, SDR_WBIT((value >> 7) & 1), SDR_WBIT((value >> 6) & 1),
                                                                 SDR_WBIT((value >> 5) & 1), SDR_WBIT((value >> 4) & 1),
                                                                 SDR_WBIT((value >> 3) & 1), SDR_WBIT((value >> 2) & 1));
        i3c_wdata_table[value * 2u + 1u] = I3CPIO_OPCODE_XFER(3, SDR_WBIT((value >> 1) & 1), SDR_WBIT((value >> 0) & 1),
                                                                 SDR_WBIT(tbit), 0, 0, 0);
    }

    adapter_i3c_set_clkrate(freq_khz);
    i3c_hl_arbcode = 0xfc;
    i3c_initialized = true;

    return I3C_STATUS_OK;
}

void adapter_i3c_deinit(void) {
    if (!i3c_initialized) return;
    hw_clear_bits(&I3C_PIO->ctrl, 1u << (PIO_CTRL_SM_ENABLE_LSB + I3C_SM));
    gpio_set_function(i3c_sda_pin, GPIO_FUNC_SIO);
    gpio_set_function(i3c_scl_pin, GPIO_FUNC_SIO);
    gpio_set_dir(i3c_sda_pin, GPIO_IN);
    gpio_set_dir(i3c_scl_pin, GPIO_IN);
    i3c_initialized = false;
}

i3c_status_t adapter_i3c_targetreset(void) {
    if (!i3c_initialized) return I3C_STATUS_NOT_INITIALIZED;
    i3c_pio_wait_tx_empty();
    i3c_pio_put32_no_check(I3CPIO_OPCODE_SCL0);
    i3c_pio_put32_no_check(I3CPIO_OPCODE_SDASTATE(1));
    i3c_pio_put32_no_check(I3CPIO_OPCODE_SDADIR(1));

    for (int i = 0; i < 7; i++) {
        i3c_pio_put32(I3CPIO_OPCODE_SDASTATE(0));
        i3c_pio_put32(I3CPIO_OPCODE_SDASTATE(1));
    }
    i3c_pio_put32(I3CPIO_OPCODE_SCL1);
    i3c_pio_put32(I3CPIO_OPCODE_SDASTATE(0));
    i3c_pio_put32(I3CPIO_OPCODE_SDASTATE(1));
    i3c_pio_put32(I3CPIO_OPCODE_SDADIR(0));
    return I3C_STATUS_OK;
}

i3c_status_t adapter_i3c_entdaa(uint8_t addr, i3c_device_info_t *dev_info) {
    if (!i3c_initialized) return I3C_STATUS_NOT_INITIALIZED;
    i3c_status_t retcode = I3C_STATUS_OK;
    uint8_t arbhdr = 0;
    uint32_t prev = save_and_disable_interrupts();

    if (i3c_ibi_type1_check()) {
        retcode = I3C_STATUS_IBI;
    }
    if (retcode == I3C_STATUS_OK) {
        i3c_start();
        retcode = i3c_arbhdr(&arbhdr);
        if (retcode == I3C_STATUS_OK) {
            i3c_sdr_write(0x07); /* CCC Broadcast ENTDAA */
            i3c_restart();
            retcode = i3c_sdr_write_addr((0x7eu << 1) | 1);
            if (retcode == I3C_STATUS_OK) {
                uint8_t pid_raw[8];
                for (uint8_t i = 0; i < 8; i++) {
                    pid_raw[i] = i3c_od_read8();
                }
                if (dev_info) {
                    memcpy(dev_info->pid, pid_raw, 6);
                    dev_info->bcr = pid_raw[6];
                    dev_info->dcr = pid_raw[7];
                    dev_info->dynamic_addr = addr;
                }
                uint8_t target_addr_field = (addr << 1) | (__builtin_parity(addr << 1) ^ 1);
                retcode = i3c_sdr_write_addr(target_addr_field);
            }
        }
        if (retcode != I3C_STATUS_IBI) {
            i3c_stop();
        }
    }
    restore_interrupts(prev);
    return retcode;
}

i3c_status_t adapter_i3c_rstdaa(void) {
    if (!i3c_initialized) return I3C_STATUS_NOT_INITIALIZED;
    uint32_t prev = save_and_disable_interrupts();
    i3c_status_t retcode = I3C_STATUS_OK;

    if (i3c_ibi_type1_check()) {
        retcode = I3C_STATUS_IBI;
    }
    if (retcode == I3C_STATUS_OK) {
        i3c_start();
        retcode = i3c_arbhdr(NULL);
        if (retcode == I3C_STATUS_OK) {
            i3c_sdr_write(0x06); /* CCC Broadcast RSTDAA */
        }
        if (retcode != I3C_STATUS_IBI) {
            i3c_stop();
        }
    }
    restore_interrupts(prev);
    return retcode;
}

i3c_status_t adapter_i3c_checkack(uint8_t addr) {
    if (!i3c_initialized) return I3C_STATUS_NOT_INITIALIZED;
    i3c_status_t retcode = I3C_STATUS_OK;
    if (i3c_ibi_type1_check()) {
        return I3C_STATUS_IBI;
    }
    i3c_start();
    retcode = i3c_arbhdr(NULL);
    if (retcode == I3C_STATUS_OK) {
        i3c_restart();
        retcode = i3c_sdr_write_addr(addr << 1);
    }
    if (retcode != I3C_STATUS_IBI) {
        i3c_stop();
    }
    sleep_us(100);
    return retcode;
}

i3c_status_t adapter_i3c_sdr_privwrite(uint8_t addr, const uint8_t *pdat, uint32_t bytecount) {
    if (!i3c_initialized) return I3C_STATUS_NOT_INITIALIZED;
    uint32_t prev = save_and_disable_interrupts();
    i3c_status_t retcode = I3C_STATUS_OK;

    if (i3c_ibi_type1_check()) {
        retcode = I3C_STATUS_IBI;
    }
    if (retcode == I3C_STATUS_OK) {
        i3c_start();
        retcode = i3c_arbhdr(NULL);
        if (retcode == I3C_STATUS_OK) {
            i3c_restart();
            retcode = i3c_sdr_write_addr(addr << 1);
            if (retcode == I3C_STATUS_OK) {
                while (bytecount--) {
                    i3c_sdr_write(*pdat++);
                }
            }
        }
        if (retcode != I3C_STATUS_IBI) {
            i3c_stop();
        }
    }
    restore_interrupts(prev);
    return retcode;
}

i3c_status_t adapter_i3c_sdr_privread(uint8_t addr, uint8_t *pdat, uint32_t *pbytecount) {
    if (!i3c_initialized) return I3C_STATUS_NOT_INITIALIZED;
    uint32_t prev = save_and_disable_interrupts();
    uint32_t readbytecount = 0;
    i3c_status_t retcode = I3C_STATUS_OK;
    uint32_t bytecount = *pbytecount;

    if (i3c_ibi_type1_check()) {
        retcode = I3C_STATUS_IBI;
    }
    if (retcode == I3C_STATUS_OK) {
        i3c_start();
        retcode = i3c_arbhdr(NULL);
        if (retcode == I3C_STATUS_OK) {
            i3c_restart();
            retcode = i3c_sdr_write_addr((addr << 1) | 1);
            s_i3c_program_sdr_overlay_sm(true);
            if (retcode == I3C_STATUS_OK) {
                bool done = false;
                while (bytecount && !done) {
                    bytecount--;
                    uint32_t value = i3c_sdr_read(bytecount == 0);
                    done = !(value & 1);
                    *pdat++ = (uint8_t)(value >> 1);
                    readbytecount++;
                }
            }
            s_i3c_program_sdr_overlay_sm(false);
        }
        if (retcode != I3C_STATUS_IBI) {
            i3c_stop();
        }
        *pbytecount = readbytecount;
    }
    restore_interrupts(prev);
    return retcode;
}

i3c_status_t adapter_i3c_sdr_privwriteread(uint8_t addr, const uint8_t *pwritedat, uint32_t writebytecount,
                                           uint8_t *preaddat, uint32_t *preadbytecount) {
    if (!i3c_initialized) return I3C_STATUS_NOT_INITIALIZED;
    uint32_t prev = save_and_disable_interrupts();
    i3c_status_t retcode = I3C_STATUS_OK;
    uint32_t readbytecount = 0;

    if (i3c_ibi_type1_check()) {
        retcode = I3C_STATUS_IBI;
    }
    if (retcode == I3C_STATUS_OK) {
        i3c_start();
        retcode = i3c_arbhdr(NULL);
        if (retcode == I3C_STATUS_OK) {
            i3c_restart();
            retcode = i3c_sdr_write_addr(addr << 1);
            if (retcode == I3C_STATUS_OK) {
                while (writebytecount--) {
                    i3c_sdr_write(*pwritedat++);
                }
                i3c_restart();
                retcode = i3c_sdr_write_addr((addr << 1) | 1);
                s_i3c_program_sdr_overlay_sm(true);
                if (retcode == I3C_STATUS_OK) {
                    uint32_t readlen = *preadbytecount;
                    bool done = false;
                    while (readlen && !done) {
                        readlen--;
                        uint32_t value = i3c_sdr_read(readlen == 0);
                        done = !(value & 1);
                        *preaddat++ = (uint8_t)(value >> 1);
                        readbytecount++;
                    }
                    *preadbytecount = readbytecount;
                }
                s_i3c_program_sdr_overlay_sm(false);
            }
        }
        if (retcode != I3C_STATUS_IBI) {
            i3c_stop();
        }
    }
    restore_interrupts(prev);
    return retcode;
}

i3c_status_t adapter_i3c_sdr_ccc_broadcast_write(const uint8_t *pdat, uint32_t bytecount) {
    if (!i3c_initialized) return I3C_STATUS_NOT_INITIALIZED;
    uint32_t prev = save_and_disable_interrupts();
    i3c_status_t retcode = I3C_STATUS_OK;

    if (i3c_ibi_type1_check()) {
        retcode = I3C_STATUS_IBI;
    }
    if (retcode == I3C_STATUS_OK) {
        i3c_start();
        retcode = i3c_arbhdr(NULL);
        if (retcode == I3C_STATUS_OK) {
            while (bytecount--) {
                i3c_sdr_write(*pdat++);
            }
        }
        if (retcode != I3C_STATUS_IBI) {
            i3c_stop();
        }
    }
    restore_interrupts(prev);
    return retcode;
}

i3c_status_t adapter_i3c_sdr_ccc_direct_write(const uint8_t *pdat, uint32_t bytecount,
                                              uint8_t addr, const uint8_t *pdirectdat, uint32_t directbytecount) {
    if (!i3c_initialized) return I3C_STATUS_NOT_INITIALIZED;
    uint32_t prev = save_and_disable_interrupts();
    i3c_status_t retcode = I3C_STATUS_OK;

    if (i3c_ibi_type1_check()) {
        retcode = I3C_STATUS_IBI;
    }
    if (retcode == I3C_STATUS_OK) {
        i3c_start();
        retcode = i3c_arbhdr(NULL);
        if (retcode == I3C_STATUS_OK) {
            while (bytecount--) {
                i3c_sdr_write(*pdat++);
            }
            i3c_restart();
            retcode = i3c_sdr_write_addr(addr << 1);
            if (retcode == I3C_STATUS_OK) {
                while (directbytecount--) {
                    i3c_sdr_write(*pdirectdat++);
                }
            }
        }
        if (retcode != I3C_STATUS_IBI) {
            i3c_stop();
        }
    }
    restore_interrupts(prev);
    return retcode;
}

i3c_status_t adapter_i3c_sdr_ccc_direct_read(const uint8_t *pdat, uint32_t bytecount,
                                             uint8_t addr, uint8_t *pdirectdat, uint32_t *pdirectbytecount) {
    if (!i3c_initialized) return I3C_STATUS_NOT_INITIALIZED;
    uint32_t prev = save_and_disable_interrupts();
    i3c_status_t retcode = I3C_STATUS_OK;
    uint32_t readbytecount = 0;

    if (i3c_ibi_type1_check()) {
        retcode = I3C_STATUS_IBI;
    }
    if (retcode == I3C_STATUS_OK) {
        i3c_start();
        retcode = i3c_arbhdr(NULL);
        if (retcode == I3C_STATUS_OK) {
            while (bytecount--) {
                i3c_sdr_write(*pdat++);
            }
            i3c_restart();
            s_i3c_program_sdr_overlay_sm(true);
            retcode = i3c_sdr_write_addr((addr << 1) | 1);
            if (retcode == I3C_STATUS_OK) {
                uint32_t readlen = *pdirectbytecount;
                bool done = false;
                while (readlen && !done) {
                    readlen--;
                    uint32_t value = i3c_sdr_read(readlen == 0);
                    done = !(value & 1);
                    *pdirectdat++ = (uint8_t)(value >> 1);
                    readbytecount++;
                }
                *pdirectbytecount = readbytecount;
            }
            s_i3c_program_sdr_overlay_sm(false);
        }
        if (retcode != I3C_STATUS_IBI) {
            i3c_stop();
        }
    }
    restore_interrupts(prev);
    return retcode;
}

i3c_status_t adapter_i3c_poll(uint8_t *pdat, uint32_t *plen) {
    if (!i3c_initialized) return I3C_STATUS_NOT_INITIALIZED;
    uint32_t maxlen = *plen;
    *plen = 0;

    if (i3c_hl_arbcode != 0xfc) {
        /* IBI Type 2 during arbitration header */
        bool done = false;
        if (maxlen > 0) {
            *pdat++ = i3c_hl_arbcode;
            *plen = *plen + 1;
            maxlen--;
        }
        s_i3c_program_sdr_overlay_sm(true);
        while (maxlen && !done) {
            maxlen--;
            uint32_t value = i3c_sdr_read(maxlen == 0);
            done = !(value & 1);
            *pdat++ = (uint8_t)(value >> 1);
            *plen = *plen + 1;
        }
        s_i3c_program_sdr_overlay_sm(false);
        i3c_stop();
        i3c_hl_arbcode = 0xfc;
        return I3C_STATUS_OK;
    } else if (i3c_ibi_type1_check()) {
        /* IBI Type 1 during idle bus (SDA pulled low) */
        uint8_t ibiword = i3c_od_read(1);
        s_i3c_program_sdr_overlay_sm(true);
        if (maxlen > 0) {
            *pdat++ = ibiword;
            *plen = *plen + 1;
            maxlen--;
        }
        if (ibiword != 0x04) { /* not Hot-Join */
            bool done = false;
            while (maxlen && !done) {
                maxlen--;
                uint32_t value = i3c_sdr_read(maxlen == 0);
                done = !(value & 1);
                *pdat++ = (uint8_t)(value >> 1);
                *plen = *plen + 1;
            }
        }
        s_i3c_program_sdr_overlay_sm(false);
        i3c_stop();
        return I3C_STATUS_OK;
    }

    return I3C_STATUS_NO_IBI;
}

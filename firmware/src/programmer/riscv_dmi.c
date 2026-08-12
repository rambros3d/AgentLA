#include "riscv_dmi.h"
#include "swio.h"
#include "rvswd.h"
#include <stdio.h>

ch32_result_t riscv_dmi_write(uint8_t reg, uint32_t data) {
    if (g_ch32_target.mode == CH32_MODE_1WIRE_SDI) {
        /* Use PIO-based SWIO engine for ~10x faster writes */
        swio_put(reg, data);
        return CH32_OK;
    } else {
        /* CH32V20x/V30x use WCH's RVSWD 2-wire protocol, NOT ARM SWD.
         * DMI registers are addressed directly in the RVSWD header:
         *   [7-bit reg][1-bit op][1-bit parity] ...
         * The RVSWD transport handles the wire framing. */
        rvswd_result_t res = rvswd_write(reg, data);
        return (res == RVSWD_OK) ? CH32_OK : CH32_ERR_TIMEOUT;
    }
}

ch32_result_t riscv_dmi_read(uint8_t reg, uint32_t *data) {
    if (!data) return CH32_ERR_UNSUPPORTED;

    if (g_ch32_target.mode == CH32_MODE_1WIRE_SDI) {
        /* Use PIO-based SWIO engine for ~10x faster reads */
        *data = swio_get(reg);
        return CH32_OK;
    } else {
        /* CH32V20x/V30x: RVSWD 2-wire DMI read */
        rvswd_result_t res = rvswd_read(reg, data);
        return (res == RVSWD_OK) ? CH32_OK : CH32_ERR_TIMEOUT;
    }
}

static uint32_t current_progbuf0 = 0;

static ch32_result_t wait_abstract_cmd_complete(void) {
    uint32_t abstractcs = 0;
    for (int i = 0; i < 500; i++) {
        ch32_result_t res = riscv_dmi_read(DMI_ABSTRACTCS, &abstractcs);
        if (res != CH32_OK) return res;
        if (!(abstractcs & ABSTRACTCS_BUSY)) {
            if ((abstractcs & ABSTRACTCS_ERR_MASK) != ABSTRACTCS_ERR_NONE) {
                /* Clear error bits */
                riscv_dmi_write(DMI_ABSTRACTCS, ABSTRACTCS_ERR_MASK);
                return CH32_ERR_NACK;
            }
            return CH32_OK;
        }
        sleep_us(10);
    }
    return CH32_ERR_TIMEOUT;
}

static ch32_result_t riscv_debug_init(void) {
    /* Quick check: verify DM register access works */
    uint32_t val = 0;

    /* Route through the active transport (RVSWD for 2-wire, SWIO for 1-wire) */
    ch32_result_t r = riscv_dmi_read(DMI_DMSTATUS, &val);
    if (r != CH32_OK || val == 0 || val == 0xFFFFFFFF) {
        return CH32_ERR_TIMEOUT;
    }

    return CH32_OK;
}

ch32_result_t riscv_halt(void) {
    current_progbuf0 = 0;
    /* Ensure debug module is active */
    ch32_result_t init_res = riscv_debug_init();
    if (init_res != CH32_OK) return init_res;

    ch32_result_t res = riscv_dmi_write(DMI_DMCONTROL, DMCONTROL_HALTREQ | DMCONTROL_DMACTIVE);
    if (res != CH32_OK) return res;

    for (int i = 0; i < 100; i++) {
        uint32_t status = 0;
        res = riscv_dmi_read(DMI_DMSTATUS, &status);
        if (res == CH32_OK && (status & DMSTATUS_ALLHALTED)) {
            riscv_dmi_write(DMI_ABSTRACTCS, ABSTRACTCS_ERR_MASK);
            return CH32_OK;
        }
        sleep_us(100);
    }
    return CH32_ERR_TIMEOUT;
}

ch32_result_t riscv_resume(void) {
    current_progbuf0 = 0;
    ch32_result_t res = riscv_dmi_write(DMI_DMCONTROL, DMCONTROL_RESUMEREQ | DMCONTROL_DMACTIVE);
    if (res != CH32_OK) return res;

    for (int i = 0; i < 100; i++) {
        uint32_t status = 0;
        res = riscv_dmi_read(DMI_DMSTATUS, &status);
        if (res == CH32_OK && (status & DMSTATUS_ALLRUNNING)) {
            /* Clear RESUMEREQ bit so core can halt again (e.g. via ebreak) */
            riscv_dmi_write(DMI_DMCONTROL, DMCONTROL_DMACTIVE);
            return CH32_OK;
        }
        sleep_us(100);
    }
    return CH32_ERR_TIMEOUT;
}

ch32_result_t riscv_is_halted(bool *halted) {
    if (!halted) return CH32_ERR_UNSUPPORTED;
    uint32_t status = 0;
    ch32_result_t res = riscv_dmi_read(DMI_DMSTATUS, &status);
    if (res != CH32_OK) return res;

    *halted = (status & DMSTATUS_ALLHALTED) != 0;
    return CH32_OK;
}

ch32_result_t riscv_read_reg(uint16_t regno, uint32_t *val) {
    if (!val) return CH32_ERR_UNSUPPORTED;

    uint32_t cmd = CMD_TYPE_ACCESS_REG | CMD_TRANSFER | CMD_AARSIZE_32 | CMD_REGNO(regno);
    ch32_result_t res = riscv_dmi_write(DMI_COMMAND, cmd);
    if (res != CH32_OK) return res;

    res = wait_abstract_cmd_complete();
    if (res != CH32_OK) return res;

    return riscv_dmi_read(DMI_DATA0, val);
}

ch32_result_t riscv_write_reg(uint16_t regno, uint32_t val) {
    ch32_result_t res = riscv_dmi_write(DMI_DATA0, val);
    if (res != CH32_OK) return res;

    uint32_t cmd = CMD_TYPE_ACCESS_REG | CMD_TRANSFER | CMD_WRITE | CMD_AARSIZE_32 | CMD_REGNO(regno);
    res = riscv_dmi_write(DMI_COMMAND, cmd);
    if (res != CH32_OK) return res;

    return wait_abstract_cmd_complete();
}

/*
 * Wait for an in-flight SBA access to complete (SBBUSY cleared).
 * Bounded so a stuck bus can never hang the firmware.
 */
static ch32_result_t riscv_sba_wait_busy(void) {
    for (int i = 0; i < 200; i++) {
        uint32_t sbcs = 0;
        ch32_result_t res = riscv_dmi_read(DMI_SBCS, &sbcs);
        if (res != CH32_OK) return res;
        if (!(sbcs & SBCS_SBBUSY)) {
            return CH32_OK;
        }
        sleep_us(100);
    }
    return CH32_ERR_TIMEOUT;
}

/*
 * System Bus Access read (RISC-V Debug Spec 0.13):
 *   1. Configure SBCS: 32-bit access, trigger read on SBADDRESS0 write
 *   2. Write SBADDRESS0 = addr  → initiates the read
 *   3. Poll SBCS.SBBUSY == 0
 *   4. Read SBDATA0
 */
ch32_result_t riscv_read_mem(uint32_t addr, uint32_t *val) {
    if (!val) return CH32_ERR_UNSUPPORTED;

    uint32_t sbcs = 0;
    riscv_dmi_read(DMI_SBCS, &sbcs);
    if (sbcs != 0) {
        ch32_result_t res = riscv_dmi_write(DMI_SBCS, SBCS_SBREADONADDR | SBCS_SBACCESS_32);
        if (res != CH32_OK) return res;

        res = riscv_dmi_write(DMI_SBADDRESS0, addr);
        if (res != CH32_OK) return res;

        res = riscv_sba_wait_busy();
        if (res != CH32_OK) return res;

        return riscv_dmi_read(DMI_SBDATA0, val);
    }

    /* Fallback for chips without SBA (e.g. CH32V203 / V307):
     * Use PROGBUF instruction execution (lw a0, 0(a1)) */
    bool was_halted = false;
    riscv_is_halted(&was_halted);
    if (!was_halted) {
        riscv_halt();
    }

    /* Load PROGBUF0 = lw a0, 0(a1) (0x0005a503), PROGBUF1 = ebreak (0x00100073) */
    if (current_progbuf0 != 0x0005a503) {
        riscv_dmi_write(DMI_ABSTRACTCS, ABSTRACTCS_ERR_MASK);
        riscv_dmi_write(DMI_PROGBUF0, 0x0005a503);
        riscv_dmi_write(DMI_PROGBUF1, 0x00100073);
        current_progbuf0 = 0x0005a503;
    }

    /* Set a1 (reg 0x100b) = addr */
    riscv_dmi_write(DMI_DATA0, addr);
    ch32_result_t res = riscv_dmi_write(DMI_COMMAND, CMD_TYPE_ACCESS_REG | CMD_TRANSFER | CMD_WRITE | CMD_AARSIZE_32 | CMD_REGNO(0x100B));
    if (res != CH32_OK) {
        if (!was_halted) riscv_resume();
        return res;
    }
    wait_abstract_cmd_complete();

    /* Execute PROGBUF (postexec) */
    res = riscv_dmi_write(DMI_COMMAND, CMD_TYPE_ACCESS_REG | CMD_POSTEXEC);
    if (res != CH32_OK) {
        if (!was_halted) riscv_resume();
        return res;
    }
    wait_abstract_cmd_complete();

    /* Read a0 (reg 0x100a) into DATA0 */
    res = riscv_dmi_write(DMI_COMMAND, CMD_TYPE_ACCESS_REG | CMD_TRANSFER | CMD_AARSIZE_32 | CMD_REGNO(0x100A));
    if (res != CH32_OK) {
        if (!was_halted) riscv_resume();
        return res;
    }
    wait_abstract_cmd_complete();

    res = riscv_dmi_read(DMI_DATA0, val);

    if (!was_halted) {
        riscv_resume();
    }
    return res;
}

/*
 * System Bus Access write (RISC-V Debug Spec 0.13):
 *   1. Configure SBCS: 32-bit access (no read-on-address)
 *   2. Write SBDATA0 = data
 *   3. Write SBADDRESS0 = addr  → initiates the write
 *   4. Poll SBCS.SBBUSY == 0
 */
ch32_result_t riscv_write_mem(uint32_t addr, uint32_t val) {
    uint32_t sbcs = 0;
    riscv_dmi_read(DMI_SBCS, &sbcs);

    if (sbcs != 0) {
        ch32_result_t res = riscv_dmi_write(DMI_SBCS, SBCS_SBACCESS_32);
        if (res != CH32_OK) return res;

        res = riscv_dmi_write(DMI_SBDATA0, val);
        if (res != CH32_OK) return res;

        res = riscv_dmi_write(DMI_SBADDRESS0, addr);
        if (res != CH32_OK) return res;

        return riscv_sba_wait_busy();
    }

    /* Fallback for chips without SBA (e.g. CH32V203 / V307):
     * Use PROGBUF instruction execution (sw s0, 0(a1)) */
    bool was_halted = false;
    riscv_is_halted(&was_halted);
    if (!was_halted) {
        riscv_halt();
    }

    /* Load PROGBUF0 = sw s0, 0(a1) (0x0085a023), PROGBUF1 = ebreak (0x00100073) */
    if (current_progbuf0 != 0x0085a023) {
        riscv_dmi_write(DMI_ABSTRACTCS, ABSTRACTCS_ERR_MASK);
        riscv_dmi_write(DMI_PROGBUF0, 0x0085a023);
        riscv_dmi_write(DMI_PROGBUF1, 0x00100073);
        current_progbuf0 = 0x0085a023;
    }

    /* Set s0 (reg 0x1008) = val */
    riscv_dmi_write(DMI_DATA0, val);
    ch32_result_t res = riscv_dmi_write(DMI_COMMAND, CMD_TYPE_ACCESS_REG | CMD_TRANSFER | CMD_WRITE | CMD_AARSIZE_32 | CMD_REGNO(0x1008));
    if (res != CH32_OK) {
        if (!was_halted) riscv_resume();
        return res;
    }
    wait_abstract_cmd_complete();

    /* Set a1 (reg 0x100b) = addr */
    riscv_dmi_write(DMI_DATA0, addr);
    res = riscv_dmi_write(DMI_COMMAND, CMD_TYPE_ACCESS_REG | CMD_TRANSFER | CMD_WRITE | CMD_AARSIZE_32 | CMD_REGNO(0x100B));
    if (res != CH32_OK) {
        if (!was_halted) riscv_resume();
        return res;
    }
    wait_abstract_cmd_complete();

    /* Execute PROGBUF (postexec) */
    res = riscv_dmi_write(DMI_COMMAND, CMD_TYPE_ACCESS_REG | CMD_POSTEXEC);
    if (res != CH32_OK) {
        if (!was_halted) riscv_resume();
        return res;
    }
    res = wait_abstract_cmd_complete();

    if (!was_halted) {
        riscv_resume();
    }
    return res;
}

ch32_result_t riscv_write_hword(uint32_t addr, uint16_t val) {
    /* SBA check or PROGBUF fallback */
    bool was_halted = false;
    riscv_is_halted(&was_halted);
    if (!was_halted) {
        riscv_halt();
    }

    /* Load PROGBUF0 = sh s0, 0(a1) (0x00859023), PROGBUF1 = ebreak (0x00100073) */
    if (current_progbuf0 != 0x00859023) {
        riscv_dmi_write(DMI_ABSTRACTCS, ABSTRACTCS_ERR_MASK);
        riscv_dmi_write(DMI_PROGBUF0, 0x00859023);
        riscv_dmi_write(DMI_PROGBUF1, 0x00100073);
        current_progbuf0 = 0x00859023;
    }

    /* Set s0 (reg 0x1008) = val */
    riscv_dmi_write(DMI_ABSTRACTCS, ABSTRACTCS_ERR_MASK);
    riscv_dmi_write(DMI_DATA0, val);
    ch32_result_t res = riscv_dmi_write(DMI_COMMAND, CMD_TYPE_ACCESS_REG | CMD_TRANSFER | CMD_WRITE | CMD_AARSIZE_32 | CMD_REGNO(0x1008));
    if (res != CH32_OK) {
        if (!was_halted) riscv_resume();
        return res;
    }
    res = wait_abstract_cmd_complete();
    if (res != CH32_OK) {
        if (!was_halted) riscv_resume();
        return res;
    }

    /* Set a1 (reg 0x100b) = addr */
    riscv_dmi_write(DMI_ABSTRACTCS, ABSTRACTCS_ERR_MASK);
    riscv_dmi_write(DMI_DATA0, addr);
    res = riscv_dmi_write(DMI_COMMAND, CMD_TYPE_ACCESS_REG | CMD_TRANSFER | CMD_WRITE | CMD_AARSIZE_32 | CMD_REGNO(0x100B));
    if (res != CH32_OK) {
        if (!was_halted) riscv_resume();
        return res;
    }
    res = wait_abstract_cmd_complete();
    if (res != CH32_OK) {
        if (!was_halted) riscv_resume();
        return res;
    }

    /* Execute PROGBUF (postexec) */
    riscv_dmi_write(DMI_ABSTRACTCS, ABSTRACTCS_ERR_MASK);
    res = riscv_dmi_write(DMI_COMMAND, CMD_TYPE_ACCESS_REG | CMD_POSTEXEC);
    if (res != CH32_OK) {
        if (!was_halted) riscv_resume();
        return res;
    }
    res = wait_abstract_cmd_complete();

    if (!was_halted) {
        riscv_resume();
    }
    return res;
}

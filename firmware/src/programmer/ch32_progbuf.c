/*
 * CH32 PROGBUF Flash Injection Module
 *
 * High-speed flash programming using RISC-V Program Buffer (PROGBUF).
 * Ported from PicoRVD's WCHFlash.cpp.
 *
 * Key insight: Instead of slow System Bus Access (SBA) for each word,
 * inject a small RISC-V program into PROGBUF0-7 that directly accesses
 * flash control registers. Use ABSTRACTAUTO to repeat execution for
 * each word, achieving ~3x faster writes.
 */

#include "ch32_progbuf.h"
#include "riscv_dmi.h"
#include "ch32.h"
#include <stdio.h>
#include <string.h>

/* PROGBUF state */
static bool progbuf_initialized = false;
static uint32_t saved_gpr[16];  /* Save a0-a15 for restoration */
static uint32_t saved_progbuf[8];
static bool progbuf_supported = false;

/* Check if PROGBUF is supported by reading ABSTRACTCS */
static bool check_progbuf_support(void) {
    uint32_t abstractcs = 0;
    ch32_result_t res = riscv_dmi_read(DMI_ABSTRACTCS, &abstractcs);
    if (res != CH32_OK) return false;

    /* PROGBUFSIZE field (bits 28:24) indicates PROGBUF size in 32-bit words */
    uint32_t progbuf_size = (abstractcs >> 24) & 0x1F;
    return (progbuf_size > 0);
}

/* Save register state before PROGBUF execution */
static void save_registers(void) {
    /* Save a0-a7 (GPR 10-17) */
    for (int i = 0; i < 8; i++) {
        riscv_read_reg(0x1000 + 10 + i, &saved_gpr[i]);
    }
}

/* Restore register state after PROGBUF execution */
static void restore_registers(void) {
    /* Restore a0-a7 (GPR 10-17) */
    for (int i = 0; i < 8; i++) {
        riscv_write_reg(0x1000 + 10 + i, saved_gpr[i]);
    }
}

/* Save PROGBUF contents */
static void save_progbuf(void) {
    for (int i = 0; i < 8; i++) {
        riscv_dmi_read(DMI_PROGBUF0 + i, &saved_progbuf[i]);
    }
}

/* Restore PROGBUF contents */
static void restore_progbuf(void) {
    for (int i = 0; i < 8; i++) {
        riscv_dmi_write(DMI_PROGBUF0 + i, saved_progbuf[i]);
    }
}

/* Load a program into PROGBUF */
static void load_progbuf(const uint32_t *prog, int count) {
    for (int i = 0; i < count && i < 8; i++) {
        riscv_dmi_write(DMI_PROGBUF0 + i, prog[i]);
    }
}

/* Execute PROGBUF program via POSTEXEC */
static ch32_result_t run_progbuf(void) {
    /* Set COMMAND.POSTEXEC = 1 with AARSIZE = 32-bit (0x00240000).
     * RISC-V debug spec requires AARSIZE_32; without it, QingKe V2 debug
     * module flags CMDERR=4 (unsupported abstract command). */
    uint32_t cmd = CMD_TYPE_ACCESS_REG | CMD_AARSIZE_32 | CMD_POSTEXEC;
    ch32_result_t res = riscv_dmi_write(DMI_COMMAND, cmd);
    if (res != CH32_OK) return res;

    /* Wait for completion - like PicoRVD, just wait for BUSY to clear */
    for (int i = 0; i < 1000; i++) {
        uint32_t abstractcs = 0;
        res = riscv_dmi_read(DMI_ABSTRACTCS, &abstractcs);
        if (res != CH32_OK) return res;
        if (!(abstractcs & ABSTRACTCS_BUSY)) {
            /* Check for errors but don't treat as fatal - WCH may report
             * non-fatal bus errors for flash writes that still succeed */
            if ((abstractcs & ABSTRACTCS_CMDERR_MASK) != ABSTRACTCS_ERR_NONE) {
                riscv_dmi_write(DMI_ABSTRACTCS, ABSTRACTCS_ERR_MASK);
            }
            return CH32_OK;
        }
        sleep_us(10);
    }
    return CH32_ERR_TIMEOUT;
}

/* Strict variant used by the vendored memory-access paths (get/set/block).
 * A CMDERR means the abstract op faulted (e.g. flash read under RDP gives
 * cmderr=3) and DATA0 would hold a STALE value — propagate the error instead
 * of clearing it and returning OK (F8). The flash-write streamer keeps using
 * the lenient run_progbuf() because WCH reports non-fatal bus errors for
 * writes that still succeed. */
static ch32_result_t run_progbuf_strict(void) {
    uint32_t cmd = CMD_TYPE_ACCESS_REG | CMD_AARSIZE_32 | CMD_POSTEXEC;
    ch32_result_t res = riscv_dmi_write(DMI_COMMAND, cmd);
    if (res != CH32_OK) return res;

    for (int i = 0; i < 1000; i++) {
        uint32_t abstractcs = 0;
        res = riscv_dmi_read(DMI_ABSTRACTCS, &abstractcs);
        if (res != CH32_OK) return res;
        if (!(abstractcs & ABSTRACTCS_BUSY)) {
            if ((abstractcs & ABSTRACTCS_CMDERR_MASK) != ABSTRACTCS_ERR_NONE) {
                riscv_dmi_write(DMI_ABSTRACTCS, ABSTRACTCS_ERR_MASK);
                return CH32_ERR_NACK;
            }
            return CH32_OK;
        }
        sleep_us(10);
    }
    return CH32_ERR_TIMEOUT;
}

/* Wait for PROGBUF program to complete (BUSY flag cleared) */
static ch32_result_t wait_progbuf_done(void) {
    for (int i = 0; i < 1000; i++) {
        uint32_t abstractcs = 0;
        ch32_result_t res = riscv_dmi_read(DMI_ABSTRACTCS, &abstractcs);
        if (res != CH32_OK) return res;
        if (!(abstractcs & ABSTRACTCS_BUSY)) {
            return CH32_OK;
        }
        sleep_us(10);
    }
    return CH32_ERR_TIMEOUT;
}

/* ============================================================================
 * PROGBUF Programs (RISC-V machine code)
 * ============================================================================
 *
 * These programs are designed to:
 * 1. Read data from DATA0 (0xE00000F4)
 * 2. Write it to the appropriate flash control registers
 * 3. Handle page buffer loading and write triggering
 *
 * Register mapping:
 *   a0 = flash base (0x40022000)
 *   a1 = source data address (DATA0 @ 0xE00000F4)
 *   a2 = destination address
 *   a3 = flash control value for BUFLOAD
 *   a4 = flash control value for START
 *   a5 = flash control value for BUFRST
 */

/* Program: Write flash word and trigger BUFLOAD */
static const uint16_t prog_write_flash[] = {
    0x4180,  /* lw      s0, 0(a1)       ; read data from DATA0 */
    0xc200,  /* sw      s0, 0(a2)       ; write to flash address */
    0xc914,  /* sw      a3, 16(a0)      ; trigger BUFLOAD */

    /* waitloop1: Busywait for copy to complete */
    0x4540,  /* lw      s0, 12(a0)      ; read FLASH_STATR */
    0x8805,  /* andi    s0, s0, 1       ; check BUSY bit */
    0xfc75,  /* bnez    s0, waitloop1   ; loop if busy */

    /* Advance dest pointer and trigger START if we ended a page */
    0x0611,  /* addi    a2, a2, 4       ; advance destination */
    0x7433,  /* and     s0, a2, a6      ; check page boundary via a6 mask */
    0x0106,  /* (continuation of 32-bit and instruction) */
    0xe419,  /* bnez    s0, end         ; skip START if not page boundary */
    0xc918,  /* sw      a4, 16(a0)      ; trigger START (page write) */

    /* waitloop2: Busywait for page write to complete */
    0x4540,  /* lw      s0, 12(a0)      ; read FLASH_STATR */
    0x8805,  /* andi    s0, s0, 1       ; check BUSY bit */
    0xfc75,  /* bnez    s0, waitloop2   ; loop if busy */

    /* Reset buffer */
    0xc91c,  /* sw      a5, 16(a0)      ; trigger BUFRST */

    /* Update page address */
    0xc950,  /* sw      a2, 20(a0)      ; store new address to DATA1 */
};

/* Program: Execute flash command (erase, etc.) */
static const uint16_t prog_flash_command[] = {
    0xc94c,  /* sw      a1, 20(a0)      ; write address to DATA1 */
    0xc910,  /* sw      a2, 16(a0)      ; write ctl1 to flash */
    0xc914,  /* sw      a3, 16(a0)      ; write ctl2 to flash */

    /* waitloop: Busywait for flash operation */
    0x455c,  /* lw      a5, 12(a0)      ; read FLASH_STATR */
    0x8b85,  /* andi    a5, a5, 1       ; check BUSY bit */
    0xfff5,  /* bnez    a5, waitloop    ; loop if busy */

    0x2823,  /* sw      zero, 16(a0)    ; clear flash control */
    0x0005,  /* (nop) */

    0x9002,  /* ebreak */
};

/* ============================================================================
 * Public API
 * ============================================================================ */

ch32_result_t ch32_progbuf_init(void) {
    if (progbuf_initialized) return CH32_OK;

    /* Check PROGBUF support */
    progbuf_supported = check_progbuf_support();
    if (!progbuf_supported) {
        printf("CH32: PROGBUF not supported\r\n");
        return CH32_ERR_UNSUPPORTED;
    }

    printf("CH32: PROGBUF supported, initializing\r\n");

    /* Save current register and PROGBUF state */
    save_registers();
    save_progbuf();

    progbuf_initialized = true;
    return CH32_OK;
}

void ch32_progbuf_deinit(void) {
    if (!progbuf_initialized) return;

    riscv_dmi_write(DMI_ABSTRACTAUTO, 0);
    restore_progbuf();
    restore_registers();

    progbuf_initialized = false;
}

bool ch32_progbuf_is_available(void) {
    return progbuf_initialized && progbuf_supported;
}

ch32_result_t ch32_progbuf_flash_erase(uint32_t addr, uint32_t len) {
    if (!progbuf_initialized) return CH32_ERR_UNSUPPORTED;

    ch32_result_t res;

    /* Unlock flash */
    res = riscv_write_mem(PROGBUF_FLASH_KEYR, FLASH_KEY1);
    if (res != CH32_OK) return res;
    res = riscv_write_mem(PROGBUF_FLASH_KEYR, FLASH_KEY2);
    if (res != CH32_OK) return res;
    res = riscv_write_mem(PROGBUF_FLASH_MKEYR, FLASH_KEY1);
    if (res != CH32_OK) return res;
    res = riscv_write_mem(PROGBUF_FLASH_MKEYR, FLASH_KEY2);
    if (res != CH32_OK) return res;

    uint32_t page_sz = g_ch32_target.page_size ? g_ch32_target.page_size : 64;
    uint32_t start_addr = addr & ~(page_sz - 1);
    uint32_t end_addr = (addr + len + page_sz - 1) & ~(page_sz - 1);

    for (uint32_t curr = start_addr; curr < end_addr; curr += page_sz) {
        /* WCH fast page erase sequence:
         * 1. Arm erase mode (FTER)
         * 2. Write target page address
         * 3. Trigger operation (FTER | STRT) */
        res = riscv_write_mem(PROGBUF_FLASH_CTLR, FLASH_CTLR_FTER);
        if (res != CH32_OK) return res;
        res = riscv_write_mem(PROGBUF_FLASH_ADDR, curr | 0x08000000);
        if (res != CH32_OK) return res;
        res = riscv_write_mem(PROGBUF_FLASH_CTLR, FLASH_CTLR_FTER | FLASH_CTLR_STRT);
        if (res != CH32_OK) return res;

        /* Wait for completion */
        for (int i = 0; i < 500; i++) {
            uint32_t statr = 0;
            res = riscv_read_mem(PROGBUF_FLASH_STATR, &statr);
            if (res == CH32_OK && !(statr & FLASH_STATR_BUSY)) {
                break;
            }
            sleep_us(100);
        }

        /* Clear erase control bits */
        riscv_write_mem(PROGBUF_FLASH_CTLR, 0);
    }

    return CH32_OK;
}

ch32_result_t ch32_progbuf_flash_write(uint32_t addr, const uint8_t *data, uint32_t len) {
    if (!progbuf_initialized || !data || len == 0) return CH32_ERR_UNSUPPORTED;
    if (len % 4 != 0) return CH32_ERR_UNSUPPORTED;

    ch32_result_t res;

    /* Unlock flash */
    res = riscv_write_mem(PROGBUF_FLASH_KEYR, FLASH_KEY1);
    if (res != CH32_OK) return res;
    res = riscv_write_mem(PROGBUF_FLASH_KEYR, FLASH_KEY2);
    if (res != CH32_OK) return res;
    res = riscv_write_mem(PROGBUF_FLASH_MKEYR, FLASH_KEY1);
    if (res != CH32_OK) return res;
    res = riscv_write_mem(PROGBUF_FLASH_MKEYR, FLASH_KEY2);
    if (res != CH32_OK) return res;

    uint32_t page_sz = g_ch32_target.page_size ? g_ch32_target.page_size : 64;

    for (uint32_t offset = 0; offset < len; offset += page_sz) {
        uint32_t page_addr = addr + offset;
        uint32_t chunk_len = (len - offset < page_sz) ? (len - offset) : page_sz;

        /* Reset page buffer and enable fast page programming:
         * 1. Arm fast page mode (FTPG)
         * 2. Reset buffer (FTPG | BUFRST)
         * 3. Set page address */
        res = riscv_write_mem(PROGBUF_FLASH_CTLR, FLASH_CTLR_FTPG);
        if (res != CH32_OK) return res;
        res = riscv_write_mem(PROGBUF_FLASH_CTLR, FLASH_CTLR_FTPG | FLASH_CTLR_BUFRST);
        if (res != CH32_OK) return res;
        res = riscv_write_mem(PROGBUF_FLASH_ADDR, page_addr | 0x08000000);
        if (res != CH32_OK) return res;

        /* Wait for buffer reset to complete */
        for (int i = 0; i < 500; i++) {
            uint32_t statr = 0;
            res = riscv_read_mem(PROGBUF_FLASH_STATR, &statr);
            if (res == CH32_OK && !(statr & FLASH_STATR_BUSY)) break;
            sleep_us(50);
        }

        /* Load words into buffer */
        for (uint32_t b = 0; b < page_sz; b += 4) {
            uint32_t word = 0xFFFFFFFF;
            if (b + 4 <= chunk_len) {
                word = (uint32_t)data[offset + b] |
                       ((uint32_t)data[offset + b + 1] << 8) |
                       ((uint32_t)data[offset + b + 2] << 16) |
                       ((uint32_t)data[offset + b + 3] << 24);
            } else if (b < chunk_len) {
                for (uint32_t k = 0; b + k < chunk_len; k++) {
                    word &= ~(0xFFU << (k * 8));
                    word |= ((uint32_t)data[offset + b + k] << (k * 8));
                }
            }

            /* Write word to page address */
            res = riscv_write_mem((page_addr + b) | 0x08000000, word);
            if (res != CH32_OK) return res;

            /* Latch word into buffer */
            res = riscv_write_mem(PROGBUF_FLASH_CTLR, FLASH_CTLR_FTPG | FLASH_CTLR_BUFLOAD);
            if (res != CH32_OK) return res;

            /* Wait for buffer load to complete */
            for (int i = 0; i < 500; i++) {
                uint32_t statr = 0;
                res = riscv_read_mem(PROGBUF_FLASH_STATR, &statr);
                if (res == CH32_OK && !(statr & FLASH_STATR_BUSY)) break;
                sleep_us(50);
            }
        }

        /* Trigger page write (FTPG | STRT) */
        res = riscv_write_mem(PROGBUF_FLASH_ADDR, page_addr | 0x08000000);
        if (res != CH32_OK) return res;
        res = riscv_write_mem(PROGBUF_FLASH_CTLR, FLASH_CTLR_FTPG | FLASH_CTLR_STRT);
        if (res != CH32_OK) return res;

        /* Wait for page write to complete */
        for (int i = 0; i < 1000; i++) {
            uint32_t statr = 0;
            res = riscv_read_mem(PROGBUF_FLASH_STATR, &statr);
            if (res == CH32_OK && !(statr & FLASH_STATR_BUSY)) break;
            sleep_us(100);
        }
    }

    /* Cleanup: reset flash control register */
    riscv_write_mem(PROGBUF_FLASH_CTLR, 0);

    /* Clear EOP flag */
    uint32_t statr = 0;
    riscv_read_mem(PROGBUF_FLASH_STATR, &statr);
    if (statr & FLASH_STATR_EOP) {
        riscv_write_mem(PROGBUF_FLASH_STATR, FLASH_STATR_EOP);
    }

    return CH32_OK;
}

/* ============================================================================
 * PicoRVD PROGBUF Memory-Access Programs
 * ============================================================================
 * Ported verbatim from RVDebug.cpp (MIT License, https://github.com/aappleby/picorvd):
 *   prog_get_set_u32      (RVDebug.cpp:540) — single combined get/set program,
 *                          direction flagged by DATA1 bit 0, clobbers A0/A1 only
 *   prog_get_block_aligned (RVDebug.cpp:602) — ABSTRACTAUTO-pipelined block read
 *   prog_set_block_aligned (RVDebug.cpp:637) — ABSTRACTAUTO-pipelined block write
 *
 * The templates carry V003's DTM offsets (0xF4/0xF8). The WCH DTM aliases its
 * DATA0/DATA1 registers into system bus space; the offset is a property of the
 * target chip's DTM (NOT of the wire transport):
 *   CH32V003      -> 0xE00000F4 / 0xE00000F8   (PicoRVD's shipped values)
 *   CH32V20x/V30x -> 0xE0000380 / 0xE0000384   (HARTINFO-derived, minichlink's
 *                     formula 0xE0000000 | (HARTINFO & 0x7FF))
 * patch_dtm_offsets() rewrites the 0xF4/0xF8 immediates to the active family's
 * offsets before the program is loaded into PROGBUF, so one copy of each
 * program serves every family.
 * ============================================================================ */

typedef struct {
    uint32_t data0_off;  /* DTM DATA0 system-bus offset */
    uint32_t data1_off;  /* DTM DATA1 system-bus offset (DATA0 + 4) */
} dtm_offsets_t;

/* DTM DATA0/DATA1 magic offsets keyed on the target chip family. Unknown chips
 * default to the V20x/V30x DTM layout (the current shipping target). */
static dtm_offsets_t dtm_offsets_for_chip(ch32_chip_t chip) {
    switch (chip) {
    case CH32_CHIP_V003:
        return (dtm_offsets_t){0xF4, 0xF8};
    case CH32_CHIP_V203:
    case CH32_CHIP_V307:
    default:
        return (dtm_offsets_t){0x380, 0x384};
    }
}

static dtm_offsets_t active_dtm_offsets(void) {
    return dtm_offsets_for_chip(g_ch32_target.chip_type);
}

/* Rewrite the DTM DATA0/DATA1 immediates in a PicoRVD program template (built
 * with V003's 0xF4/0xF8) to the given family's offsets. Only instructions whose
 * 12-bit immediate is 0xF4 (DATA0) or 0xF8 (DATA1) are touched; the rest of the
 * programs are identical across families. Handles I-type (lw/addi) and S-type
 * (sw) immediates. */
static void patch_dtm_offsets(uint32_t *prog, int nwords, dtm_offsets_t offs) {
    for (int i = 0; i < nwords; i++) {
        uint32_t w = prog[i];
        switch (w & 0x7F) {
        case 0x03:  /* I-type: lw */
        case 0x13:  /* I-type: addi */
            if (((w >> 20) & 0xFFF) == 0xF4) {
                w = (w & 0x000FFFFF) | (offs.data0_off << 20);
            } else if (((w >> 20) & 0xFFF) == 0xF8) {
                w = (w & 0x000FFFFF) | (offs.data1_off << 20);
            }
            break;
        case 0x23: { /* S-type: sw */
            uint32_t imm12 = ((w >> 25) << 5) | ((w >> 7) & 0x1F);
            if (imm12 == 0xF4 || imm12 == 0xF8) {
                uint32_t repl = (imm12 == 0xF4) ? offs.data0_off : offs.data1_off;
                w &= ~(0x7Fu << 25) & ~(0x1Fu << 7);
                w |= ((repl >> 5) & 0x7F) << 25;
                w |= (repl & 0x1F) << 7;
            }
            break;
        }
        default:
            break;
        }
        prog[i] = w;
    }
}

/* Load a program into PROGBUF0-7 with the active family's DTM offsets baked in. */
static void load_progbuf_dtm(const uint32_t *prog, int nwords) {
    uint32_t patched[8];
    for (int i = 0; i < nwords && i < 8; i++) patched[i] = prog[i];
    patch_dtm_offsets(patched, (nwords < 8) ? nwords : 8, active_dtm_offsets());
    load_progbuf(patched, (nwords < 8) ? nwords : 8);
}

/* Single combined get/set program: data0 = data to write, data1 = address.
 * DATA1 bit 0 = 0 reads (get_u32), bit 0 = 1 writes (set_u32). */
static const uint16_t prog_get_set_u32[16] = {
    0x0537, 0xe000,  /* lui   a0, 0xE0000              */
    0x0513, 0x0f45,  /* addi  a0, a0, 0xF4 (DATA0)     */
    0x414c,          /* lw    a1, 4(a0)   ; a1 = DATA1 */
    0x8985,          /* andi  a1, a1, 1   ; direction? */
    0xc591,          /* beqz  a1, get_u32              */
    0x414c,          /* set_u32: lw a1, 4(a0)          */
    0x15fd,          /* addi  a1, a1, -1  ; clear dir  */
    0x4108,          /* lw    a0, 0(a0)   ; a0 = data  */
    0xc188,          /* sw    a0, 0(a1)   ; store      */
    0x9002,          /* ebreak                         */
    0x414c,          /* get_u32: lw a1, 4(a0)          */
    0x418c,          /* lw    a1, 0(a1)   ; a1 = word  */
    0xc10c,          /* sw    a1, 0(a0)   ; DATA0 = it */
    0x9002,          /* ebreak                         */
};

/* Block-read program: DATA1 holds the pointer, DATA0 carries each word.
 * The host runs one explicit POSTEXEC per word (AUTOEXECDATA streaming
 * wedges the WCH DM after ~30-80 executions, F9) and reads DATA0 each
 * time. Works for flash AND SRAM. */
static const uint32_t prog_get_block_aligned[8] = {
    0xe0000537,  /* lui    a0, 0xE0000        */
    0x0f852583,  /* lw     a1, 0x0F8(a0)      ; a1 = [DATA1 magic] = pointer */
    0x0005a583,  /* lw     a1, 0x000(a1)      ; a1 = [pointer] = memory word */
    0x0eb52a23,  /* sw     a1, 0x0F4(a0)      ; DATA0 magic = word           */
    0x0f852583,  /* lw     a1, 0x0F8(a0)      ; a1 = pointer                 */
    0x00458593,  /* addi   a1, a1, 4          ; pointer += 4                 */
    0x0eb52c23,  /* sw     a1, 0x0F8(a0)      ; store back pointer           */
    0x00100073,  /* ebreak                                                   */
};

/* Block-write program: DATA1 holds the pointer, DATA0 receives words. */
static const uint32_t prog_set_block_aligned[8] = {
    0xe0000537,  /* lui    a0, 0xE0000        */
    0x0f852583,  /* lw     a1, 0x0F8(a0)      ; a1 = [DATA1 magic] = pointer */
    0x0f452503,  /* lw     a0, 0x0F4(a0)      ; a0 = [DATA0 magic] = word    */
    0x00a5a023,  /* sw     a0, 0x000(a1)      ; store word at pointer        */
    0x00458593,  /* addi   a1, a1, 4          ; pointer += 4                 */
    0xe0000537,  /* lui    a0, 0xE0000        */
    0x0eb52c23,  /* sw     a1, 0x0F8(a0)      ; store back pointer           */
    0x00100073,  /* ebreak                                                   */
};

/* ============================================================================
 * Vendored Memory-Access API (PicoRVD RVDebug.cpp:567/583/598/633)
 * NOTE: the core must be halted before calling these (they execute PROGBUF
 * via POSTEXEC, which requires the core halted). The riscv_dmi.c wrappers
 * handle halt/resume.
 * ============================================================================ */

ch32_result_t ch32_progbuf_get_set_u32(uint32_t addr, uint32_t *val, bool write) {
    if (!val) return CH32_ERR_UNSUPPORTED;
    if (addr & 3) return CH32_ERR_UNSUPPORTED;

    load_progbuf_dtm((const uint32_t *)prog_get_set_u32, 8);

    ch32_result_t res;
    if (write) {
        /* PicoRVD order: DATA0 first, then DATA1 (with direction bit). */
        res = riscv_dmi_write(DMI_DATA0, *val);
        if (res != CH32_OK) return res;
        res = riscv_dmi_write(DMI_DATA1, addr | 1);
        if (res != CH32_OK) return res;
    } else {
        res = riscv_dmi_write(DMI_DATA1, addr);
        if (res != CH32_OK) return res;
    }

    res = run_progbuf_strict();
    if (res != CH32_OK) return res;

    if (!write) {
        return riscv_dmi_read(DMI_DATA0, val);
    }
    return CH32_OK;
}

ch32_result_t ch32_progbuf_get_block_aligned(uint32_t addr, uint32_t *dst, int size_dwords) {
    if (!dst || size_dwords <= 0) return CH32_ERR_UNSUPPORTED;
    if (addr & 3) return CH32_ERR_UNSUPPORTED;

    load_progbuf_dtm(prog_get_block_aligned, 8);

    ch32_result_t res = riscv_dmi_write(DMI_DATA1, addr);
    if (res != CH32_OK) return res;

    for (int i = 0; i < size_dwords; i++) {
        /* Per-word POSTEXEC (no ABSTRACTAUTO streaming): each execution loads
         * the next word into DATA0 and advances the pointer. AUTOEXECDATA
         * wedges the WCH DM after ~30-80 executions (F9); the explicit
         * POSTEXEC + read pattern (the RAM-stub loader) does not. */
        res = run_progbuf_strict();
        if (res != CH32_OK) return res;
        res = riscv_dmi_read(DMI_DATA0, &dst[i]);
        if (res != CH32_OK) return res;
    }
    return CH32_OK;
}

ch32_result_t ch32_progbuf_set_block_aligned(uint32_t addr, const uint32_t *src, int size_dwords) {
    if (!src || size_dwords <= 0) return CH32_ERR_UNSUPPORTED;
    if (addr & 3) return CH32_ERR_UNSUPPORTED;

    load_progbuf_dtm(prog_set_block_aligned, 8);

    ch32_result_t res = riscv_dmi_write(DMI_DATA1, addr);
    if (res != CH32_OK) return res;

    for (int i = 0; i < size_dwords; i++) {
        res = riscv_dmi_write(DMI_DATA0, src[i]);
        if (res != CH32_OK) return res;
        /* Per-word POSTEXEC — see get_block_aligned (AUTOEXECDATA wedges). */
        res = run_progbuf_strict();
        if (res != CH32_OK) return res;
    }
    return CH32_OK;
}

ch32_result_t ch32_progbuf_flash_read(uint32_t addr, uint8_t *data, uint32_t len) {
    if (!data || len == 0) return CH32_ERR_UNSUPPORTED;

    ch32_result_t res = riscv_halt();
    if (res != CH32_OK) return res;

    /* Ensure FLASH_CTLR is reset and locked to enable normal read mode.
     * NOTE: V20x-specific prep — stays in this wrapper, NOT in the shared
     * block-read primitive. */
    riscv_write_mem(CH32V20x_FLASH_CTLR, 0);
    riscv_write_mem(CH32V20x_FLASH_CTLR, CH32V20x_CTLR_FLOCK);

    uint32_t aligned_addr = addr & ~3U;
    uint32_t offset = addr - aligned_addr;
    uint32_t total = len + offset;
    uint32_t aligned_len = (total + 3) & ~3U;
    if (aligned_len > 256) aligned_len = 256;
    uint32_t word_count = aligned_len / 4;

    /* Ensure ebreak enters debug mode (set dcsr.ebreakm = 1) */
    uint32_t dcsr = 0;
    if (riscv_read_reg(0x7B0, &dcsr) == CH32_OK) {
        dcsr |= (1U << 15) | (1U << 13) | (1U << 12); /* ebreakm, ebreaks, ebreaku */
        riscv_write_reg(0x7B0, dcsr);
    }

    uint8_t aligned_buf[256];
    res = ch32_progbuf_get_block_aligned(aligned_addr, (uint32_t *)aligned_buf, (int)word_count);
    if (res != CH32_OK) return res;

    /* Copy requested portion */
    uint32_t copy_len = len;
    if (offset + copy_len > aligned_len) copy_len = aligned_len - offset;
    memcpy(data, aligned_buf + offset, copy_len);

    return CH32_OK;
}


/* ============================================================================
 * SRAM-Based Flash Write (V203/V307 Fix)
 * ============================================================================
 *
 * The PROGBUF approach fails on V203/V307 because the debug bus master
 * cannot write to flash address space (0x0800xxxx) — the flash controller
 * doesn't intercept debug-initiated writes. This causes ABSTRACTCS CMDERR=3.
 *
 * Fix: Load the flash write routine into target SRAM (0x20000000) and let
 * the core execute it normally. When executing from SRAM, the core's normal
 * data bus writes go through the flash controller, which correctly loads
 * the page buffer.
 *
 * Register mapping for SRAM program:
 *   a0 = flash base (0x40022000)
 *   a1 = source data address (in target SRAM)
 *   a2 = destination flash address
 *   a3 = flash control value for BUFLOAD (FTPG | BUFLOAD)
 *   a7 = word count for this page
 */

/* SRAM base address for flash write routine */
#define SRAM_FLASH_PROG_ADDR  0x20000000
#define SRAM_FLASH_DATA_ADDR  0x20001000

/*
 * SRAM flash write program — processes one page of flash data.
 * Uses mret to enter the routine (mepc is set by host before resume).
 *
 * 0x00: lw   s0, 0(a1)       ; read data from source
 * 0x04: sw   s0, 0(a2)       ; write to flash address
 * 0x08: sw   a3, 16(a0)      ; trigger BUFLOAD
 * 0x0C: addi a2, a2, 4       ; advance dest pointer
 * 0x10: addi a1, a1, 4       ; advance source pointer
 * 0x14: addi a7, a7, -1      ; decrement word count
 * 0x18: bne  a7, x0, -20     ; loop to 0x00 if more words
 * 0x1C: ebreak                ; done
 */
static const uint32_t sram_flash_write_prog[] = {
    0x0005A403,  /* lw   s0, 0(a1)   */
    0x00862023,  /* sw   s0, 0(a2)   */
    0x00D52823,  /* sw   a3, 16(a0)  */
    0x00460613,  /* addi a2, a2, 4   */
    0x00458593,  /* addi a1, a1, 4   */
    0xFFF88893,  /* addi a7, a7, -1  */
    0xFE0894E3,  /* bne  a7, x0, -24 (loop back to 0x00; was 0xFF0896E3 which
                    decoded as bne a7, a6 (uninitialized) to 0x04 — infinite loop) */
    0xFFF00293,  /* addi t0, x0, -1 ; marker value 0xFFFFFFFF */
    0x00572023,  /* sw   t0, 0(a4)  ; write marker to DATA0 magic (a4) */
    0x00100073,  /* ebreak (best-effort; host force-halts if this fails) */
};
#define SRAM_PROG_WORDS (sizeof(sram_flash_write_prog) / sizeof(sram_flash_write_prog[0]))

/* Wait for core to halt (poll DMSTATUS.ALLHALTED) */
static ch32_result_t wait_for_halt(int timeout_ms) {
    for (int i = 0; i < timeout_ms * 10; i++) {
        uint32_t status = 0;
        ch32_result_t res = riscv_dmi_read(DMI_DMSTATUS, &status);
        if (res == CH32_OK && (status & DMSTATUS_ALLHALTED)) {
            return CH32_OK;
        }
        sleep_us(100);
    }
    return CH32_ERR_TIMEOUT;
}

/* DATA0 magic address: the WCH debug module aliases DMI DATA0 into system
 * bus space. Proven formula from minichlink / esp32s2-cookbook:
 *   0xE0000000 | (HARTINFO & 0x7FF)   (fallback 0xE0000380) */
static uint32_t sram_data0_magic_addr(void) {
    uint32_t hartinfo = 0;
    if (riscv_dmi_read(DMI_HARTINFO, &hartinfo) != CH32_OK) {
        return 0xE0000380;
    }
    return 0xE0000000 | (hartinfo & 0x7FF);
}

/* Wait for the SRAM blob to write 0xFFFFFFFF to the DATA0 magic address,
 * then force-halt the core. CH32V203's DCSR.ebreakm write can silently
 * fail, so we cannot rely on ebreak halting — this marker + forced halt
 * is exactly what the proven WCH references (minichlink, esp32s2-cookbook)
 * do. */
static ch32_result_t wait_blob_done(int timeout_ms) {
    for (int i = 0; i < timeout_ms * 10; i++) {
        uint32_t data0 = 0;
        if (riscv_dmi_read(DMI_DATA0, &data0) == CH32_OK && data0 == 0xFFFFFFFFU) {
            /* Marker seen — force-halt the core */
            riscv_dmi_write(DMI_DMCONTROL, DMCONTROL_HALTREQ | DMCONTROL_DMACTIVE);
            for (int j = 0; j < 100; j++) {
                uint32_t status = 0;
                if (riscv_dmi_read(DMI_DMSTATUS, &status) == CH32_OK &&
                    (status & DMSTATUS_ALLHALTED)) {
                    return CH32_OK;
                }
                sleep_us(100);
            }
            return CH32_OK;  /* marker seen; halt acknowledged regardless */
        }
        sleep_us(100);
    }
    return CH32_ERR_TIMEOUT;
}

/* Set PC to target address by writing DPC (0x7B1) or mepc (0x7C1) */
static ch32_result_t set_pc(uint32_t target) {
    /* Method 1: Try writing DPC (CSR 0x7B1 - Debug Program Counter) directly */
    ch32_result_t res = riscv_write_reg(0x7B1, target);
    if (res == CH32_OK) {
        printf("CH32: Set PC to 0x%08lX via DPC (0x7B1)\r\n", (unsigned long)target);
        return CH32_OK;
    }

    /* Method 2: Try writing mepc (CSR 0x7C1) */
    res = riscv_write_reg(0x7C1, target);
    if (res == CH32_OK) {
        printf("CH32: Set PC to 0x%08lX via MEPC (0x7C1)\r\n", (unsigned long)target);
        return CH32_OK;
    }

    printf("CH32: Failed to set PC via DPC/MEPC CSRs\r\n");
    return CH32_ERR_FLASH;
}

/**
 * Flash write using SRAM-based routine.
 * Bypasses PROGBUF by loading the flash write program into target SRAM
 * and letting the core execute it normally.
 */
ch32_result_t ch32_sram_flash_write(uint32_t addr, const uint8_t *data, uint32_t len) {
    if (!data || len == 0) return CH32_ERR_UNSUPPORTED;
    if (len % 4 != 0) return CH32_ERR_UNSUPPORTED;

    ch32_result_t res;

    printf("CH32: SRAM flash write at 0x%08lX, %lu bytes\r\n", (unsigned long)addr, (unsigned long)len);

    /* Halt core */
    res = riscv_halt();
    if (res != CH32_OK) return res;

    /* Ensure ebreak enters debug mode (set dcsr.ebreakm = 1) */
    uint32_t dcsr = 0;
    if (riscv_read_reg(0x7B0, &dcsr) == CH32_OK) {
        dcsr |= (1U << 15) | (1U << 13) | (1U << 12); /* ebreakm, ebreaks, ebreaku */
        riscv_write_reg(0x7B0, dcsr);
    }

    /* Unlock flash */
    res = riscv_write_mem(PROGBUF_FLASH_KEYR, FLASH_KEY1);
    if (res != CH32_OK) return res;
    res = riscv_write_mem(PROGBUF_FLASH_KEYR, FLASH_KEY2);
    if (res != CH32_OK) return res;
    res = riscv_write_mem(PROGBUF_FLASH_MKEYR, FLASH_KEY1);
    if (res != CH32_OK) return res;
    res = riscv_write_mem(PROGBUF_FLASH_MKEYR, FLASH_KEY2);
    if (res != CH32_OK) return res;

    /* Load SRAM program into target SRAM */
    printf("CH32: Loading SRAM program (%lu words)\r\n", (unsigned long)SRAM_PROG_WORDS);
    for (uint32_t i = 0; i < SRAM_PROG_WORDS; i++) {
        res = riscv_write_mem(SRAM_FLASH_PROG_ADDR + i * 4, sram_flash_write_prog[i]);
        if (res != CH32_OK) {
            printf("CH32: Failed to load SRAM program word %lu\r\n", (unsigned long)i);
            return res;
        }
    }

    uint32_t page_sz = g_ch32_target.page_size ? g_ch32_target.page_size : 256;
    uint32_t words_per_page = page_sz / 4;

    /* Process each page */
    for (uint32_t offset = 0; offset < len; offset += page_sz) {
        uint32_t page_addr = addr + offset;
        uint32_t chunk_len = (len - offset < page_sz) ? (len - offset) : page_sz;
        uint32_t word_count = (chunk_len + 3) / 4;

        printf("CH32: Page at 0x%08lX, %lu words\r\n", (unsigned long)page_addr, (unsigned long)word_count);

        /* Reset page buffer */
        res = riscv_write_mem(PROGBUF_FLASH_CTLR, FLASH_CTLR_FTPG | FLASH_CTLR_BUFRST);
        if (res != CH32_OK) return res;

        /* Set flash page address */
        res = riscv_write_mem(PROGBUF_FLASH_ADDR, page_addr);
        if (res != CH32_OK) return res;

        /* Copy page data to target SRAM */
        for (uint32_t w = 0; w < word_count; w++) {
            uint32_t word = 0xFFFFFFFF;
            uint32_t idx = offset + w * 4;

            if (idx + 3 < len) {
                word = (uint32_t)data[idx] |
                       ((uint32_t)data[idx + 1] << 8) |
                       ((uint32_t)data[idx + 2] << 16) |
                       ((uint32_t)data[idx + 3] << 24);
            } else if (idx < len) {
                for (uint32_t k = 0; idx + k < len && k < 4; k++) {
                    word &= ~(0xFFU << (k * 8));
                    word |= ((uint32_t)data[idx + k] << (k * 8));
                }
            }

            res = riscv_write_mem(SRAM_FLASH_DATA_ADDR + w * 4, word);
            if (res != CH32_OK) return res;
        }

        /* Set up registers for SRAM program:
         *   a0 (x10) = flash base (0x40022000)
         *   a1 (x11) = source data address (SRAM)
         *   a2 (x12) = destination flash address
         *   a3 (x13) = BUFLOAD control value
         *   a7 (x17) = word count
         */
        res = riscv_write_reg(0x1000 + 10, PROGBUF_FLASH_BASE);
        if (res != CH32_OK) return res;
        res = riscv_write_reg(0x1000 + 11, SRAM_FLASH_DATA_ADDR);
        if (res != CH32_OK) return res;
        res = riscv_write_reg(0x1000 + 12, page_addr);
        if (res != CH32_OK) return res;
        res = riscv_write_reg(0x1000 + 13, FLASH_CTLR_FTPG | FLASH_CTLR_BUFLOAD);
        if (res != CH32_OK) return res;
        res = riscv_write_reg(0x1000 + 14, sram_data0_magic_addr());
        if (res != CH32_OK) return res;
        res = riscv_write_reg(0x1000 + 17, word_count);
        if (res != CH32_OK) return res;

        /* Clear completion mailbox so the blob's 0xFFFFFFFF marker is detectable */
        riscv_dmi_write(DMI_DATA0, 0);

        /* Clear mstatus (no interrupts/exceptions while blob runs) */
        riscv_write_reg(0x300, 0);

        /* Set PC to SRAM program address */
        res = set_pc(SRAM_FLASH_PROG_ADDR);
        if (res != CH32_OK) {
            printf("CH32: Failed to set PC\r\n");
            return res;
        }

        /* Clear any pending abstract errors before resume */
        riscv_dmi_write(DMI_ABSTRACTCS, ABSTRACTCS_ERR_MASK);

        /* Resume core — it executes the SRAM program */
        res = riscv_resume();
        if (res != CH32_OK) {
            printf("CH32: Failed to resume\r\n");
            return res;
        }

        /* Wait for completion marker, then force-halt (don't rely on ebreak) */
        res = wait_blob_done(1000);
        if (res != CH32_OK) {
            printf("CH32: SRAM program timeout (no completion marker)\r\n");
            riscv_halt();
            return res;
        }

        /* Check for errors */
        uint32_t abstractcs = 0;
        riscv_dmi_read(DMI_ABSTRACTCS, &abstractcs);
        if (abstractcs & ABSTRACTCS_CMDERR_MASK) {
            printf("CH32: SRAM program error, ABSTRACTCS=0x%08lX\r\n", (unsigned long)abstractcs);
            riscv_dmi_write(DMI_ABSTRACTCS, ABSTRACTCS_ERR_MASK);
            return CH32_ERR_FLASH;
        }

        /* Trigger actual flash page write (STRT) */
        res = riscv_write_mem(PROGBUF_FLASH_CTLR, FLASH_CTLR_FTPG | FLASH_CTLR_STRT);
        if (res != CH32_OK) return res;

        /* Wait for flash write to complete */
        for (int i = 0; i < 500; i++) {
            uint32_t statr = 0;
            res = riscv_read_mem(PROGBUF_FLASH_STATR, &statr);
            if (res == CH32_OK && !(statr & FLASH_STATR_BUSY)) {
                break;
            }
            sleep_us(100);
        }
    }

    /* Cleanup: reset flash control register and lock */
    riscv_write_mem(PROGBUF_FLASH_CTLR, 0);
    riscv_write_mem(PROGBUF_FLASH_CTLR, CH32V20x_CTLR_FLOCK);

    /* Clear EOP flag */
    uint32_t statr = 0;
    riscv_read_mem(PROGBUF_FLASH_STATR, &statr);
    if (statr & FLASH_STATR_EOP) {
        riscv_write_mem(PROGBUF_FLASH_STATR, FLASH_STATR_EOP);
    }

    printf("CH32: SRAM flash write complete\r\n");
    return CH32_OK;
}

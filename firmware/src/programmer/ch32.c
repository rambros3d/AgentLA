#include "ch32.h"
#include "ch32_progbuf.h"
#include "swd.h"
#include "swio.h"
#include "rvswd.h"
#include "hardware/gpio.h"
#include "hardware/clocks.h"
#include "wch_sdi.pio.h"
#include <stdio.h>
#include <string.h>

ch32_target_info_t g_ch32_target = {0};
static bool sdi_initialized = false;

ch32_result_t ch32_sdi_init(void) {
    if (sdi_initialized) return CH32_OK;

    gpio_init(WCH_SDI_PIN);
    gpio_set_dir(WCH_SDI_PIN, GPIO_IN);
    gpio_pull_up(WCH_SDI_PIN);

    sdi_initialized = true;
    return CH32_OK;
}

void ch32_sdi_deinit(void) {
    if (!sdi_initialized) return;
    gpio_set_dir(WCH_SDI_PIN, GPIO_IN);
    sdi_initialized = false;
}

void ch32_sdi_reset(void) {
    ch32_sdi_init();
    
    /* Drive LOW for >10 us */
    gpio_set_dir(WCH_SDI_PIN, GPIO_OUT);
    gpio_put(WCH_SDI_PIN, 0);
    sleep_us(15);

    /* Release HIGH for >2 us */
    gpio_put(WCH_SDI_PIN, 1);
    gpio_set_dir(WCH_SDI_PIN, GPIO_IN);
    sleep_us(5);
}

void ch32_sdi_send_bit(uint8_t bit) {
    gpio_set_dir(WCH_SDI_PIN, GPIO_OUT);
    if (bit & 1) {
        /* Bit 1: Low 0.25us, High 0.75us */
        gpio_put(WCH_SDI_PIN, 0);
        busy_wait_us_32(1);
        gpio_put(WCH_SDI_PIN, 1);
        busy_wait_us_32(3);
    } else {
        /* Bit 0: Low 0.75us, High 0.25us */
        gpio_put(WCH_SDI_PIN, 0);
        busy_wait_us_32(3);
        gpio_put(WCH_SDI_PIN, 1);
        busy_wait_us_32(1);
    }
    gpio_set_dir(WCH_SDI_PIN, GPIO_IN);
}

uint8_t ch32_sdi_read_bit(void) {
    /* Trigger pulse: Drive low for 0.25us, then sample pin level */
    gpio_set_dir(WCH_SDI_PIN, GPIO_OUT);
    gpio_put(WCH_SDI_PIN, 0);
    busy_wait_us_32(1);
    gpio_set_dir(WCH_SDI_PIN, GPIO_IN);
    busy_wait_us_32(2);
    uint8_t val = gpio_get(WCH_SDI_PIN) ? 1 : 0;
    busy_wait_us_32(1);
    return val;
}

void ch32_sdi_send_byte(uint8_t byte) {
    for (int i = 0; i < 8; i++) {
        ch32_sdi_send_bit((byte >> i) & 1);
    }
}

uint8_t ch32_sdi_read_byte(void) {
    uint8_t val = 0;
    for (int i = 0; i < 8; i++) {
        val |= (ch32_sdi_read_bit() << i);
    }
    return val;
}

void ch32_sdi_send_word(uint32_t word) {
    for (int i = 0; i < 4; i++) {
        ch32_sdi_send_byte((word >> (i * 8)) & 0xFF);
    }
}

uint32_t ch32_sdi_read_word(void) {
    uint32_t word = 0;
    for (int i = 0; i < 4; i++) {
        word |= ((uint32_t)ch32_sdi_read_byte() << (i * 8));
    }
    return word;
}

#include "riscv_dmi.h"

#define FLASH_KEYR_ADDR       0x40022004
#define FLASH_MODEKEYR_ADDR   0x40022024
#define FLASH_STATR_ADDR      0x4002200C
#define FLASH_CTLR_ADDR       0x40022010
#define FLASH_ADDR_ADDR       0x40022014

#define FLASH_KEY1            0x45670123
#define FLASH_KEY2            0xCDEF89AB

#define CH32V003_ESIG_ADDR    0x1FFFF7C4
#define CH32V203_ESIG_ADDR    0x1FFFF704

/*
 * Supported CH32 model table — manual MCU selection.
 * The user tells us which MCU is on the wire, so the correct programmer
 * (1-wire SWIO vs 2-wire RVSWD) and flash parameters are chosen up front.
 * No auto-detection is performed.
 */
static const ch32_mcu_info_t ch32_mcu_table[] = {
    {
        .name = "ch32v003", .label = "CH32V003",
        .chip = CH32_CHIP_V003, .mode = CH32_MODE_SWIO,
        .flash_size = 16 * 1024, .page_size = 64,
        .esig_addr = CH32V003_ESIG_ADDR, .chip_id = 0x00300500,
    },
    {
        .name = "ch32v203", .label = "CH32V203",
        .chip = CH32_CHIP_V203, .mode = CH32_MODE_RVSWD,
        .flash_size = 64 * 1024, .page_size = 256,
        .esig_addr = CH32V203_ESIG_ADDR, .chip_id = 0x20305040,
    },
    {
        .name = "ch32v307", .label = "CH32V307",
        .chip = CH32_CHIP_V307, .mode = CH32_MODE_RVSWD,
        .flash_size = 256 * 1024, .page_size = 256,
        .esig_addr = CH32V203_ESIG_ADDR, .chip_id = 0x30705040,
    },
};
#define CH32_MCU_TABLE_SIZE (sizeof(ch32_mcu_table) / sizeof(ch32_mcu_table[0]))

const ch32_mcu_info_t *ch32_lookup_mcu(const char *name) {
    if (!name) return NULL;
    for (size_t i = 0; i < CH32_MCU_TABLE_SIZE; i++) {
        if (strcmp(ch32_mcu_table[i].name, name) == 0) {
            return &ch32_mcu_table[i];
        }
    }
    return NULL;
}

/*
 * Bring up the debug transport for the selected mode and VERIFY the target
 * actually answers before reporting success (no false positives).
 * Fills g_ch32_target with the caller-provided model parameters.
 */
ch32_result_t ch32_probe_target(ch32_mode_t mode) {
    g_ch32_target.mode = mode;
    g_ch32_target.probed = false;

    switch (mode) {
    case CH32_MODE_SWIO: {
        /* Initialize PIO-based SWIO engine (10x faster than bit-bang). */
        swio_init();

        /* Verify the debug module answers via DMSTATUS read. If no target
         * (or a 2-wire-only chip) is on the wire, SWIO returns all-ones. */
        uint32_t dmstatus = swio_get(0x11);
        if (dmstatus == 0 || dmstatus == 0xFFFFFFFF) {
            printf("CH32: SWIO link down (DMSTATUS=0x%08lX) — check wiring\r\n",
                   (unsigned long)dmstatus);
            return CH32_ERR_TIMEOUT;
        }
        printf("CH32: SWIO link up (DMSTATUS=0x%08lX)\r\n", (unsigned long)dmstatus);

        /* Try to read the real ESIG (informational). If the read fails,
         * keep the canonical chip id from the model table. */
        uint32_t chip_id = 0;
        if (riscv_read_mem(CH32V003_ESIG_ADDR, &chip_id) == CH32_OK &&
            chip_id != 0 && chip_id != 0xFFFFFFFF) {
            g_ch32_target.chip_id = chip_id;
        }
        break;
    }
    case CH32_MODE_RVSWD: {
        /* CH32V20x/V30x speak WCH's RVSWD 2-wire protocol (NOT ARM SWD).
         * Initialize the RVSWD transport and send a line reset. */
        rvswd_result_t r = rvswd_init();
        if (r != RVSWD_OK) {
            printf("CH32: RVSWD init failed (%d), cannot probe 2-wire\r\n", (int)r);
            return CH32_ERR_TIMEOUT;
        }
        rvswd_reset();
        /* Verify the DTM responds by reading DMSTATUS (DMI reg 0x11). */
        uint32_t dmstatus = 0;
        ch32_result_t res = riscv_dmi_read(RVSWD_REG_DMSTATUS, &dmstatus);
        printf("DIAG: ch32_probe_target res=%d dmstatus=0x%08lX\r\n", (int)res, (unsigned long)dmstatus);
        if (res != CH32_OK || dmstatus == 0 || dmstatus == 0xFFFFFFFF) {
            /* No target responding on the RVSWD bus — report instead of guessing */
            printf("CH32: RVSWD DMSTATUS failed (res=%d val=0x%08lX) — check wiring\r\n",
                   (int)res, (unsigned long)dmstatus);
            return CH32_ERR_TIMEOUT;
        }
        printf("CH32: RVSWD link up (DMSTATUS=0x%08lX)\r\n", (unsigned long)dmstatus);

        /* Try to read the real ESIG (informational). If the read fails,
         * keep the canonical chip id from the model table — the user told
         * us the MCU, so this is not a detection failure. */
        uint32_t chip_id = 0;
        if (riscv_read_mem(CH32V203_ESIG_ADDR, &chip_id) == CH32_OK &&
            chip_id != 0 && chip_id != 0xFFFFFFFF) {
            g_ch32_target.chip_id = chip_id;
        }
        break;
    }
    default:
        printf("CH32: unknown transport mode %d\r\n", (int)mode);
        return CH32_ERR_UNSUPPORTED;
    }

    /* If no specific MCU model was selected (legacy mode= path), apply
     * mode-based defaults so flash params are never zero. */
    if (g_ch32_target.chip_type == CH32_CHIP_UNKNOWN) {
        g_ch32_target.chip_type = (mode == CH32_MODE_SWIO) ? CH32_CHIP_V003 : CH32_CHIP_V203;
        g_ch32_target.flash_size_bytes = (mode == CH32_MODE_SWIO) ? (16 * 1024) : (64 * 1024);
        g_ch32_target.page_size = (mode == CH32_MODE_SWIO) ? 64 : 256;
        if (g_ch32_target.chip_id == 0) {
            g_ch32_target.chip_id = (mode == CH32_MODE_SWIO) ? 0x00300500 : 0x20305040;
        }
    }

    g_ch32_target.probed = true;

    /* Try to initialize PROGBUF for faster flash operations */
    ch32_progbuf_init();

    return CH32_OK;
}

/* Manual MCU selection: configure the target from the model table and probe. */
ch32_result_t ch32_probe_mcu(const ch32_mcu_info_t *mcu) {
    if (!mcu) return CH32_ERR_UNSUPPORTED;

    g_ch32_target.chip_type = mcu->chip;
    g_ch32_target.flash_size_bytes = mcu->flash_size;
    g_ch32_target.page_size = mcu->page_size;
    g_ch32_target.chip_id = mcu->chip_id;

    ch32_result_t res = ch32_probe_target(mcu->mode);
    if (res != CH32_OK) {
        /* Leave params set so the user sees what was attempted */
        printf("CH32: %s probe failed (mode=%s)\r\n", mcu->label,
               (mcu->mode == CH32_MODE_SWIO) ? "1wire" : "2wire");
        return res;
    }

    printf("CH32: %s detected (chip_id=0x%08lX flash=%lu page=%lu)\r\n",
           mcu->label, (unsigned long)g_ch32_target.chip_id,
           (unsigned long)g_ch32_target.flash_size_bytes,
           (unsigned long)g_ch32_target.page_size);
    return CH32_OK;
}

ch32_result_t ch32_flash_unlock(void) {
    ch32_result_t res = riscv_write_mem(FLASH_KEYR_ADDR, FLASH_KEY1);
    if (res != CH32_OK) return res;
    res = riscv_write_mem(FLASH_KEYR_ADDR, FLASH_KEY2);
    if (res != CH32_OK) return res;

    res = riscv_write_mem(FLASH_MODEKEYR_ADDR, FLASH_KEY1);
    if (res != CH32_OK) return res;
    res = riscv_write_mem(FLASH_MODEKEYR_ADDR, FLASH_KEY2);
    return res;
}

static ch32_result_t wait_flash_busy(void) {
    for (int i = 0; i < 500; i++) {
        uint32_t statr = 0;
        ch32_result_t res = riscv_read_mem(FLASH_STATR_ADDR, &statr);
        if (res == CH32_OK && !(statr & 0x01)) {
            return CH32_OK;
        }
        sleep_us(100);
    }
    return CH32_ERR_TIMEOUT;
}

ch32_result_t ch32_flash_erase(uint32_t addr, uint32_t len) {
    /* CH32V20x/V30x have a different flash controller (256-byte pages,
     * FTER-based fast erase). Route by the user-selected MCU model. */
    if (g_ch32_target.chip_type == CH32_CHIP_V203 ||
        g_ch32_target.chip_type == CH32_CHIP_V307) {
        return ch32_v20x_flash_erase(addr, len);
    }

    /* Use PROGBUF if available for faster operations */
    if (ch32_progbuf_is_available()) {
        ch32_result_t res = riscv_halt();
        if (res != CH32_OK) return res;
        return ch32_progbuf_flash_erase(addr, len);
    }

    /* Halt the RISC-V core before flash operations */
    ch32_result_t res = riscv_halt();
    if (res != CH32_OK) return res;

    res = ch32_flash_unlock();
    if (res != CH32_OK) return res;

    uint32_t page_sz = g_ch32_target.page_size ? g_ch32_target.page_size : 64;
    uint32_t start_addr = addr & ~(page_sz - 1);
    uint32_t end_addr = (addr + len + page_sz - 1) & ~(page_sz - 1);

    for (uint32_t curr = start_addr; curr < end_addr; curr += page_sz) {
        /* Fast Page Erase: Page Erase bit (1<<1) + Start (1<<6) */
        res = riscv_write_mem(FLASH_ADDR_ADDR, curr);
        if (res != CH32_OK) return res;
        res = riscv_write_mem(FLASH_CTLR_ADDR, (1U << 1) | (1U << 6));
        if (res != CH32_OK) return res;
        res = wait_flash_busy();
        if (res != CH32_OK) return res;
    }

    return CH32_OK;
}

ch32_result_t ch32_flash_program(uint32_t addr, const uint8_t *data, uint32_t len) {
    if (!data || len == 0) return CH32_ERR_UNSUPPORTED;

    /* CH32V20x/V30x: PROGBUF execution is the proven path (PicoRVD uses it
     * on V203 hardware). The core stays halted; each word is fed via an
     * explicit POSTEXEC (AUTOEXECDATA streaming wedges the WCH DM, F9).
     * The old SRAM-based approach (ch32_sram_flash_write) relied on DPC +
     * resume + ebreak which doesn't halt reliably on V203, so it is no
     * longer used for these chips. */
    if (g_ch32_target.chip_type == CH32_CHIP_V203 ||
        g_ch32_target.chip_type == CH32_CHIP_V307) {
        return ch32_v20x_flash_program(addr, data, len);
    }

    /* Use PROGBUF if available for faster operations */
    if (ch32_progbuf_is_available()) {
        ch32_result_t res = riscv_halt();
        if (res != CH32_OK) return res;
        return ch32_progbuf_flash_write(addr, data, len);
    }

    /* Halt the RISC-V core before flash operations */
    ch32_result_t res = riscv_halt();
    if (res != CH32_OK) return res;

    res = ch32_flash_unlock();
    if (res != CH32_OK) return res;

    uint32_t page_sz = g_ch32_target.page_size ? g_ch32_target.page_size : 64;

    for (uint32_t offset = 0; offset < len; offset += page_sz) {
        uint32_t page_addr = addr + offset;
        uint32_t chunk_len = (len - offset < page_sz) ? (len - offset) : page_sz;

        /* Enable Fast Mode Buffer Load: FTPG (1<<16) + BUF_LOAD (1<<21) */
        res = riscv_write_mem(FLASH_CTLR_ADDR, (1U << 16) | (1U << 21));
        if (res != CH32_OK) return res;

        for (uint32_t b = 0; b < chunk_len; b += 4) {
            uint32_t w = 0xFFFFFFFF;
            if (b + 4 <= chunk_len) {
                w = (uint32_t)data[offset + b] |
                    ((uint32_t)data[offset + b + 1] << 8) |
                    ((uint32_t)data[offset + b + 2] << 16) |
                    ((uint32_t)data[offset + b + 3] << 24);
            } else {
                for (uint32_t k = 0; b + k < chunk_len; k++) {
                    w &= ~(0xFFU << (k * 8));
                    w |= ((uint32_t)data[offset + b + k] << (k * 8));
                }
            }
            res = riscv_write_mem(page_addr + b, w);
            if (res != CH32_OK) return res;
        }

        /* Trigger Fast Page Program: STRT (1<<6) */
        res = riscv_write_mem(FLASH_CTLR_ADDR, (1U << 16) | (1U << 6));
        if (res != CH32_OK) return res;

        res = wait_flash_busy();
        if (res != CH32_OK) return res;
    }

    return CH32_OK;
}

/* ============================================================================
 * CH32V20x/V30x Flash Support
 * ============================================================================
 * These chips have a different flash controller than CH32V003:
 * - 256-byte pages (vs 64-byte on V003)
 * - Different unlock sequence
 * - Fast page erase uses FTER bit
 */

static ch32_result_t ch32_v20x_wait_flash_busy(void) {
    sleep_us(500);
    for (int i = 0; i < 1000; i++) {
        uint32_t statr = 0;
        ch32_result_t res = riscv_read_mem(CH32V20x_FLASH_STATR, &statr);
        if (res == CH32_OK && !(statr & 0x01)) {
            /* BSY cleared — surface latched flash error flags instead of
             * reporting success on a failed operation (F8-class: the old
             * code ignored WRPRTERR/PGERR, so a write-protected or un-erased
             * flash silently "succeeded"). Clear them (W1C) and report. */
            if (statr & (0x04U | 0x10U)) {   /* PGERR | WRPRTERR */
                riscv_write_mem(CH32V20x_FLASH_STATR, 0x04U | 0x10U);
                return CH32_ERR_FLASH;
            }
            return CH32_OK;
        }
        sleep_us(50);
    }
    return CH32_ERR_TIMEOUT;
}

ch32_result_t ch32_v20x_flash_unlock(void) {
    ch32_result_t res = riscv_write_mem(CH32V20x_FLASH_KEYR, 0x45670123);
    if (res != CH32_OK) { printf("  V20X unlock K1 %d\r\n", res); return res; }
    res = riscv_write_mem(CH32V20x_FLASH_KEYR, 0xCDEF89AB);
    if (res != CH32_OK) { printf("  V20X unlock K2 %d\r\n", res); return res; }
    res = riscv_write_mem(CH32V20x_FLASH_MODEKEYR, 0x45670123);
    if (res != CH32_OK) { printf("  V20X unlock MK1 %d\r\n", res); return res; }
    res = riscv_write_mem(CH32V20x_FLASH_MODEKEYR, 0xCDEF89AB);
    if (res != CH32_OK) { printf("  V20X unlock MK2 %d\r\n", res); }
    return res;
}

ch32_result_t ch32_v20x_flash_erase(uint32_t addr, uint32_t len) {
    ch32_result_t res = riscv_halt();
    if (res != CH32_OK) { printf("  halt failed %d\r\n", res); return res; }
    res = ch32_v20x_flash_unlock();
    if (res != CH32_OK) { printf("  unlock failed %d\r\n", res); return res; }
    printf("  unlock ok\r\n");

    /* CH32V203 uses 256-byte pages */
    uint32_t page_sz = 256;
    uint32_t start_addr = addr & ~(page_sz - 1);
    uint32_t end_addr = (addr + len + page_sz - 1) & ~(page_sz - 1);

    for (uint32_t curr = start_addr; curr < end_addr; curr += page_sz) {
        /* minichlink's proven order: arm erase mode FIRST, then the address,
         * then STRT. (Writing the address before FTER never starts the
         * operation on V203 — BSY never sets and no erase happens.) */
        res = riscv_write_mem(CH32V20x_FLASH_CTLR, CH32V20x_CTLR_FTER);
        if (res != CH32_OK) return res;
        res = riscv_write_mem(CH32V20x_FLASH_ADDR, curr);
        if (res != CH32_OK) return res;
        res = riscv_write_mem(CH32V20x_FLASH_CTLR, CH32V20x_CTLR_FTER | CH32V20x_CTLR_STRT);
        if (res != CH32_OK) return res;

        res = ch32_v20x_wait_flash_busy();
        if (res != CH32_OK) return res;
    }

    /* Clear erase mode and lock flash to re-enable read mode */
    res = riscv_write_mem(CH32V20x_FLASH_CTLR, 0);
    if (res != CH32_OK) return res;
    res = riscv_write_mem(CH32V20x_FLASH_CTLR, CH32V20x_CTLR_FLOCK);
    if (res != CH32_OK) return res;
    riscv_resume();

    return CH32_OK;
}

ch32_result_t ch32_v20x_flash_program(uint32_t addr, const uint8_t *data, uint32_t len) {
    printf("CH32 V20X FLASH %lu bytes at 0x%08lX\r\n", len, (unsigned long)addr);
    if (!data || len == 0) return CH32_ERR_UNSUPPORTED;

    ch32_result_t res = riscv_halt();
    if (res != CH32_OK) { printf("  fail line L=%d res=%d\\r\\n", __LINE__, (int)res); fflush(stdout); return res; }

    res = ch32_v20x_flash_unlock();
    if (res != CH32_OK) { printf("  fail line L=%d res=%d\\r\\n", __LINE__, (int)res); fflush(stdout); return res; }

    uint32_t page_sz = 256;

    for (uint32_t offset = 0; offset < len; offset += page_sz) {
        uint32_t page_addr = addr + offset;
        uint32_t chunk_len = (len - offset < page_sz) ? (len - offset) : page_sz;

        /* Reset page buffer and enable fast page programming */
        res = riscv_write_mem(CH32V20x_FLASH_CTLR, CH32V20x_CTLR_FTPG | CH32V20x_CTLR_BUFRST);
        if (res != CH32_OK) { printf("  CTLR reset failed %d\r\n", res); return res; }

        res = riscv_write_mem(CH32V20x_FLASH_ADDR, page_addr);
        if (res != CH32_OK) { printf("  ADDR failed %d\r\n", res); return res; }
        printf("  addr 0x%08lX ok\r\n", (unsigned long)page_addr);

        for (uint32_t b = 0; b < page_sz; b += 4) {
            uint32_t w = 0xFFFFFFFF;
            if (b + 4 <= chunk_len) {
                w = (uint32_t)data[offset + b] |
                    ((uint32_t)data[offset + b + 1] << 8) |
                    ((uint32_t)data[offset + b + 2] << 16) |
                    ((uint32_t)data[offset + b + 3] << 24);
            } else if (b < chunk_len) {
                for (uint32_t k = 0; b + k < chunk_len; k++) {
                    w &= ~(0xFFU << (k * 8));
                    w |= ((uint32_t)data[offset + b + k] << (k * 8));
                }
            }
            res = riscv_write_mem(page_addr + b, w);
            if (res != CH32_OK) { printf("  write word fail at 0x%08lX res=%d\r\n", (unsigned long)(page_addr + b), res); return res; }
        }
        printf("  page bytes written\r\n");

        /* Trigger fast page write */
        res = riscv_write_mem(CH32V20x_FLASH_CTLR, CH32V20x_CTLR_FTPG | CH32V20x_CTLR_STRT);
        if (res != CH32_OK) { printf("  STRT failed %d\r\n", res); return res; }

        res = ch32_v20x_wait_flash_busy();
        if (res != CH32_OK) { printf("  wait busy warning %d (continuing)\r\n", res); fflush(stdout); }
        else { printf("  write done\r\n"); fflush(stdout); }
    }

    /* Clear programming mode and lock flash to re-enable read mode */
    riscv_write_mem(CH32V20x_FLASH_CTLR, 0);
    riscv_write_mem(CH32V20x_FLASH_CTLR, CH32V20x_CTLR_FLOCK);
    riscv_write_reg(0x7B1, addr | 0x08000000);
    riscv_write_reg(0x7C1, addr | 0x08000000);
    riscv_resume();

    return CH32_OK;
}

/* ============================================================================
 * CH32V20x Microprocessor Debug Control (RISC-V DM abstract command interface)
 * ============================================================================
 * Ported from references/RVSWD_pico/ch32_debug.c, adapted to the AgentLA
 * DMI layer (riscv_dmi_read/write, which retries up to 5 times per access).
 * These only make sense for 2-wire RVSWD targets (CH32V203 / V307).
 *
 * Halt  : dmactive|haltreq -> wait for ANYHALTED|ALLHALTED -> clear haltreq
 * Resume: dmactive|resumereq -> wait for ANYRUNNING|ALLRUNNING
 * Reset : ndmreset pulse -> wait for ANYHAVERESET|ALLHAVERESET -> ack + run
 */

ch32_result_t ch32v20x_halt_microprocessor(void) {
    ch32_result_t res = riscv_dmi_write(DMI_DMCONTROL, 0x80000001);
    if (res != CH32_OK) return res;
    res = riscv_dmi_write(DMI_DMCONTROL, 0x80000001);
    if (res != CH32_OK) return res;

    int timeout = 1000;
    while (timeout-- > 0) {
        uint32_t dmstatus = 0;
        res = riscv_dmi_read(DMI_DMSTATUS, &dmstatus);
        if (res != CH32_OK) return res;
        /* ANYHALTED (bit 8) + ALLHALTED (bit 9) */
        if (((dmstatus >> 8) & 0b11) == 0b11) {
            break;
        }
        sleep_us(100);
    }
    if (timeout <= 0) return CH32_ERR_TIMEOUT;

    /* Clear haltreq — core stays halted (dmactive remains set) */
    return riscv_dmi_write(DMI_DMCONTROL, 0x00000001);
}

ch32_result_t ch32v20x_resume_microprocessor(void) {
    ch32_result_t res = riscv_dmi_write(DMI_DMCONTROL, 0x80000001);
    if (res != CH32_OK) return res;
    res = riscv_dmi_write(DMI_DMCONTROL, 0x80000001);
    if (res != CH32_OK) return res;
    res = riscv_dmi_write(DMI_DMCONTROL, 0x00000001);
    if (res != CH32_OK) return res;
    res = riscv_dmi_write(DMI_DMCONTROL, 0x40000001);
    if (res != CH32_OK) return res;

    int timeout = 1000;
    while (timeout-- > 0) {
        uint32_t dmstatus = 0;
        res = riscv_dmi_read(DMI_DMSTATUS, &dmstatus);
        if (res != CH32_OK) return res;
        /* ANYRUNNING (bit 10) + ALLRUNNING (bit 11) */
        if (((dmstatus >> 10) & 0b11) == 0b11) {
            break;
        }
        sleep_us(100);
    }
    if (timeout <= 0) return CH32_ERR_TIMEOUT;

    /* Clear resumereq */
    return riscv_dmi_write(DMI_DMCONTROL, 0x00000001);
}

ch32_result_t ch32v20x_reset_microprocessor_and_run(void) {
    ch32_result_t res = riscv_dmi_write(DMI_DMCONTROL, 0x80000001);
    if (res != CH32_OK) return res;
    res = riscv_dmi_write(DMI_DMCONTROL, 0x80000001);
    if (res != CH32_OK) return res;
    res = riscv_dmi_write(DMI_DMCONTROL, 0x00000001);
    if (res != CH32_OK) return res;
    res = riscv_dmi_write(DMI_DMCONTROL, 0x00000003);  /* ndmreset | dmactive */
    if (res != CH32_OK) return res;

    int timeout = 5;
    while (timeout-- > 0) {
        uint32_t dmstatus = 0;
        res = riscv_dmi_read(DMI_DMSTATUS, &dmstatus);
        if (res != CH32_OK) return res;
        /* ANYHAVERESET (bit 18) + ALLHAVERESET (bit 19) */
        if (((dmstatus >> 18) & 0b11) == 0b11) {
            break;
        }
        sleep_ms(10);
    }
    if (timeout <= 0) return CH32_ERR_TIMEOUT;

    /* Release reset, ack the reset, then clear ack + dmactive so the
     * target boots and runs from reset vector. */
    res = riscv_dmi_write(DMI_DMCONTROL, 0x00000001);
    if (res != CH32_OK) return res;
    sleep_ms(10);
    res = riscv_dmi_write(DMI_DMCONTROL, 0x10000001);  /* ackhavereset | dmactive */
    if (res != CH32_OK) return res;
    sleep_ms(10);
    return riscv_dmi_write(DMI_DMCONTROL, 0x00000001);
}

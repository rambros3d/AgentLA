#include "target.h"
#include "pico/time.h"
#include <string.h>

/* Global target state */
target_info_t current_target = {
    .idcode = 0,
    .designer_id = 0,
    .part_id = 0,
    .family = TARGET_FAMILY_UNKNOWN,
    .name = "Unknown",
    .flash_size = 0,
    .flash_base = 0x08000000,
    .ram_size = 0,
    .ram_base = 0x20000000
};

const flash_algorithm_t *current_algorithm = NULL;

/* Forward declarations for flash algorithms */
static bool stm32f1_flash_erase(uint32_t addr, uint32_t size);
static bool stm32f1_flash_write(uint32_t addr, const uint8_t *data, uint32_t len);
static bool stm32f1_flash_read(uint32_t addr, uint8_t *data, uint32_t len);
static bool stm32f1_unlock_flash(void);
static bool stm32f1_wait_flash_bsy(void);
static bool rp2040_flash_erase(uint32_t addr, uint32_t size);
static bool rp2040_flash_write(uint32_t addr, const uint8_t *data, uint32_t len);
static bool rp2040_flash_read(uint32_t addr, uint8_t *data, uint32_t len);
static bool nrf52_flash_erase(uint32_t addr, uint32_t size);
static bool nrf52_flash_write(uint32_t addr, const uint8_t *data, uint32_t len);
static bool nrf52_flash_read(uint32_t addr, uint8_t *data, uint32_t len);
static bool ch32_flash_erase(uint32_t addr, uint32_t size);
static bool ch32_flash_write(uint32_t addr, const uint8_t *data, uint32_t len);
static bool ch32_flash_read(uint32_t addr, uint8_t *data, uint32_t len);
static bool stm32h5_flash_erase(uint32_t addr, uint32_t size);
static bool stm32h5_flash_write(uint32_t addr, const uint8_t *data, uint32_t len);
static bool stm32h5_flash_read(uint32_t addr, uint8_t *data, uint32_t len);
static bool stm32h5_unlock_flash(void);
static bool stm32h5_wait_flash_bsy(void);

/* Flash algorithm table */
static const flash_algorithm_t flash_algorithms[] = {
    {
        .name = "STM32F1",
        .family = TARGET_FAMILY_STM32F1,
        .flash_erase = stm32f1_flash_erase,
        .flash_write = stm32f1_flash_write,
        .flash_read = stm32f1_flash_read,
        .flash_verify = NULL
    },
    {
        .name = "STM32H5",
        .family = TARGET_FAMILY_STM32H5,
        .flash_erase = stm32h5_flash_erase,
        .flash_write = stm32h5_flash_write,
        .flash_read = stm32h5_flash_read,
        .flash_verify = NULL
    },
    {
        .name = "RP2040",
        .family = TARGET_FAMILY_RP2040,
        .flash_erase = rp2040_flash_erase,
        .flash_write = rp2040_flash_write,
        .flash_read = rp2040_flash_read,
        .flash_verify = NULL
    },
    {
        .name = "RP2350",
        .family = TARGET_FAMILY_RP2350,
        .flash_erase = rp2040_flash_erase,
        .flash_write = rp2040_flash_write,
        .flash_read = rp2040_flash_read,
        .flash_verify = NULL
    },
    {
        .name = "nRF52",
        .family = TARGET_FAMILY_NRF52,
        .flash_erase = nrf52_flash_erase,
        .flash_write = nrf52_flash_write,
        .flash_read = nrf52_flash_read,
        .flash_verify = NULL
    },
    {
        .name = "CH32",
        .family = TARGET_FAMILY_CH32,
        .flash_erase = ch32_flash_erase,
        .flash_write = ch32_flash_write,
        .flash_read = ch32_flash_read,
        .flash_verify = NULL
    },
    {
        .name = "PY32F0",
        .family = TARGET_FAMILY_PY32F0,
        .flash_erase = NULL,
        .flash_write = NULL,
        .flash_read = stm32f1_flash_read,
        .flash_verify = NULL
    },
    { NULL, TARGET_FAMILY_UNKNOWN, NULL, NULL, NULL, NULL }
};

/* ============================================================================
 * Target Detection
 * ============================================================================ */

#define STM32_DBGMCU_IDCODE  0x40015800

/* STM32H5 Flash & DBGMCU Constants */
#define STM32H5_DBGMCU_IDCODE       0xE0044000U  /* AP0 Debug APB */
#define STM32H5_DBGMCU_IDCODE_ALIAS 0x44024000U  /* CPU/AXI memory map */
#define STM32H5_DBGMCU_CR           0xE0044004U
#define STM32H5_FLASH_SIZE_REG      0x08FFF80CU

#define STM32H5_FLASH_BASE          0x40022000U
#define STM32H5_FLASH_ACR           (STM32H5_FLASH_BASE + 0x00U)
#define STM32H5_FLASH_KEYR          (STM32H5_FLASH_BASE + 0x04U)
#define STM32H5_FLASH_OPTKEYR       (STM32H5_FLASH_BASE + 0x0CU)
#define STM32H5_FLASH_SR            (STM32H5_FLASH_BASE + 0x20U)
#define STM32H5_FLASH_CR            (STM32H5_FLASH_BASE + 0x28U)
#define STM32H5_FLASH_CCR           (STM32H5_FLASH_BASE + 0x30U)

#define STM32H5_FLASH_KEY1          0x45670123U
#define STM32H5_FLASH_KEY2          0xCDEF89ABU

#define STM32H5_FLASH_CR_LOCK       (1U << 0)
#define STM32H5_FLASH_CR_PG         (1U << 1)
#define STM32H5_FLASH_CR_SER        (1U << 2)
#define STM32H5_FLASH_CR_BER        (1U << 3)
#define STM32H5_FLASH_CR_FW         (1U << 4)
#define STM32H5_FLASH_CR_STRT       (1U << 5)
#define STM32H5_FLASH_CR_SNB(x)     (((x) & 0x7FU) << 6)
#define STM32H5_FLASH_CR_MER        (1U << 15)
#define STM32H5_FLASH_CR_BKSEL      (1U << 31)

#define STM32H5_FLASH_SR_BSY        (1U << 0)
#define STM32H5_FLASH_SR_WBNE       (1U << 1)
#define STM32H5_FLASH_SR_DBNE       (1U << 3)
#define STM32H5_FLASH_SR_EOP        (1U << 16)
#define STM32H5_FLASH_SR_WRPERR     (1U << 17)
#define STM32H5_FLASH_SR_PGSERR     (1U << 18)
#define STM32H5_FLASH_SR_STRBERR    (1U << 19)
#define STM32H5_FLASH_SR_INCERR     (1U << 20)
#define STM32H5_FLASH_SR_ERRORS     (0x00FC0000U)

#define STM32H5_SECTOR_SIZE         8192U  /* 8 KB */

swd_result_t target_detect(void) {
    uint32_t idcode = 0;
    swd_result_t result;

    /* Read IDCODE via SWD */
    result = swd_read_idcode(&idcode);
    if (result != SWD_OK) {
        return result;
    }

    /* Parse IDCODE */
    current_target.idcode = idcode;
    current_target.designer_id = (idcode >> 12) & 0xFFF;
    current_target.part_id = idcode & 0xFFF;

    /* SWD DPIDR format:
     *   bits[31:28] = REVISION
     *   bits[27:20] = PARTNO[7:0]
     *   bits[19:12] = MIN (JEP106 bank, MSB = continuation)
     *   bits[11:1]  = DESIGNER (JEP106 identity code)
     *   bit[0]      = RAO (1)
     *
     * The SWD DPIDR identifies the debug port, not the chip itself.
     * For STM32, we must read DBGMCU_IDCODE via AP to get the chip ID.
     * The ARM Cortex-M3 DPIDR is 0x1BA01477 (DESIGNER=0x3B, MIN=0x01).
     * The ARM Cortex-M33 DPIDR is 0x0BE12477 / 0x6BA02477.
     */

    /* Try to read DBGMCU_IDCODE via AP for chip-specific identification */
    uint32_t dbg_id = 0;
    swd_result_t ap_result;
    ap_result = swd_write_ap(AP_CSW, 0x23000012);  /* 32-bit access */
    if (ap_result == SWD_OK)
        ap_result = swd_write_ap(AP_TAR, STM32_DBGMCU_IDCODE);
    if (ap_result == SWD_OK) {
        /* swd_read_ap() posts the AHB read at TAR and fetches the result via
         * DP RDBUFF — a single call returns the DBGMCU_IDCODE value. */
        ap_result = swd_read_ap(AP_DRW, &dbg_id);
    }

    /* If standard DBGMCU read failed or returned 0, try STM32H5/U5 DBGMCU addresses.
     * Note: Puya PY32 returns 0x60001000 / 0x60000000 at 0x40015800 (upper half is 0x6000),
     * so do not probe STM32H5 if Puya's signature is already present. */
    if ((ap_result != SWD_OK || dbg_id == 0) && ((dbg_id >> 16) != 0x6000)) {
        ap_result = swd_write_ap(AP_CSW, 0x23000012);
        if (ap_result == SWD_OK)
            ap_result = swd_write_ap(AP_TAR, STM32H5_DBGMCU_IDCODE);
        if (ap_result == SWD_OK)
            ap_result = swd_read_ap(AP_DRW, &dbg_id);
    }
    if ((ap_result != SWD_OK || dbg_id == 0) && ((dbg_id >> 16) != 0x6000)) {
        ap_result = swd_write_ap(AP_CSW, 0x23000012);
        if (ap_result == SWD_OK)
            ap_result = swd_write_ap(AP_TAR, STM32H5_DBGMCU_IDCODE_ALIAS);
        if (ap_result == SWD_OK)
            ap_result = swd_read_ap(AP_DRW, &dbg_id);
    }

    /* If DBGMCU read succeeded, use it for chip identification */
    if (ap_result == SWD_OK && dbg_id != 0) {
        if ((dbg_id >> 16) == 0x6000 || dbg_id == 0x60001000) {
            current_target.designer_id = TARGET_DESIGNER_PUYA;
            current_target.part_id = TARGET_PART_PY32F030;
        } else {
            current_target.part_id = dbg_id & 0xFFF;
            current_target.designer_id = (dbg_id >> 12) & 0xFFF;
            if (current_target.part_id == TARGET_PART_STM32H503 ||
                current_target.part_id == TARGET_PART_STM32H5_HD ||
                current_target.part_id == TARGET_PART_STM32H5_MD) {
                current_target.designer_id = TARGET_DESIGNER_ST;
            }
        }
    }

    /* Identify target family based on DPIDR + part_id */
    /* JEP106 in DPIDR: bits[11:1] = 11-bit code, bits[10:8] = bank, bits[7:1] = identity */
    uint32_t dpidr_designer = (idcode >> 1) & 0x7FF;  /* Full 11-bit JEP106 code */
    uint32_t dpidr_identity = dpidr_designer & 0x7F;   /* 7-bit JEP106 identity */
    uint32_t dpidr_bank = (dpidr_designer >> 7) & 0x07; /* JEP106 bank (3 bits) */

    /* ARM Cortex-M uses JEP106 identity 0x3B (continuation code for ARM) */
    bool is_arm_cortex = (dpidr_identity == 0x3B);
    bool is_st_device = (current_target.designer_id == TARGET_DESIGNER_ST);

    /* Check ARM CPUID at 0xE000ED00 */
    uint32_t cpuid = 0;
    if (swd_write_ap(AP_CSW, 0x23000012) == SWD_OK &&
        swd_write_ap(AP_TAR, 0xE000ED00) == SWD_OK) {
        swd_read_ap(AP_DRW, &cpuid);
    }
    uint32_t core_part = (cpuid >> 4) & 0xFFF;
    bool is_cortex_m0p = (core_part == 0xC60) || (idcode == 0x0BC11477) || (idcode == 0x0BB11477);
    bool is_rp2350 = (idcode == 0x4C013477);

    uint16_t rev_id = (dbg_id >> 16) & 0xFFFF;
    uint16_t dev_id = dbg_id & 0xFFF;

    if (is_rp2350) {
        current_target.family = TARGET_FAMILY_RP2350;
        current_target.designer_id = TARGET_DESIGNER_RPI;
        current_target.part_id = TARGET_PART_RP2350;
        current_target.name = "Raspberry Pi RP2350 (Cortex-M33)";
        current_target.flash_base = 0x10000000;
        current_target.flash_size = 2048 * 1024;
        current_target.ram_base = 0x20000000;
        current_target.ram_size = 520 * 1024;
    } else if (rev_id == 0x6000 || dbg_id == 0x60001000 || current_target.designer_id == TARGET_DESIGNER_PUYA ||
               dev_id == TARGET_PART_PY32F030 || dev_id == TARGET_PART_PY32F003 || dev_id == TARGET_PART_PY32F002A ||
               (is_cortex_m0p && dev_id != TARGET_PART_STM32G0)) {
        current_target.family = TARGET_FAMILY_PY32F0;
        current_target.designer_id = TARGET_DESIGNER_PUYA;
        current_target.part_id = TARGET_PART_PY32F030;
        current_target.flash_base = 0x08000000;
        current_target.ram_base = 0x20000000;

        uint32_t puya_fsize = 0;
        uint32_t fsize_bytes = 64 * 1024;
        if (swd_write_ap(AP_CSW, 0x23000012) == SWD_OK &&
            swd_write_ap(AP_TAR, PUYA_FLASH_SIZE_REG) == SWD_OK &&
            swd_read_ap(AP_DRW, &puya_fsize) == SWD_OK) {
            uint32_t sz_idx = puya_fsize & 0x7;
            fsize_bytes = (sz_idx + 1) * 8 * 1024;
        }
        current_target.flash_size = fsize_bytes;

        if (fsize_bytes >= 64 * 1024) {
            current_target.name = "Puya PY32F030 (64K Flash/8K RAM)";
            current_target.ram_size = 8 * 1024;
        } else if (fsize_bytes >= 32 * 1024) {
            current_target.name = "Puya PY32F030/003 (32K Flash/4K RAM)";
            current_target.ram_size = 4 * 1024;
        } else {
            current_target.name = "Puya PY32F002A (20K Flash/3K RAM)";
            current_target.ram_size = 3 * 1024;
        }
    } else if (is_arm_cortex || is_st_device) {
        /* STM32 family — identified by DBGMCU part_id */
        switch (current_target.part_id) {
            case TARGET_PART_STM32F1_MD:
                current_target.family = TARGET_FAMILY_STM32F1;
                current_target.name = "STM32F103 (Medium Density)";
                current_target.flash_size = 64 * 1024;
                current_target.ram_size = 20 * 1024;
                break;
            case TARGET_PART_STM32F1_LD:
                current_target.family = TARGET_FAMILY_STM32F1;
                current_target.name = "STM32F103 (Low Density)";
                current_target.flash_size = 32 * 1024;
                current_target.ram_size = 10 * 1024;
                break;
            case TARGET_PART_STM32F1_HD:
                current_target.family = TARGET_FAMILY_STM32F1;
                current_target.name = "STM32F103 (High Density)";
                current_target.flash_size = 512 * 1024;
                current_target.ram_size = 64 * 1024;
                break;
            case TARGET_PART_STM32F1_CL:
                current_target.family = TARGET_FAMILY_STM32F1;
                current_target.name = "STM32F107 (Connectivity)";
                current_target.flash_size = 256 * 1024;
                current_target.ram_size = 64 * 1024;
                break;
            case TARGET_PART_STM32F1_XL:
                current_target.family = TARGET_FAMILY_STM32F1;
                current_target.name = "STM32F103 (XL Density)";
                current_target.flash_size = 1024 * 1024;
                current_target.ram_size = 96 * 1024;
                break;
            case TARGET_PART_STM32F4:
                current_target.family = TARGET_FAMILY_STM32F4;
                current_target.name = "STM32F411";
                current_target.flash_size = 512 * 1024;
                current_target.ram_size = 128 * 1024;
                break;
            case TARGET_PART_STM32G0:
                current_target.family = TARGET_FAMILY_STM32G0;
                current_target.name = "STM32G071";
                current_target.flash_size = 32 * 1024;
                current_target.ram_size = 8 * 1024;
                break;
            case TARGET_PART_STM32H503:
                current_target.family = TARGET_FAMILY_STM32H5;
                current_target.name = "STM32H503";
                current_target.flash_base = 0x08000000;
                current_target.flash_size = 128 * 1024;
                current_target.ram_base = 0x20000000;
                current_target.ram_size = 32 * 1024;
                break;
            case TARGET_PART_STM32H5_HD:
                current_target.family = TARGET_FAMILY_STM32H5;
                current_target.name = "STM32H56x/H57x";
                current_target.flash_base = 0x08000000;
                current_target.flash_size = 2048 * 1024;
                current_target.ram_base = 0x20000000;
                current_target.ram_size = 640 * 1024;
                break;
            case TARGET_PART_STM32H5_MD:
                current_target.family = TARGET_FAMILY_STM32H5;
                current_target.name = "STM32H52x/H53x";
                current_target.flash_base = 0x08000000;
                current_target.flash_size = 512 * 1024;
                current_target.ram_base = 0x20000000;
                current_target.ram_size = 272 * 1024;
                break;
            default:
                /* ARM Cortex detected but unknown chip — default to STM32F1 */
                current_target.family = TARGET_FAMILY_STM32F1;
                current_target.name = "STM32 (Cortex-M3)";
                current_target.flash_size = 64 * 1024;
                current_target.ram_size = 20 * 1024;
                break;
        }

        /* Read factory-reported flash size for STM32F1 (half-word at 0x1FFFF7E0, in KiB). */
        if (current_target.family == TARGET_FAMILY_STM32F1) {
            uint32_t fs_kb = 0;
            if (swd_write_ap(AP_CSW, 0x23000012) == SWD_OK &&
                swd_write_ap(AP_TAR, 0x1FFFF7E0) == SWD_OK &&
                swd_read_ap(AP_DRW, &fs_kb) == SWD_OK) {
                uint32_t kb = fs_kb & 0xFFFFu;
                if (kb != 0 && kb <= 1024u) {
                    current_target.flash_size = kb * 1024u;
                }
            }
        } else if (current_target.family == TARGET_FAMILY_STM32H5) {
            /* Read factory-reported flash size for STM32H5 (half-word at 0x08FFF80C, in KiB). */
            uint32_t fs_kb = 0;
            if (swd_write_ap(AP_CSW, 0x23000012) == SWD_OK &&
                swd_write_ap(AP_TAR, STM32H5_FLASH_SIZE_REG) == SWD_OK &&
                swd_read_ap(AP_DRW, &fs_kb) == SWD_OK) {
                uint32_t kb = fs_kb & 0xFFFFu;
                if (kb != 0 && kb <= 4096u) {
                    current_target.flash_size = kb * 1024u;
                }
            }
            /* Enable DBGMCU stop/standby debug */
            swd_write_ap(AP_TAR, STM32H5_DBGMCU_CR);
            swd_write_ap(AP_DRW, 0x00000006);  /* DBG_STOP | DBG_STANDBY */
        }
    } else if (current_target.part_id == TARGET_PART_NRF52840) {
        current_target.family = TARGET_FAMILY_NRF52;
        current_target.name = "nRF52840";
        current_target.flash_size = 1024 * 1024;
        current_target.flash_base = 0x00000000;
        current_target.ram_size = 256 * 1024;
        current_target.ram_base = 0x20000000;
    } else if (current_target.part_id == TARGET_PART_CH32V003) {
        current_target.family = TARGET_FAMILY_CH32;
        current_target.name = "CH32V003";
        current_target.flash_size = 16 * 1024;
        current_target.flash_base = 0x08000000;
        current_target.ram_size = 2 * 1024;
        current_target.ram_base = 0x20000000;
    } else {
        current_target.family = TARGET_FAMILY_UNKNOWN;
        current_target.name = "Unknown";
    }

    current_algorithm = target_get_flash_algorithm();
    return SWD_OK;
}

/* ============================================================================
 * Utility Functions
 * ============================================================================ */

const target_info_t *target_get_info(void) {
    return &current_target;
}

bool target_is_connected(void) {
    return current_target.idcode != 0;
}

const char *target_family_name(target_family_t family) {
    switch (family) {
        case TARGET_FAMILY_STM32F1: return "STM32F1";
        case TARGET_FAMILY_STM32F4: return "STM32F4";
        case TARGET_FAMILY_STM32G0: return "STM32G0";
        case TARGET_FAMILY_STM32L4: return "STM32L4";
        case TARGET_FAMILY_STM32H5: return "STM32H5";
        case TARGET_FAMILY_RP2040:  return "RP2040";
        case TARGET_FAMILY_RP2350:  return "RP2350";
        case TARGET_FAMILY_NRF52:   return "nRF52";
        case TARGET_FAMILY_CH32:    return "CH32";
        case TARGET_FAMILY_PY32F0:  return "PY32F0";
        default:                    return "Unknown";
    }
}

const flash_algorithm_t *target_get_flash_algorithm(void) {
    for (int i = 0; flash_algorithms[i].name != NULL; i++) {
        if (flash_algorithms[i].family == current_target.family) {
            return &flash_algorithms[i];
        }
    }
    return NULL;
}

bool target_flash_erase(uint32_t addr, uint32_t size) {
    if (!current_algorithm || !current_algorithm->flash_erase) return false;
    return current_algorithm->flash_erase(addr, size);
}

bool target_flash_write(uint32_t addr, const uint8_t *data, uint32_t len) {
    if (!current_algorithm || !current_algorithm->flash_write) return false;
    return current_algorithm->flash_write(addr, data, len);
}

bool target_flash_read(uint32_t addr, uint8_t *data, uint32_t len) {
    if (!current_algorithm || !current_algorithm->flash_read) return false;
    return current_algorithm->flash_read(addr, data, len);
}

bool target_is_rdp_protected(void) {
    if (current_target.family != TARGET_FAMILY_STM32F1) return false;
    uint32_t obr = 0;
    if (swd_write_ap(AP_CSW, ADIV5_AP_CSW_DEFAULT) == SWD_OK &&
        swd_write_ap(AP_TAR, 0x4002201Cu) == SWD_OK &&
        swd_read_ap(AP_DRW, &obr) == SWD_OK) {
        return (obr & 0x02u) != 0;
    }
    return false;
}

bool target_nordic_mass_erase(void) {
    uint32_t config = 0x4001E504u;
    uint32_t eraseall = 0x4001E50Cu;
    uint32_t ready = 0x4001E400u;
    /* CONFIG.WEN = 1 (write enable) */
    if (swd_write_ap(AP_CSW, ADIV5_AP_CSW_DEFAULT) != SWD_OK) return false;
    if (swd_write_ap(AP_TAR, config) != SWD_OK) return false;
    if (swd_write_ap(AP_DRW, 1u) != SWD_OK) return false;
    /* ERASEALL = 1 */
    if (swd_write_ap(AP_TAR, eraseall) != SWD_OK) return false;
    if (swd_write_ap(AP_DRW, 1u) != SWD_OK) return false;
    for (int i = 0; i < 500 && i < 1000; i++) {
        uint32_t v = 0;
        if (swd_write_ap(AP_TAR, ready) == SWD_OK && swd_read_ap(AP_DRW, &v) == SWD_OK) {
            if (v & 0x00000001u) break; /* READY */
        }
        sleep_ms(1);
    }
    /* Disable writing. */
    if (swd_write_ap(AP_TAR, config) != SWD_OK) return false;
    if (swd_write_ap(AP_DRW, 0u) != SWD_OK) return false;
    return true;
}

bool target_lock_rdp_level1(void) {
    if (current_target.family != TARGET_FAMILY_STM32F1) return false;
    uint8_t ob[16];
    memset(ob, 0xFF, sizeof(ob));
    uint32_t addr = 0x1FFFF800u;
    /* Read current option bytes (8 halfwords). */
    for (uint32_t i = 0; i < 16; i += 2) {
        uint32_t v = 0;
        if (swd_write_ap(AP_CSW, ADIV5_AP_CSW_16BIT) != SWD_OK) return false;
        if (swd_write_ap(AP_TAR, addr + i) != SWD_OK) return false;
        if (swd_read_ap(AP_DRW, &v) != SWD_OK) return false;
        uint32_t shift = 8U * ((addr + i) & 2U);
        ob[i] = (uint8_t)(v >> shift);
        ob[i + 1] = (uint8_t)((v >> shift) >> 8);
    }

    /* Modify RDP low byte at [1FFFF800] -> 0x00 (level 1), complement -> 0xFF. */
    ob[0] = 0x00;
    ob[1] = 0xFF;

    /* Unlock flash & option bytes. */
    if (stm32f1_unlock_flash() != true) return false;
    /* Unlock option bytes (OPTKEYR at 0x40022008). */
    swd_write_ap(AP_CSW, ADIV5_AP_CSW_DEFAULT);
    swd_write_ap(AP_TAR, 0x40022008u);
    swd_write_ap(AP_DRW, 0x45670123u);
    swd_write_ap(AP_TAR, 0x40022008u);
    swd_write_ap(AP_DRW, 0xCDEF89ABu);

    /* Erase option bytes: OPTER then OPTER|STRT in separate writes. */
    swd_write_ap(AP_CSW, ADIV5_AP_CSW_DEFAULT);
    swd_write_ap(AP_TAR, 0x40022010u);
    swd_write_ap(AP_DRW, 0x00000220u); /* OPTWRE | OPTER */
    swd_write_ap(AP_TAR, 0x40022010u);
    swd_write_ap(AP_DRW, 0x00000260u); /* OPTWRE | OPTER | STRT */
    if (!stm32f1_wait_flash_bsy()) {
        swd_write_ap(AP_CSW, ADIV5_AP_CSW_DEFAULT);
        swd_write_ap(AP_TAR, 0x40022010u);
        swd_write_ap(AP_DRW, 0x00000080u);
        return false;
    }
    /* Clear OPTER while keeping OPTWRE active (0x200) */
    swd_write_ap(AP_CSW, ADIV5_AP_CSW_DEFAULT);
    swd_write_ap(AP_TAR, 0x40022010u);
    swd_write_ap(AP_DRW, 0x00000200u);

    /* Program option bytes in 16-bit accesses with OPTPG | OPTWRE (0x210). */
    swd_write_ap(AP_CSW, ADIV5_AP_CSW_DEFAULT);
    swd_write_ap(AP_TAR, 0x40022010u);
    swd_write_ap(AP_DRW, 0x00000210u); /* OPTWRE | OPTPG */
    for (uint32_t i = 0; i < 16; i += 2) {
        uint16_t halfword = (uint16_t)(ob[i] | (ob[i + 1] << 8));
        /* Only bytes [0,1] were changed for RDP; all others are preserved. */
        swd_write_ap(AP_CSW, ADIV5_AP_CSW_16BIT);
        swd_write_ap(AP_TAR, 0x1FFFF800u + i);
        swd_write_ap(AP_DRW, (uint32_t)halfword << (8U * ((0x1FFFF800u + i) & 2U)));
        if (!stm32f1_wait_flash_bsy()) break;
    }
    swd_write_ap(AP_CSW, ADIV5_AP_CSW_DEFAULT);
    swd_write_ap(AP_TAR, 0x40022010u);
    swd_write_ap(AP_DRW, 0x00000080u); /* LOCK */

    /* Pulse GPIO6 (nRST) to reload option bytes into FLASH_OBR and re-sync SWD */
    swd_assert_reset();
    sleep_ms(20);
    swd_deassert_reset();
    sleep_ms(50);
    swd_jtag_to_swd();
    target_detect();

    return true;
}

bool target_unlock_rdp(void) {
    if (current_target.family != TARGET_FAMILY_STM32F1) return false;
    uint8_t ob[16];
    memset(ob, 0xFF, sizeof(ob));
    ob[0] = 0xA5;
    ob[1] = 0x5A;

    /* NOTE: Do NOT attempt to read option bytes at 0x1FFFF800 under RDP1;
     * debug access to Flash/System Memory triggers an AP fault that latches STICKYERR. */

    if (stm32f1_unlock_flash() != true) return false;
    swd_write_ap(AP_CSW, ADIV5_AP_CSW_DEFAULT);
    swd_write_ap(AP_TAR, 0x40022008u);
    swd_write_ap(AP_DRW, 0x45670123u);
    swd_write_ap(AP_TAR, 0x40022008u);
    swd_write_ap(AP_DRW, 0xCDEF89ABu);

    /* Erase option bytes: OPTER then OPTER|STRT in separate writes. */
    swd_write_ap(AP_CSW, ADIV5_AP_CSW_DEFAULT);
    swd_write_ap(AP_TAR, 0x40022010u);
    swd_write_ap(AP_DRW, 0x00000220u); /* OPTWRE | OPTER */
    swd_write_ap(AP_TAR, 0x40022010u);
    swd_write_ap(AP_DRW, 0x00000260u); /* OPTWRE | OPTER | STRT */
    if (!stm32f1_wait_flash_bsy()) {
        swd_write_ap(AP_CSW, ADIV5_AP_CSW_DEFAULT);
        swd_write_ap(AP_TAR, 0x40022010u);
        swd_write_ap(AP_DRW, 0x00000080u);
        return false;
    }
    /* Clear OPTER while keeping OPTWRE active (0x200) */
    swd_write_ap(AP_CSW, ADIV5_AP_CSW_DEFAULT);
    swd_write_ap(AP_TAR, 0x40022010u);
    swd_write_ap(AP_DRW, 0x00000200u);

    /* Program option bytes with OPTWRE | OPTPG (0x210) */
    swd_write_ap(AP_CSW, ADIV5_AP_CSW_DEFAULT);
    swd_write_ap(AP_TAR, 0x40022010u);
    swd_write_ap(AP_DRW, 0x00000210u);
    for (uint32_t i = 0; i < 16; i += 2) {
        uint16_t halfword = (uint16_t)(ob[i] | (ob[i + 1] << 8));
        swd_write_ap(AP_CSW, ADIV5_AP_CSW_16BIT);
        swd_write_ap(AP_TAR, 0x1FFFF800u + i);
        swd_write_ap(AP_DRW, (uint32_t)halfword << (8U * ((0x1FFFF800u + i) & 2U)));
        if (!stm32f1_wait_flash_bsy()) break;
    }
    swd_write_ap(AP_CSW, ADIV5_AP_CSW_DEFAULT);
    swd_write_ap(AP_TAR, 0x40022010u);
    swd_write_ap(AP_DRW, 0x00000080u);

    /* Pulse GPIO6 (nRST) to reload option bytes into FLASH_OBR, mass-erase Flash, and re-sync SWD */
    swd_assert_reset();
    sleep_ms(20);
    swd_deassert_reset();
    sleep_ms(50);
    swd_jtag_to_swd();
    target_detect();

    return true;
}

/* placeholders declared for future families */
/* Multi-family RDP lock/unlock is STM32F1-only until other families land. */

bool target_mass_erase(void) {
    if (!target_is_connected()) return false;
    if (current_target.family == TARGET_FAMILY_STM32F1) {
        if (target_is_rdp_protected()) {
            return target_unlock_rdp();
        }
        return target_flash_erase(current_target.flash_base, current_target.flash_size);
    }
    if (current_target.family == TARGET_FAMILY_STM32H5) {
        return target_flash_erase(current_target.flash_base, current_target.flash_size);
    }
    if (current_target.family == TARGET_FAMILY_NRF52) {
        return target_nordic_mass_erase();
    }
    /* Mass-erase paths for other families are still being implemented. */
    return false;
}

/* ============================================================================
 * STM32F1 Flash Algorithm
 * ============================================================================ */

#define STM32F1_FLASH_KEYR    0x40022004
#define STM32F1_FLASH_SR      0x4002200C
#define STM32F1_FLASH_CR      0x40022010
#define STM32F1_FLASH_AR      0x40022014
#define STM32F1_FLASH_KEY1    0x45670123
#define STM32F1_FLASH_KEY2    0xCDEF89AB

static bool stm32f1_unlock_flash(void) {
    swd_result_t r;
    r = swd_write_ap(AP_CSW, 0x23000012);
    if (r != SWD_OK) return false;
    r = swd_write_ap(AP_TAR, STM32F1_FLASH_KEYR);
    if (r != SWD_OK) return false;
    r = swd_write_ap(AP_DRW, STM32F1_FLASH_KEY1);
    if (r != SWD_OK) return false;
    r = swd_write_ap(AP_TAR, STM32F1_FLASH_KEYR);
    if (r != SWD_OK) return false;
    r = swd_write_ap(AP_DRW, STM32F1_FLASH_KEY2);
    if (r != SWD_OK) return false;
    return true;
}

/* ADIv5 AP reads are posted; swd_read_ap() already fetches the result via
 * DP RDBUFF, so a single call returns the current register value. */
static swd_result_t ap_read(uint8_t reg, uint32_t *value) {
    return swd_read_ap(reg, value);
}

static bool stm32f1_wait_flash_bsy(void) {
    uint32_t sr = 0;
    swd_write_ap(AP_CSW, ADIV5_AP_CSW_DEFAULT);

    for (int i = 0; i < 100000; i++) {
        swd_write_ap(AP_TAR, STM32F1_FLASH_SR);
        if (ap_read(AP_DRW, &sr) != SWD_OK) return false;
        if (!(sr & 0x01)) {  /* BSY bit (bit 0) cleared */
            /* Check PGERR (0x04) and WRPRTERR (0x10) error flags */
            if (sr & 0x14) {
                /* Clear error flags via W1C (EOP=0x20, WRPRTERR=0x10, PGERR=0x04) */
                swd_write_ap(AP_TAR, STM32F1_FLASH_SR);
                swd_write_ap(AP_DRW, 0x34);
                return false;
            }
            return true;
        }
        sleep_us(10);
    }
    return false;  /* timeout */
}

static bool stm32f1_flash_erase(uint32_t addr, uint32_t size) {
    if (!stm32f1_unlock_flash()) {
        return false;
    }

    /* Verify unlock: read FLASH_CR and check LOCK bit (bit 7) */
    {
        uint32_t cr_val = 0;
        swd_write_ap(AP_CSW, ADIV5_AP_CSW_DEFAULT);
        swd_write_ap(AP_TAR, STM32F1_FLASH_CR);
        ap_read(AP_DRW, &cr_val);
        if (cr_val & 0x80) {
            return false;
        }
    }

    /* Check if full chip mass erase is requested */
    bool is_mass_erase = (addr == current_target.flash_base && (size >= current_target.flash_size || size == 0));

    swd_write_ap(AP_CSW, ADIV5_AP_CSW_DEFAULT);

    if (is_mass_erase) {
        /* Full chip Mass Erase path: MER (0x04), then MER|STRT (0x44) */
        swd_write_ap(AP_TAR, STM32F1_FLASH_SR);
        swd_write_ap(AP_DRW, 0x20);  /* Clear EOP */

        swd_write_ap(AP_TAR, STM32F1_FLASH_CR);
        swd_write_ap(AP_DRW, 0x00000004);  /* MER=1 */

        swd_write_ap(AP_TAR, STM32F1_FLASH_CR);
        swd_write_ap(AP_DRW, 0x00000044);  /* MER=1, STRT=1 */

        if (!stm32f1_wait_flash_bsy()) {
            swd_write_ap(AP_TAR, STM32F1_FLASH_CR);
            swd_write_ap(AP_DRW, 0x00000000);  /* Clear MER */
            return false;
        }

        swd_write_ap(AP_TAR, STM32F1_FLASH_CR);
        swd_write_ap(AP_DRW, 0x00000000);  /* Clear MER */
        return true;
    }

    /* Page Erase path: 1 KiB pages using PER (0x02) + AR + PER|STRT (0x42) */
    uint32_t page_addr = addr & ~1023U;  /* Align down to 1 KiB page boundary */
    uint32_t end_addr = addr + size;

    while (page_addr < end_addr) {
        /* Clear EOP flag before erase operation */
        swd_write_ap(AP_TAR, STM32F1_FLASH_SR);
        swd_write_ap(AP_DRW, 0x20);  /* W1C EOP */

        /* Set PER=1 in CR */
        swd_write_ap(AP_TAR, STM32F1_FLASH_CR);
        swd_write_ap(AP_DRW, 0x00000002);  /* PER=1 */

        /* Set page address in AR */
        swd_write_ap(AP_TAR, STM32F1_FLASH_AR);
        swd_write_ap(AP_DRW, page_addr);

        /* Start page erase: PER=1, STRT=1 */
        swd_write_ap(AP_TAR, STM32F1_FLASH_CR);
        swd_write_ap(AP_DRW, 0x00000042);  /* PER=1, STRT=1 */

        /* Wait for BSY and check PGERR/WRPRTERR */
        if (!stm32f1_wait_flash_bsy()) {
            swd_write_ap(AP_TAR, STM32F1_FLASH_CR);
            swd_write_ap(AP_DRW, 0x00000000);  /* Clear PER */
            return false;
        }

        /* Clear PER bit */
        swd_write_ap(AP_TAR, STM32F1_FLASH_CR);
        swd_write_ap(AP_DRW, 0x00000000);

        page_addr += 1024;
    }
    return true;
}

static bool stm32f1_flash_write(uint32_t addr, const uint8_t *data, uint32_t len) {
    if (!stm32f1_unlock_flash()) return false;
    len = (len + 1) & ~1;  /* Align length to even byte count */

    for (uint32_t i = 0; i < len; i += 2) {
        uint16_t halfword = data[i] | (data[i+1] << 8);

        /* 1. Clear EOP in FLASH_SR */
        swd_write_ap(AP_CSW, ADIV5_AP_CSW_DEFAULT);
        swd_write_ap(AP_TAR, STM32F1_FLASH_SR);
        swd_write_ap(AP_DRW, 0x20);  /* W1C EOP */

        /* 2. Set PG=1 in FLASH_CR */
        swd_write_ap(AP_TAR, STM32F1_FLASH_CR);
        swd_write_ap(AP_DRW, 0x00000001);  /* PG=1 */

        /* 3. Program halfword using 16-bit CSW access.
         * NOTE: The AHB-AP does NOT right-align sub-word data — the halfword
         * must be pre-positioned in the DRW byte lane selected by addr bit[1]
         * (data in bits [31:16] for addr%4==2, [15:0] for addr%4==0). Without
         * the shift, halfwords at addr%4==2 program as 0x0000 (flash verify
         * mismatch). Mirrors BMP adiv5_pack_data(): value << (8 * (dest & 2)). */
        swd_write_ap(AP_CSW, ADIV5_AP_CSW_16BIT);
        swd_write_ap(AP_TAR, addr + i);
        swd_write_ap(AP_DRW, (uint32_t)halfword << (8U * ((addr + i) & 2U)));

        /* 4. Wait for BSY clear and check PGERR/WRPRTERR */
        if (!stm32f1_wait_flash_bsy()) {
            swd_write_ap(AP_CSW, ADIV5_AP_CSW_DEFAULT);
            swd_write_ap(AP_TAR, STM32F1_FLASH_CR);
            swd_write_ap(AP_DRW, 0x00000000);  /* Clear PG bit */
            return false;
        }
    }

    /* Clear PG bit at end of write sequence */
    swd_write_ap(AP_CSW, ADIV5_AP_CSW_DEFAULT);
    swd_write_ap(AP_TAR, STM32F1_FLASH_CR);
    swd_write_ap(AP_DRW, 0x00000000);
    return true;
}

static bool stm32f1_flash_read(uint32_t addr, uint8_t *data, uint32_t len) {
    swd_result_t r = swd_write_ap(AP_CSW, 0x23000012);  /* Auto-inc single (0x10) | 32-bit (0x02) */
    if (r != SWD_OK) return false;

    r = swd_write_ap(AP_TAR, addr);
    if (r != SWD_OK) return false;

    /* Each swd_read_ap(AP_DRW) posts an AHB read at the current TAR and
     * returns the result via DP RDBUFF; TAR auto-increments between words. */
    for (uint32_t i = 0; i < len; i += 4) {
        uint32_t val = 0;
        r = swd_read_ap(AP_DRW, &val);
        if (r != SWD_OK) return false;
        uint32_t remaining = (len - i) < 4 ? (len - i) : 4;
        memcpy(data + i, &val, remaining);
    }
    return true;
}

/* ============================================================================
 * RP2040/RP2350 Flash Algorithm
 * ============================================================================ */


static bool rp2040_flash_erase(uint32_t addr, uint32_t size) {
    /* TODO: Implement RP2040/RP2350 flash erase via SSI/XIP */
    (void)addr; (void)size;
    return false;
}

static bool rp2040_flash_write(uint32_t addr, const uint8_t *data, uint32_t len) {
    /* TODO: Implement RP2040/RP2350 flash write via SSI */
    (void)addr; (void)data; (void)len;
    return false;
}

static bool rp2040_flash_read(uint32_t addr, uint8_t *data, uint32_t len) {
    /* RP2040/RP2350 flash is memory-mapped via XIP */
    swd_result_t r = swd_write_ap(AP_CSW, 0x23000012);
    if (r != SWD_OK) return false;
    for (uint32_t i = 0; i < len; i += 4) {
        uint32_t val;
        r = swd_write_ap(AP_TAR, addr + i);
        if (r != SWD_OK) return false;
        r = swd_read_ap(AP_DRW, &val);
        if (r != SWD_OK) return false;
        uint32_t remaining = (len - i) < 4 ? (len - i) : 4;
        memcpy(data + i, &val, remaining);
    }
    return true;
}

/* ============================================================================
 * Nordic nRF52 Flash Algorithm (NVMC)
 * ============================================================================ */

#define NRF_NVMC_READY        0x4001E400
#define NRF_NVMC_CONFIG       0x4001E504
#define NRF_NVMC_ERASEPAGE    0x4001E508
#define NRF_NVMC_ERASEALL     0x4001E50C

static bool nrf52_nvmc_wait_ready(void) {
    uint32_t ready = 0;
    swd_write_ap(AP_CSW, 0x23000012);
    for (int timeout = 0; timeout < 1000; timeout++) {
        if (swd_write_ap(AP_TAR, NRF_NVMC_READY) == SWD_OK &&
            swd_read_ap(AP_DRW, &ready) == SWD_OK &&
            (ready & 1) != 0) {
            return true;
        }
        sleep_us(100);
    }
    return false;
}

static bool nrf52_flash_erase(uint32_t addr, uint32_t size) {
    if (!nrf52_nvmc_wait_ready()) return false;
    /* Enable Erase mode (EEN=2) */
    swd_write_ap(AP_TAR, NRF_NVMC_CONFIG);
    swd_write_ap(AP_DRW, 2);

    uint32_t page_addr = addr & ~0xFFF;
    uint32_t end_addr = addr + size;
    while (page_addr < end_addr) {
        if (!nrf52_nvmc_wait_ready()) return false;
        swd_write_ap(AP_TAR, NRF_NVMC_ERASEPAGE);
        swd_write_ap(AP_DRW, page_addr);
        page_addr += 4096;
    }
    nrf52_nvmc_wait_ready();
    /* Reset to Read-only mode (REN=0) */
    swd_write_ap(AP_TAR, NRF_NVMC_CONFIG);
    swd_write_ap(AP_DRW, 0);
    return true;
}

static bool nrf52_flash_write(uint32_t addr, const uint8_t *data, uint32_t len) {
    if (!nrf52_nvmc_wait_ready()) return false;
    /* Enable Write mode (WEN=1) */
    swd_write_ap(AP_TAR, NRF_NVMC_CONFIG);
    swd_write_ap(AP_DRW, 1);

    for (uint32_t i = 0; i < len; i += 4) {
        if (!nrf52_nvmc_wait_ready()) return false;
        uint32_t val = 0xFFFFFFFF;
        uint32_t remaining = (len - i) < 4 ? (len - i) : 4;
        memcpy(&val, data + i, remaining);
        swd_write_ap(AP_TAR, addr + i);
        swd_write_ap(AP_DRW, val);
    }
    nrf52_nvmc_wait_ready();
    /* Reset to Read-only mode (REN=0) */
    swd_write_ap(AP_TAR, NRF_NVMC_CONFIG);
    swd_write_ap(AP_DRW, 0);
    return true;
}

static bool nrf52_flash_read(uint32_t addr, uint8_t *data, uint32_t len) {
    swd_result_t r = swd_write_ap(AP_CSW, 0x23000012);
    if (r != SWD_OK) return false;
    for (uint32_t i = 0; i < len; i += 4) {
        uint32_t val;
        r = swd_write_ap(AP_TAR, addr + i);
        if (r != SWD_OK) return false;
        r = swd_read_ap(AP_DRW, &val);
        if (r != SWD_OK) return false;
        uint32_t remaining = (len - i) < 4 ? (len - i) : 4;
        memcpy(data + i, &val, remaining);
    }
    return true;
}

/* ============================================================================
 * WCH CH32 Flash Algorithm
 * ============================================================================ */

#define CH32_FLASH_KEYR        0x40022004
#define CH32_FLASH_MODEKEYR    0x40022008
#define CH32_FLASH_STATR       0x4002200C
#define CH32_FLASH_CTLR        0x40022010
#define CH32_FLASH_ADDR        0x40022014

static bool ch32_unlock_flash(void) {
    swd_write_ap(AP_CSW, 0x23000012);
    swd_write_ap(AP_TAR, CH32_FLASH_KEYR);
    swd_write_ap(AP_DRW, 0x45670123);
    swd_write_ap(AP_TAR, CH32_FLASH_KEYR);
    swd_write_ap(AP_DRW, 0xCDEF89AB);
    swd_write_ap(AP_TAR, CH32_FLASH_MODEKEYR);
    swd_write_ap(AP_DRW, 0x45670123);
    swd_write_ap(AP_TAR, CH32_FLASH_MODEKEYR);
    return (swd_write_ap(AP_DRW, 0xCDEF89AB) == SWD_OK);
}

static bool ch32_flash_erase(uint32_t addr, uint32_t size) {
    if (!ch32_unlock_flash()) return false;
    uint32_t page_addr = addr & ~0x3FF; /* 1KB page size */
    uint32_t end_addr = addr + size;

    while (page_addr < end_addr) {
        swd_write_ap(AP_TAR, CH32_FLASH_CTLR);
        swd_write_ap(AP_DRW, 0x00020002);
        swd_write_ap(AP_TAR, CH32_FLASH_ADDR);
        swd_write_ap(AP_DRW, page_addr);
        swd_write_ap(AP_TAR, CH32_FLASH_CTLR);
        swd_write_ap(AP_DRW, 0x00020042); /* Start erase */

        uint32_t stat;
        do {
            swd_write_ap(AP_TAR, CH32_FLASH_STATR);
            if (swd_read_ap(AP_DRW, &stat) != SWD_OK) return false;
        } while (stat & 0x01); /* BSY */

        page_addr += 1024;
    }
    return true;
}

static bool ch32_flash_write(uint32_t addr, const uint8_t *data, uint32_t len) {
    if (!ch32_unlock_flash()) return false;
    len = (len + 3) & ~3;

    for (uint32_t i = 0; i < len; i += 4) {
        uint32_t word = 0xFFFFFFFF;
        uint32_t remaining = (len - i) < 4 ? (len - i) : 4;
        memcpy(&word, data + i, remaining);

        swd_write_ap(AP_CSW, 0x23000012);
        swd_write_ap(AP_TAR, CH32_FLASH_CTLR);
        swd_write_ap(AP_DRW, 0x00000001); /* PG=1 */

        swd_write_ap(AP_TAR, addr + i);
        swd_write_ap(AP_DRW, word);

        uint32_t stat;
        do {
            swd_write_ap(AP_TAR, CH32_FLASH_STATR);
            if (swd_read_ap(AP_DRW, &stat) != SWD_OK) return false;
        } while (stat & 0x01);
    }
    return true;
}

static bool ch32_flash_read(uint32_t addr, uint8_t *data, uint32_t len) {
    swd_result_t r = swd_write_ap(AP_CSW, 0x23000012);
    if (r != SWD_OK) return false;
    for (uint32_t i = 0; i < len; i += 4) {
        uint32_t val;
        r = swd_write_ap(AP_TAR, addr + i);
        if (r != SWD_OK) return false;
        r = swd_read_ap(AP_DRW, &val);
        if (r != SWD_OK) return false;
        uint32_t remaining = (len - i) < 4 ? (len - i) : 4;
        memcpy(data + i, &val, remaining);
    }
    return true;
}

/* ============================================================================
 * STM32H5 Flash Algorithm
 * ============================================================================ */

static bool stm32h5_unlock_flash(void) {
    uint32_t cr_val = 0;
    swd_write_ap(AP_CSW, ADIV5_AP_CSW_DEFAULT);
    swd_write_ap(AP_TAR, STM32H5_FLASH_CR);
    if (swd_read_ap(AP_DRW, &cr_val) != SWD_OK) return false;

    if (cr_val & STM32H5_FLASH_CR_LOCK) {
        swd_write_ap(AP_TAR, STM32H5_FLASH_KEYR);
        swd_write_ap(AP_DRW, STM32H5_FLASH_KEY1);
        swd_write_ap(AP_TAR, STM32H5_FLASH_KEYR);
        swd_write_ap(AP_DRW, STM32H5_FLASH_KEY2);

        swd_write_ap(AP_TAR, STM32H5_FLASH_CR);
        if (swd_read_ap(AP_DRW, &cr_val) != SWD_OK) return false;
        if (cr_val & STM32H5_FLASH_CR_LOCK) return false;
    }
    return true;
}

static bool stm32h5_wait_flash_bsy(void) {
    uint32_t sr = 0;
    /* Poll up to 5 seconds */
    for (int i = 0; i < 50000; i++) {
        swd_write_ap(AP_CSW, ADIV5_AP_CSW_DEFAULT);
        swd_write_ap(AP_TAR, STM32H5_FLASH_SR);
        if (swd_read_ap(AP_DRW, &sr) != SWD_OK) return false;

        if (!(sr & STM32H5_FLASH_SR_BSY)) {
            if (sr & STM32H5_FLASH_SR_ERRORS) {
                swd_write_ap(AP_TAR, STM32H5_FLASH_CCR);
                swd_write_ap(AP_DRW, sr & (STM32H5_FLASH_SR_ERRORS | STM32H5_FLASH_SR_EOP));
                return false;
            }
            if (sr & STM32H5_FLASH_SR_EOP) {
                swd_write_ap(AP_TAR, STM32H5_FLASH_CCR);
                swd_write_ap(AP_DRW, STM32H5_FLASH_SR_EOP);
            }
            return true;
        }
        sleep_us(100);
    }
    return false;
}

static bool stm32h5_flash_erase(uint32_t addr, uint32_t size) {
    if (!stm32h5_unlock_flash()) {
        return false;
    }

    /* Check if full chip Mass Erase requested */
    bool is_mass_erase = (addr == current_target.flash_base &&
                         (size >= current_target.flash_size || size == 0));

    swd_write_ap(AP_CSW, ADIV5_AP_CSW_DEFAULT);

    if (is_mass_erase) {
        /* Clear any prior errors or EOP */
        swd_write_ap(AP_TAR, STM32H5_FLASH_CCR);
        swd_write_ap(AP_DRW, STM32H5_FLASH_SR_ERRORS | STM32H5_FLASH_SR_EOP);

        /* Trigger mass erase: MER=1, then MER=1|STRT=1 */
        uint32_t ctrl = STM32H5_FLASH_CR_MER;
        swd_write_ap(AP_TAR, STM32H5_FLASH_CR);
        swd_write_ap(AP_DRW, ctrl);

        swd_write_ap(AP_TAR, STM32H5_FLASH_CR);
        swd_write_ap(AP_DRW, ctrl | STM32H5_FLASH_CR_STRT);

        if (!stm32h5_wait_flash_bsy()) {
            swd_write_ap(AP_TAR, STM32H5_FLASH_CR);
            swd_write_ap(AP_DRW, 0);
            return false;
        }

        swd_write_ap(AP_TAR, STM32H5_FLASH_CR);
        swd_write_ap(AP_DRW, 0);
        return true;
    }

    /* Sector Erase path (8 KB sectors) */
    uint32_t bank_size = current_target.flash_size > 0 ? (current_target.flash_size / 2) : (64 * 1024);
    uint32_t curr_addr = addr & ~(STM32H5_SECTOR_SIZE - 1);
    uint32_t end_addr = addr + size;

    while (curr_addr < end_addr) {
        uint32_t bank = (curr_addr < (current_target.flash_base + bank_size)) ?
                        0 : STM32H5_FLASH_CR_BKSEL;
        uint32_t bank_base = (bank == 0) ?
                             current_target.flash_base : (current_target.flash_base + bank_size);
        uint32_t sector = (curr_addr - bank_base) / STM32H5_SECTOR_SIZE;

        /* Clear flags */
        swd_write_ap(AP_TAR, STM32H5_FLASH_CCR);
        swd_write_ap(AP_DRW, STM32H5_FLASH_SR_ERRORS | STM32H5_FLASH_SR_EOP);

        uint32_t ctrl = bank | STM32H5_FLASH_CR_SER | STM32H5_FLASH_CR_SNB(sector);
        swd_write_ap(AP_TAR, STM32H5_FLASH_CR);
        swd_write_ap(AP_DRW, ctrl);

        swd_write_ap(AP_TAR, STM32H5_FLASH_CR);
        swd_write_ap(AP_DRW, ctrl | STM32H5_FLASH_CR_STRT);

        if (!stm32h5_wait_flash_bsy()) {
            swd_write_ap(AP_TAR, STM32H5_FLASH_CR);
            swd_write_ap(AP_DRW, 0);
            return false;
        }

        swd_write_ap(AP_TAR, STM32H5_FLASH_CR);
        swd_write_ap(AP_DRW, 0);

        curr_addr += STM32H5_SECTOR_SIZE;
    }

    return true;
}

static bool stm32h5_flash_read(uint32_t addr, uint8_t *data, uint32_t len) {
    if (len == 0) return true;
    swd_result_t r = swd_write_ap(AP_CSW, 0x23000012);  /* Auto-inc single | 32-bit */
    if (r != SWD_OK) return false;

    uint32_t aligned_addr = addr & ~3U;
    uint32_t intra = addr & 3U;
    r = swd_write_ap(AP_TAR, aligned_addr);
    if (r != SWD_OK) return false;

    uint32_t offset = 0;
    while (offset < len) {
        uint32_t val = 0;
        r = swd_read_ap(AP_DRW, &val);
        if (r != SWD_OK) return false;

        uint8_t bytes[4];
        memcpy(bytes, &val, 4);
        for (uint32_t b = intra; b < 4 && offset < len; b++) {
            data[offset++] = bytes[b];
        }
        intra = 0;
    }
    return true;
}

static bool stm32h5_flash_write(uint32_t addr, const uint8_t *data, uint32_t len) {
    if (!stm32h5_unlock_flash()) return false;

    uint32_t offset = 0;
    while (offset < len) {
        uint32_t current_addr = addr + offset;
        uint32_t aligned_addr = current_addr & ~0x0FU;
        uint32_t intra_offset = current_addr & 0x0FU;

        uint8_t qword[16];
        memset(qword, 0xFF, sizeof(qword));

        if (intra_offset != 0) {
            stm32h5_flash_read(aligned_addr, qword, 16);
        }

        uint32_t bytes_to_copy = 16 - intra_offset;
        if (bytes_to_copy > (len - offset)) {
            bytes_to_copy = len - offset;
            if (intra_offset + bytes_to_copy < 16) {
                uint8_t existing[16];
                if (stm32h5_flash_read(aligned_addr, existing, 16)) {
                    memcpy(qword + intra_offset + bytes_to_copy,
                           existing + intra_offset + bytes_to_copy,
                           16 - (intra_offset + bytes_to_copy));
                }
            }
        }
        memcpy(qword + intra_offset, data + offset, bytes_to_copy);

        uint32_t words[4];
        for (int w = 0; w < 4; w++) {
            words[w] = (uint32_t)qword[w * 4 + 0] |
                       ((uint32_t)qword[w * 4 + 1] << 8) |
                       ((uint32_t)qword[w * 4 + 2] << 16) |
                       ((uint32_t)qword[w * 4 + 3] << 24);
        }

        /* 1. Clear status/errors */
        swd_write_ap(AP_CSW, ADIV5_AP_CSW_DEFAULT);
        swd_write_ap(AP_TAR, STM32H5_FLASH_CCR);
        swd_write_ap(AP_DRW, STM32H5_FLASH_SR_ERRORS | STM32H5_FLASH_SR_EOP);

        /* 2. Set PG=1 */
        swd_write_ap(AP_TAR, STM32H5_FLASH_CR);
        swd_write_ap(AP_DRW, STM32H5_FLASH_CR_PG);

        /* 3. Program four 32-bit words (128 bits) consecutively */
        for (int w = 0; w < 4; w++) {
            swd_write_ap(AP_TAR, aligned_addr + w * 4);
            swd_write_ap(AP_DRW, words[w]);
        }

        /* 4. Wait for BSY to clear and check for errors */
        if (!stm32h5_wait_flash_bsy()) {
            swd_write_ap(AP_TAR, STM32H5_FLASH_CR);
            swd_write_ap(AP_DRW, 0);
            return false;
        }

        offset += bytes_to_copy;
    }

    swd_write_ap(AP_CSW, ADIV5_AP_CSW_DEFAULT);
    swd_write_ap(AP_TAR, STM32H5_FLASH_CR);
    swd_write_ap(AP_DRW, 0);
    return true;
}



/*
 * AgentLA Target Detection
 * Auto-detect target MCU via SWD IDCODE
 */

#ifndef AGENTLA_TARGET_H
#define AGENTLA_TARGET_H

#include "pico/stdlib.h"
#include "swd.h"

/* Target Family IDs (designer bits [31:12])
 * SWD DPIDR format: bits[31:12] = Design, bits[11:1] = PartNo, bit[0] = Reserved
 * STMicroelectronics: JEDEC bank code 0x043, identity 0x43
 * Raspberry Pi: JEDEC bank code 0x483, identity 0x483
 */
#define TARGET_DESIGNER_ST       0x043   /* STMicroelectronics (bits [15:4] of designer field) */
#define TARGET_DESIGNER_RPI      0x483   /* Raspberry Pi (bits [15:4] of designer field) */
#define TARGET_DESIGNER_NXP      0x044   /* NXP */
#define TARGET_DESIGNER_ATMEL    0x010   /* Atmel/Microchip */
#define TARGET_DESIGNER_NORDIC   0x244   /* Nordic Semiconductor */
#define TARGET_DESIGNER_WCH      0x01A   /* WCH (WinChipHead) */

/* STM32 Part IDs (bits [11:1]) - extracted from DBGMCU_IDCODE */
#define TARGET_PART_STM32F1_MD   0x410   /* STM32F103 Medium Density (Blue Pill C8T6: 64K Flash/20K RAM) */
#define TARGET_PART_STM32F1_LD   0x412   /* STM32F103 Low Density (32K Flash/10K RAM) */
#define TARGET_PART_STM32F1_HD   0x414   /* STM32F103 High Density (512K Flash/64K RAM) */
#define TARGET_PART_STM32F1_CL   0x418   /* STM32F105/107 Connectivity Line (256K Flash/64K RAM) */
#define TARGET_PART_STM32F1_XL   0x430   /* STM32F103 XL Density (1024K Flash/96K RAM) */
#define TARGET_PART_STM32F1      TARGET_PART_STM32F1_HD
#define TARGET_PART_STM32F4      0x413   /* STM32F411 */
#define TARGET_PART_STM32G0      0x456   /* STM32G071 */
#define TARGET_PART_STM32L4      0x435   /* STM32L476 */
#define TARGET_PART_STM32H503    0x474   /* STM32H503 (Cortex-M33, 128K Flash / 32K RAM) */
#define TARGET_PART_STM32H5_HD   0x484   /* STM32H562/H563/H573 (Cortex-M33, up to 2MB Flash / 640K RAM) */
#define TARGET_PART_STM32H5_MD   0x478   /* STM32H523/H533 (Cortex-M33, up to 512K Flash / 272K RAM) */

/* RP2040/RP2350 Part IDs */
#define TARGET_PART_RP2040       0x00C   /* RP2040 */
#define TARGET_PART_RP2350       0x00D   /* RP2350 */

/* Nordic & WCH Part IDs */
#define TARGET_PART_NRF52840     0x528   /* nRF52840 */
#define TARGET_PART_CH32V003     0x003   /* CH32V003 */

/* Target Family Enum */
typedef enum {
    TARGET_FAMILY_UNKNOWN = 0,
    TARGET_FAMILY_STM32F1,
    TARGET_FAMILY_STM32F4,
    TARGET_FAMILY_STM32G0,
    TARGET_FAMILY_STM32L4,
    TARGET_FAMILY_STM32H5,
    TARGET_FAMILY_RP2040,
    TARGET_FAMILY_RP2350,
    TARGET_FAMILY_NRF52,
    TARGET_FAMILY_CH32
} target_family_t;

/* Target Info Structure */
typedef struct {
    uint32_t idcode;
    uint16_t designer_id;
    uint16_t part_id;
    target_family_t family;
    const char *name;
    uint32_t flash_size;
    uint32_t flash_base;
    uint32_t ram_size;
    uint32_t ram_base;
} target_info_t;

/* Flash Algorithm Interface */
typedef struct {
    const char *name;
    target_family_t family;
    bool (*flash_erase)(uint32_t addr, uint32_t size);
    bool (*flash_write)(uint32_t addr, const uint8_t *data, uint32_t len);
    bool (*flash_read)(uint32_t addr, uint8_t *data, uint32_t len);
    bool (*flash_verify)(uint32_t addr, const uint8_t *data, uint32_t len);
} flash_algorithm_t;

/* Global target state */
extern target_info_t current_target;
extern const flash_algorithm_t *current_algorithm;

/* ============================================================================
 * Target Detection
 * ============================================================================ */

/**
 * Detect connected target via SWD IDCODE.
 * @return SWD_OK if target detected, error code otherwise
 */
swd_result_t target_detect(void);

/**
 * Get current target info.
 * @return Pointer to target_info_t structure
 */
const target_info_t *target_get_info(void);

/**
 * Check if target is connected.
 * @return true if target detected, false otherwise
 */
bool target_is_connected(void);

/**
 * Get target family name as string.
 * @param family Target family enum
 * @return String name of family
 */
const char *target_family_name(target_family_t family);

/* ============================================================================
 * Flash Algorithm Selection
 * ============================================================================ */

/**
 * Get flash algorithm for current target.
 * @return Pointer to flash_algorithm_t, or NULL if not supported
 */
const flash_algorithm_t *target_get_flash_algorithm(void);

/**
 * Flash erase operation.
 * @param addr Start address
 * @param size Size in bytes
 * @return true on success
 */
bool target_flash_erase(uint32_t addr, uint32_t size);
bool target_mass_erase(void);
bool target_is_rdp_protected(void);
bool target_is_rdp_protected(void);
bool target_lock_rdp_level1(void);
bool target_unlock_rdp(void);
bool target_nordic_mass_erase(void);

/**
 * Flash write operation.
 * @param addr Start address
 * @param data Data to write
 * @param len Data length
 * @return true on success
 */
bool target_flash_write(uint32_t addr, const uint8_t *data, uint32_t len);

/**
 * Flash read operation.
 * @param addr Start address
 * @param data Buffer to read into
 * @param len Data length
 * @return true on success
 */
bool target_flash_read(uint32_t addr, uint8_t *data, uint32_t len);

#endif /* AGENTLA_TARGET_H */

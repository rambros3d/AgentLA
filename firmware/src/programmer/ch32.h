/*
 * WCH CH32 / RISC-V Driver & 1-Wire SDI Interface
 */

#ifndef CH32_H
#define CH32_H

#include <stdint.h>
#include <stdbool.h>
#include "pico/stdlib.h"
#include "hardware/pio.h"
#include "board_settings.h"

#define WCH_SDI_PIN       5    /* GPIO 5 (SWDIO) */
#define WCH_SDI_PIO       pio1
#define WCH_SDI_SM        1    /* SM1 (SM0 used by SWD) */

typedef enum {
    CH32_OK = 0,
    CH32_ERR_TIMEOUT,
    CH32_ERR_NACK,
    CH32_ERR_UNSUPPORTED,
    CH32_ERR_FLASH
} ch32_result_t;

/* Transport (wire protocol) selection. These are distinct protocols:
 *   SWIO   — WCH 1-wire serial wire IO, CH32V003 only (swio.c/swio.pio)
 *   RVSWD  — WCH 2-wire RISC-V SWD variant, CH32V20x/V30x (rvswd.c)
 * ARM SWD (Cortex-M, swd.c/target.c) is deliberately NOT part of this
 * enum — it has its own module and command family. */
typedef enum {
    CH32_MODE_SWIO = 0,   /* 1-wire SWIO (CH32V003) */
    CH32_MODE_RVSWD = 1   /* 2-wire RVSWD (CH32V203 / V307) */
} ch32_mode_t;

/* CH32 chip variants — chip properties (DTM offsets, flash controller,
 * page size) are keyed on chip_type, NOT on the transport mode. */
typedef enum {
    CH32_CHIP_UNKNOWN = 0,
    CH32_CHIP_V003,           /* QingKe V2A core, 1-wire SWIO */
    CH32_CHIP_V203,           /* QingKe V4A core, 2-wire RVSWD */
    CH32_CHIP_V307,           /* QingKe V4A core, 2-wire RVSWD */
    CH32_CHIP_F103,           /* ARM Cortex-M3 (Blue Pill compatible) */
} ch32_chip_t;

/* CH32V20x flash register addresses (different from V003) */
#define CH32V20x_FLASH_KEYR    0x40022004
#define CH32V20x_FLASH_MODEKEYR 0x40022024
#define CH32V20x_FLASH_STATR   0x4002200C
#define CH32V20x_FLASH_CTLR    0x40022010
#define CH32V20x_FLASH_ADDR    0x40022014
#define CH32V20x_FLASH_CTLR2   0x40022018

/* CH32V20x flash control bits */
#define CH32V20x_CTLR_PG       (1U << 0)
#define CH32V20x_CTLR_PER      (1U << 1)
#define CH32V20x_CTLR_MER      (1U << 2)
#define CH32V20x_CTLR_STRT     (1U << 6)
#define CH32V20x_CTLR_FLOCK    (1U << 15)
#define CH32V20x_CTLR_FTPG     (1U << 16)
#define CH32V20x_CTLR_FTER     (1U << 17)
#define CH32V20x_CTLR_BUFLOAD  (1U << 18)
#define CH32V20x_CTLR_BUFRST   (1U << 19)

typedef struct {
    uint32_t chip_id;
    uint32_t flash_size_bytes;
    uint32_t page_size;
    ch32_mode_t mode;
    ch32_chip_t chip_type;
    bool probed;
} ch32_target_info_t;

extern ch32_target_info_t g_ch32_target;

/*
 * MCU model table entry — used for manual MCU selection.
 * The user tells us which MCU is connected, so the programmer is chosen
 * from the model (no unreliable auto-detection needed).
 */
typedef struct {
    const char *name;       /* CLI token, e.g. "ch32v203" */
    const char *label;      /* display name, e.g. "CH32V203" */
    ch32_chip_t chip;
    ch32_mode_t mode;       /* debug transport: SWIO (1-wire) or RVSWD (2-wire) */
    uint32_t flash_size;    /* flash size in bytes */
    uint32_t page_size;     /* flash page size in bytes */
    uint32_t esig_addr;     /* ESIG register for chip id (informational) */
    uint32_t chip_id;       /* canonical chip id for this model */
} ch32_mcu_info_t;

/*
 * Look up an MCU model by CLI name (e.g. "ch32v203").
 * Returns NULL if the name is not in the table.
 */
const ch32_mcu_info_t *ch32_lookup_mcu(const char *name);

/*
 * Manual MCU selection: configure the target from a model table entry,
 * bring up the debug link, and verify the target answers.
 * Sets g_ch32_target.* on success.
 */
ch32_result_t ch32_probe_mcu(const ch32_mcu_info_t *mcu);

/* Low-level 1-wire SDI operations */
ch32_result_t ch32_sdi_init(void);
void ch32_sdi_deinit(void);
void ch32_sdi_reset(void);
void ch32_sdi_send_bit(uint8_t bit);
uint8_t ch32_sdi_read_bit(void);
void ch32_sdi_send_byte(uint8_t byte);
uint8_t ch32_sdi_read_byte(void);
void ch32_sdi_send_word(uint32_t word);
uint32_t ch32_sdi_read_word(void);

/* Probe and Flash Driver API */
ch32_result_t ch32_probe_target(ch32_mode_t mode);
ch32_result_t ch32_flash_unlock(void);
ch32_result_t ch32_flash_erase(uint32_t addr, uint32_t len);
ch32_result_t ch32_flash_program(uint32_t addr, const uint8_t *data, uint32_t len);

/* CH32V20x specific flash functions */
ch32_result_t ch32_v20x_flash_unlock(void);
ch32_result_t ch32_v20x_flash_erase(uint32_t addr, uint32_t len);
ch32_result_t ch32_v20x_flash_program(uint32_t addr, const uint8_t *data, uint32_t len);

/* CH32V20x microprocessor debug control (RISC-V DM) */
ch32_result_t ch32v20x_halt_microprocessor(void);
ch32_result_t ch32v20x_resume_microprocessor(void);
ch32_result_t ch32v20x_reset_microprocessor_and_run(void);

#endif /* CH32_H */

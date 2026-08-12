/*
 * CH32 PROGBUF Flash Injection Module
 *
 * High-speed flash programming using RISC-V Program Buffer (PROGBUF).
 * Instead of slow System Bus Access (SBA) for each word, this module
 * injects a small RISC-V program into PROGBUF0-7 that directly
 * accesses flash control registers, achieving ~3x faster writes.
 *
 * Based on PicoRVD's WCHFlash.cpp approach.
 */

#ifndef CH32_PROGBUF_H
#define CH32_PROGBUF_H

#include <stdint.h>
#include <stdbool.h>
#include "ch32.h"

/* Flash register addresses (CH32V003/V203 common) */
#define PROGBUF_FLASH_BASE    0x40022000
#define PROGBUF_FLASH_KEYR    (PROGBUF_FLASH_BASE + 0x04)
#define PROGBUF_FLASH_STATR   (PROGBUF_FLASH_BASE + 0x0C)
#define PROGBUF_FLASH_CTLR    (PROGBUF_FLASH_BASE + 0x10)
#define PROGBUF_FLASH_ADDR    (PROGBUF_FLASH_BASE + 0x14)
#define PROGBUF_FLASH_MKEYR   (PROGBUF_FLASH_BASE + 0x24)

/* Flash control bits */
#define FLASH_CTLR_PG         (1U << 0)
#define FLASH_CTLR_PER        (1U << 1)
#define FLASH_CTLR_MER        (1U << 2)
#define FLASH_CTLR_STRT       (1U << 6)
#define FLASH_CTLR_FTPG       (1U << 16)
#define FLASH_CTLR_FTER       (1U << 17)
#define FLASH_CTLR_BUFLOAD    (1U << 18)
#define FLASH_CTLR_BUFRST     (1U << 19)

/* Flash status bits */
#define FLASH_STATR_BUSY      (1U << 0)
#define FLASH_STATR_EOP       (1U << 5)

/* Flash unlock keys */
#define FLASH_KEY1            0x45670123
#define FLASH_KEY2            0xCDEF89AB

/**
 * Initialize PROGBUF flash injection.
 * Verifies PROGBUF support and saves register state.
 * @return CH32_OK on success
 */
ch32_result_t ch32_progbuf_init(void);

/**
 * Deinitialize PROGBUF flash injection.
 * Restores saved register state.
 */
void ch32_progbuf_deinit(void);

/**
 * Check if PROGBUF flash injection is available.
 * @return true if PROGBUF is supported and initialized
 */
bool ch32_progbuf_is_available(void);

/**
 * Erase flash using PROGBUF injection.
 * @param addr Start address (must be page-aligned)
 * @param len Length in bytes
 * @return CH32_OK on success
 */
ch32_result_t ch32_progbuf_flash_erase(uint32_t addr, uint32_t len);

/**
 * Program flash using PROGBUF injection.
 * @param addr Destination address (must be page-aligned)
 * @param data Source data buffer
 * @param len Length in bytes (must be multiple of 4)
 * @return CH32_OK on success
 */
ch32_result_t ch32_progbuf_flash_write(uint32_t addr, const uint8_t *data, uint32_t len);

/**
 * Read flash via System Bus Access.
 * @param addr Source address
 * @param data Destination buffer
 * @param len Length in bytes
 * @return CH32_OK on success
 */
ch32_result_t ch32_progbuf_flash_read(uint32_t addr, uint8_t *data, uint32_t len);

/**
 * Program flash using SRAM-based routine (V203/V307 fix).
 * Bypasses PROGBUF by loading the flash write program into target SRAM.
 * @param addr Destination address (must be page-aligned)
 * @param data Source data buffer
 * @param len Length in bytes (must be multiple of 4)
 * @return CH32_OK on success
 */
ch32_result_t ch32_sram_flash_write(uint32_t addr, const uint8_t *data, uint32_t len);

#endif /* CH32_PROGBUF_H */

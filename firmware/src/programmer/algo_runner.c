#include "algo_runner.h"
#include "swd.h"
#include "pico/time.h"
#include <string.h>

#define ALGO_CORTEXM_DHCSR  0xE000EDF0u
#define ALGO_CORTEXM_DCRSR  0xE000EDF4u
#define ALGO_CORTEXM_DCRDR  0xE000EDF8u

#define DHCSR_DBGKEY     0xA05F0000u
#define DHCSR_C_DEBUGEN  0x00000001u
#define DHCSR_C_HALT     0x00000002u
#define DHCSR_S_REGRDY   0x00010000u
#define DHCSR_S_HALT     0x00020000u

static bool algo_mem_write32_checked(uint32_t addr, uint32_t value) {
    if (swd_write_ap(AP_CSW, ADIV5_AP_CSW_DEFAULT) != SWD_OK) return false;
    if (swd_write_ap(AP_TAR, addr) != SWD_OK) return false;
    return swd_write_ap(AP_DRW, value) == SWD_OK;
}

swd_result_t algo_mem_read32(uint32_t addr, uint32_t *value) {
    if (!value) return SWD_ERR_NO_TARGET;
    if (swd_write_ap(AP_CSW, ADIV5_AP_CSW_DEFAULT) != SWD_OK) return SWD_ERR_NO_TARGET;
    if (swd_write_ap(AP_TAR, addr) != SWD_OK) return SWD_ERR_NO_TARGET;
    return swd_read_ap(AP_DRW, value);
}

swd_result_t algo_mem_write32(uint32_t addr, uint32_t value) {
    return algo_mem_write32_checked(addr, value) ? SWD_OK : SWD_ERR_NO_TARGET;
}

swd_result_t algo_ram_download(uint32_t addr, const uint8_t *src, uint32_t len) {
    if (!src || len == 0) return SWD_ERR_NO_TARGET;
    uint32_t pos = 0;
    while (pos + 4 <= len) {
        uint32_t w = src[pos] | ((uint32_t)src[pos + 1] << 8) |
                     ((uint32_t)src[pos + 2] << 16) | ((uint32_t)src[pos + 3] << 24);
        if (algo_mem_write32(addr + pos, w) != SWD_OK) return SWD_ERR_NO_TARGET;
        pos += 4;
    }
    if (pos < len) {
        uint32_t w = 0;
        uint32_t rem = len - pos;
        for (uint32_t i = 0; i < rem; i++) w |= ((uint32_t)src[pos + i]) << (8 * i);
        if (algo_mem_write32(addr + pos, w) != SWD_OK) return SWD_ERR_NO_TARGET;
    }
    return SWD_OK;
}

swd_result_t algo_core_halt(void) {
    if (swd_write_ap(AP_CSW, ADIV5_AP_CSW_DEFAULT) != SWD_OK) return SWD_ERR_NO_TARGET;
    if (algo_mem_write32(ALGO_CORTEXM_DHCSR, DHCSR_DBGKEY | DHCSR_C_DEBUGEN | DHCSR_C_HALT) != SWD_OK) return SWD_ERR_NO_TARGET;
    for (int i = 0; i < 1000; i++) {
        uint32_t dhcsr = 0;
        if (algo_mem_read32(ALGO_CORTEXM_DHCSR, &dhcsr) == SWD_OK &&
            (dhcsr & DHCSR_S_HALT)) return SWD_OK;
        sleep_us(10);
    }
    return SWD_ERR_NO_TARGET;
}

swd_result_t algo_core_resume(void) {
    if (algo_mem_write32(ALGO_CORTEXM_DHCSR, DHCSR_DBGKEY | DHCSR_C_DEBUGEN) != SWD_OK) return SWD_ERR_NO_TARGET;
    return SWD_OK;
}

swd_result_t algo_core_write_reg(uint8_t reg, uint32_t value) {
    if (algo_core_halt() != SWD_OK) return SWD_ERR_NO_TARGET;
    if (algo_mem_write32(ALGO_CORTEXM_DCRDR, value) != SWD_OK) return SWD_ERR_NO_TARGET;
    if (algo_mem_write32(ALGO_CORTEXM_DCRSR, (1u << 16) | (reg & 0x1Fu)) != SWD_OK) return SWD_ERR_NO_TARGET;
    for (int i = 0; i < 1000; i++) {
        uint32_t dhcsr = 0;
        if (algo_mem_read32(ALGO_CORTEXM_DHCSR, &dhcsr) == SWD_OK &&
            (dhcsr & DHCSR_S_REGRDY)) return SWD_OK;
        sleep_us(10);
    }
    return SWD_ERR_NO_TARGET;
}

swd_result_t algo_core_read_reg(uint8_t reg, uint32_t *value) {
    if (!value) return SWD_ERR_NO_TARGET;
    if (algo_core_halt() != SWD_OK) return SWD_ERR_NO_TARGET;
    if (algo_mem_write32(ALGO_CORTEXM_DCRSR, (reg & 0x1Fu)) != SWD_OK) return SWD_ERR_NO_TARGET;
    for (int i = 0; i < 1000; i++) {
        uint32_t dhcsr = 0;
        if (algo_mem_read32(ALGO_CORTEXM_DHCSR, &dhcsr) == SWD_OK &&
            (dhcsr & DHCSR_S_REGRDY)) {
            return algo_mem_read32(ALGO_CORTEXM_DCRDR, value);
        }
        sleep_us(10);
    }
    return SWD_ERR_NO_TARGET;
}

swd_result_t algo_exec(uint32_t entry, uint32_t a0, uint32_t a1,
                       uint32_t a2, uint32_t a3, uint32_t *result) {
    if (!result) return SWD_ERR_NO_TARGET;
    if (algo_core_halt() != SWD_OK) return SWD_ERR_NO_TARGET;
    if (algo_core_write_reg(0, a0) != SWD_OK) return SWD_ERR_NO_TARGET;
    if (algo_core_write_reg(1, a1) != SWD_OK) return SWD_ERR_NO_TARGET;
    if (algo_core_write_reg(2, a2) != SWD_OK) return SWD_ERR_NO_TARGET;
    if (algo_core_write_reg(3, a3) != SWD_OK) return SWD_ERR_NO_TARGET;
    if (algo_core_write_reg(14, 0x00000000u) != SWD_OK) return SWD_ERR_NO_TARGET;
    if (algo_core_write_reg(15, entry | 1u) != SWD_OK) return SWD_ERR_NO_TARGET;
    if (algo_core_resume() != SWD_OK) return SWD_ERR_NO_TARGET;
    for (int i = 0; i < 5000; i++) {
        uint32_t dhcsr = 0;
        if (algo_mem_read32(ALGO_CORTEXM_DHCSR, &dhcsr) == SWD_OK &&
            (dhcsr & DHCSR_S_HALT)) break;
        sleep_us(10);
    }
    if (algo_core_read_reg(0, result) != SWD_OK) return SWD_ERR_NO_TARGET;
    return SWD_OK;
swd_result_t algo_runner_init(const algo_blob_header_t *hdr, uint32_t code_base,
                              uint32_t flash_base, uint32_t clk, uint32_t *result) {
    if (!hdr || !result) return SWD_ERR_NO_TARGET;
    uint32_t r = 0;
    swd_result_t rc = algo_invoke(hdr, code_base, hdr->init_offset, flash_base, clk, 0, 0, &r);
    *result = r;
    return rc;
}

swd_result_t algo_runner_erase_sector(const algo_blob_header_t *hdr, uint32_t code_base,
                                      uint32_t sector_addr, uint32_t *result) {
    if (!hdr || !result) return SWD_ERR_NO_TARGET;
    uint32_t r = 0;
    swd_result_t rc = algo_invoke(hdr, code_base, hdr->erase_offset, sector_addr, 0, 0, 0, &r);
    *result = r;
    return rc;
}

swd_result_t algo_runner_program_page(const algo_blob_header_t *hdr, uint32_t code_base,
                                      uint32_t page_addr, uint32_t size,
                                      uint32_t src_addr, uint32_t *result) {
    if (!hdr || !result) return SWD_ERR_NO_TARGET;
    uint32_t r = 0;
    swd_result_t rc = algo_invoke(hdr, code_base, hdr->program_offset, page_addr, size, src_addr, 0, &r);
    *result = r;
    return rc;
}

swd_result_t algo_runner_verify(const algo_blob_header_t *hdr, uint32_t code_base,
                               uint32_t addr, uint32_t size, uint32_t src_addr,
                               uint32_t *result) {
    if (!hdr || !result) return SWD_ERR_NO_TARGET;
    if (hdr->verify_offset == 0) { *result = 0; return SWD_OK; }
    uint32_t r = 0;
    swd_result_t rc = algo_invoke(hdr, code_base, hdr->verify_offset, addr, size, src_addr, 0, &r);
    *result = r;
    return rc;
}

swd_result_t algo_runner_uninit(const algo_blob_header_t *hdr, uint32_t code_base,
                               uint32_t flags, uint32_t *result) {
    if (!hdr || !result) return SWD_ERR_NO_TARGET;
    uint32_t r = 0;
    swd_result_t rc = algo_invoke(hdr, code_base, hdr->uninit_offset, flags, 0, 0, 0, &r);
    *result = r;
    return rc;
}


swd_result_t algo_invoke(const algo_blob_header_t *hdr, uint32_t code_base,
                         uint32_t entry_offset, uint32_t r0, uint32_t r1,
                         uint32_t r2, uint32_t r3, uint32_t *result) {
    if (!hdr || !result) return SWD_ERR_NO_TARGET;
    uint32_t entry = code_base + entry_offset;
    return algo_exec(entry, r0, r1, r2, r3, result);
}

}

#ifndef AGENTLA_ALGO_RUNNER_H
#define AGENTLA_ALGO_RUNNER_H

#include "swd.h"
#include <stdint.h>
#include <stddef.h>

/* AgentLA blob header — kept in target SRAM alongside the code it describes. */
typedef struct {
    uint32_t magic;          /* "ALGO" */
    uint32_t version;        /* 1 */
    uint32_t code_size;      /* bytes of executable Thumb code following */
    uint32_t init_offset;    /* offset within code of Init()             */
    uint32_t erase_offset;   /* offset of EraseSector()                 */
    uint32_t program_offset; /* offset of ProgramPage()                 */
    uint32_t verify_offset;  /* offset of Verify() (0 if absent)        */
    uint32_t uninit_offset;  /* offset of UnInit()                      */
    uint32_t ram_base;       /* free SRAM start for data buffers         */
    uint32_t ram_size;       /* bytes available in that window           */
    uint32_t stack_top;      /* initial SP value to seed before calling  */
} algo_blob_header_t;

#define ALGO_MAGIC     0x4F474C41u /* "ALGO" little-endian */
#define ALGO_VERSION   1u

swd_result_t algo_mem_read32(uint32_t addr, uint32_t *value);
swd_result_t algo_mem_write32(uint32_t addr, uint32_t value);
swd_result_t algo_ram_download(uint32_t addr, const uint8_t *src, uint32_t len);

swd_result_t algo_core_halt(void);
swd_result_t algo_core_resume(void);
swd_result_t algo_core_write_reg(uint8_t reg, uint32_t value);
swd_result_t algo_core_read_reg(uint8_t reg, uint32_t *value);

/* Execute one of the algorithm entry points described by a blob header. */
swd_result_t algo_invoke(const algo_blob_header_t *hdr, uint32_t code_base,
                         uint32_t entry_offset, uint32_t r0, uint32_t r1,
                         uint32_t r2, uint32_t r3, uint32_t *result);

/* High-level orchestration over a loaded blob. */
swd_result_t algo_runner_init(const algo_blob_header_t *hdr, uint32_t code_base,
                              uint32_t flash_base, uint32_t clk, uint32_t *result);
swd_result_t algo_runner_erase_sector(const algo_blob_header_t *hdr, uint32_t code_base,
                                      uint32_t sector_addr, uint32_t *result);
swd_result_t algo_runner_program_page(const algo_blob_header_t *hdr, uint32_t code_base,
                                      uint32_t page_addr, uint32_t size,
                                      uint32_t src_addr, uint32_t *result);
swd_result_t algo_runner_verify(const algo_blob_header_t *hdr, uint32_t code_base,
                               uint32_t addr, uint32_t size, uint32_t src_addr,
                               uint32_t *result);
swd_result_t algo_runner_uninit(const algo_blob_header_t *hdr, uint32_t code_base,
                               uint32_t flags, uint32_t *result);

#endif /* AGENTLA_ALGO_RUNNER_H */
/*
 * RISC-V Debug Module Interface (DMI) Register Definitions & API
 */

#ifndef AGENTLA_RISCV_DMI_H
#define AGENTLA_RISCV_DMI_H

#include <stdint.h>
#include <stdbool.h>
#include "ch32.h"

/* DMI Register Addresses (RISC-V Debug Spec 0.13 / WCH RISC-V DM) */
#define DMI_DATA0        0x04
#define DMI_DATA1        0x05
#define DMI_DMCONTROL    0x10
#define DMI_DMSTATUS     0x11
#define DMI_HARTINFO     0x12
#define DMI_HAWINDOWSEL  0x14
#define DMI_HAWINDOW     0x15
#define DMI_ABSTRACTCS   0x16
#define DMI_COMMAND      0x17
#define DMI_ABSTRACTAUTO 0x18
#define DMI_CONFSTRPTR0  0x19
#define DMI_NEXTDM       0x1D
#define DMI_CUSTOM       0x1F
#define DMI_PROGBUF0     0x20
#define DMI_PROGBUF1     0x21
#define DMI_PROGBUF2     0x22
#define DMI_PROGBUF3     0x23
#define DMI_PROGBUF4     0x24
#define DMI_PROGBUF5     0x25
#define DMI_PROGBUF6     0x26
#define DMI_PROGBUF7     0x27
#define DMI_SBCS         0x38
#define DMI_SBADDRESS0   0x39
#define DMI_SBDATA0      0x3C

/* DMCONTROL Bits */
#define DMCONTROL_HALTREQ      (1U << 31)
#define DMCONTROL_RESUMEREQ    (1U << 30)
#define DMCONTROL_HARTRESET    (1U << 29)
#define DMCONTROL_ACKHAVERESET (1U << 28)
#define DMCONTROL_NDMRESET     (1U << 1)
#define DMCONTROL_DMACTIVE     (1U << 0)

/* DMSTATUS Bits */
#define DMSTATUS_IMPEBREAK     (1U << 22)
#define DMSTATUS_ALLHAVERESET  (1U << 19)
#define DMSTATUS_ANYHAVERESET  (1U << 18)
#define DMSTATUS_ALLRESUMED    (1U << 17)
#define DMSTATUS_ANYRESUMED    (1U << 16)
#define DMSTATUS_ALLNONEXISTENT (1U << 15)
#define DMSTATUS_ANYNONEXISTENT (1U << 14)
#define DMSTATUS_ALLUNAVAIL    (1U << 13)
#define DMSTATUS_ANYUNAVAIL    (1U << 12)
#define DMSTATUS_ALLRUNNING    (1U << 11)
#define DMSTATUS_ANYRUNNING    (1U << 10)
#define DMSTATUS_ALLHALTED     (1U << 9)
#define DMSTATUS_ANYHALTED     (1U << 8)
#define DMSTATUS_AUTHENTICATED (1U << 7)

/* SBCS Bits (System Bus Access) */
#define SBCS_SBBUSY          (1U << 21)
#define SBCS_SBREADONADDR    (1U << 20)
#define SBCS_SBACCESS_32     (2U << 17)   /* 32-bit bus access */
#define SBCS_SBERROR_MASK    (0x7U << 12)

/* ABSTRACTCS Bits */
#define ABSTRACTCS_BUSY        (1U << 12)
#define ABSTRACTCS_CMDERR_MASK (0x7U << 8)
#define ABSTRACTCS_ERR_MASK    (0x7U << 8)
#define ABSTRACTCS_ERR_NONE    (0U << 8)
#define ABSTRACTCS_ERR_NOTSUPP (1U << 8)
#define ABSTRACTCS_ERR_BUSY    (2U << 8)
#define ABSTRACTCS_ERR_HALT    (3U << 8)
#define ABSTRACTCS_PROGBUFSIZE_MASK (0x1FU << 24)

/* ABSTRACTAUTO Bits */
#define ABSTRACTAUTO_AUTOEXECDATA   (1U << 0)
#define ABSTRACTAUTO_AUTOEXECPROG   (1U << 16)

/* COMMAND Register Constants (Access Register Command) */
#define CMD_TYPE_ACCESS_REG    (0x00U << 24)
#define CMD_TRANSFER           (1U << 17)
#define CMD_WRITE              (1U << 16)
#define CMD_POSTEXEC           (1U << 18)
#define CMD_AARSIZE_32         (2U << 20)
#define CMD_REGNO(r)           ((uint32_t)(r) & 0xFFFFU)

/* RISC-V GPR Register Numbers */
#define RISCV_REG_X0           0x1000
#define RISCV_REG_X1_RA        0x1001
#define RISCV_REG_X2_SP        0x1002
#define RISCV_REG_PC           0x7B1

/* DMI Transfer functions */
ch32_result_t riscv_dmi_write(uint8_t reg, uint32_t data);
ch32_result_t riscv_dmi_read(uint8_t reg, uint32_t *data);

/* Core control API */
ch32_result_t riscv_halt(void);
ch32_result_t riscv_resume(void);
ch32_result_t riscv_is_halted(bool *halted);
ch32_result_t riscv_read_reg(uint16_t regno, uint32_t *val);
ch32_result_t riscv_write_reg(uint16_t regno, uint32_t val);
ch32_result_t riscv_read_mem(uint32_t addr, uint32_t *val);
ch32_result_t riscv_write_mem(uint32_t addr, uint32_t val);
ch32_result_t riscv_write_hword(uint32_t addr, uint16_t val);

#endif /* AGENTLA_RISCV_DMI_H */

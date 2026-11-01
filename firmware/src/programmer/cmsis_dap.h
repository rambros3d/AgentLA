/**
 * @file cmsis_dap.h
 * @brief CMSIS-DAP v1 protocol implementation
 */
#ifndef AGENTLA_CMSIS_DAP_H
#define AGENTLA_CMSIS_DAP_H

#include <stdint.h>
#include <stdbool.h>

/* CMSIS-DAP Command IDs */
#define DAP_INFO_ID                 0x00
#define DAP_LED                     0x01
#define DAP_CONNECT                 0x02
#define DAP_DISCONNECT              0x03
#define DAP_TRANSFER_CONFIGURE      0x04
#define DAP_TRANSFER                0x05
#define DAP_TRANSFER_BLOCK          0x06
#define DAP_TRANSFER_ABORT          0x07
#define DAP_WRITE_ABORT            0x08
#define DAP_DELAY                   0x09
#define DAP_RESET_TARGET            0x0A
#define DAP_SWJ_PINS               0x10
#define DAP_SWJ_CLOCK              0x11
#define DAP_SWJ_SEQUENCE           0x12
#define DAP_SWD_CONFIGURE          0x13
#define DAP_JTAG_SEQUENCE          0x14
#define DAP_JTAG_CONFIGURE         0x15
#define DAP_JTAG_IDCODE            0x16
#define DAP_SWO_TRANSPORT          0x17
#define DAP_SWO_MODE               0x18
#define DAP_SWO_BAUDRATE           0x19
#define DAP_SWO_CONTROL            0x1A
#define DAP_SWO_STATUS             0x1B
#define DAP_SWO_DATA               0x1C
#define DAP_SWD_SEQUENCE           0x1D
#define DAP_SWO_EXTENDED_STATUS    0x1E
#define DAP_QUEUE_COMMANDS         0x7E
#define DAP_EXECUTE_COMMANDS       0x7F

/* DAP_Info IDs per CMSIS-DAP spec (0x01-0x0F for standard info, 0xF0-0xFF for extended) */
#define DAP_INFO_ID_VENDOR          0x01
#define DAP_INFO_ID_PRODUCT         0x02
#define DAP_INFO_ID_SER_NUM         0x03
#define DAP_INFO_ID_FW_VER          0x04
#define DAP_INFO_ID_HW_VER          0x05
#define DAP_INFO_ID_MAX_PACKET_SIZE    0xFF
#define DAP_INFO_ID_MAX_TRANSFER_COUNT 0xFE
#define DAP_INFO_ID_SUPPORTED_PROTOCOLS 0xF0

/* SWD Protocol */
#define DAP_PORT_SWD               0x01
#define DAP_PORT_JTAG              0x02

/* Transfer Request bits */
#define DAP_TRANSFER_APnDP         0x01
#define DAP_TRANSFER_RnW           0x02
#define DAP_TRANSFER_MATCH         0x10

/* Transfer Response (Bits [2:0] = ACK value, Bit 3 = Protocol Error, Bit 4 = Mismatch) */
#define DAP_TRANSFER_OK            0x01
#define DAP_TRANSFER_WAIT          0x02
#define DAP_TRANSFER_FAULT         0x04
#define DAP_TRANSFER_NO_ACK        0x07
#define DAP_TRANSFER_ERROR         0x08
#define DAP_TRANSFER_MISMATCH      0x10

/* Process a CMSIS-DAP command and produce a response */
uint16_t cmsis_dap_process(const uint8_t *request, uint8_t *response, uint16_t req_len);

#endif /* AGENTLA_CMSIS_DAP_H */

/**
 * @file adapter_i2c_usb.h
 * @brief Native i2c-tiny-usb driverless endpoint handler for AgentLA
 */

#ifndef __AGENTLA_ADAPTER_I2C_USB__
#define __AGENTLA_ADAPTER_I2C_USB__

#include "pico/stdlib.h"
#include "tusb.h"

/* i2c-tiny-usb commands */
#define CMD_ECHO          0
#define CMD_GET_FUNC      1
#define CMD_SET_DELAY     2
#define CMD_GET_STATUS    3
#define CMD_I2C_IO        4

#define CMD_I2C_IO_BEGIN  (1 << 0)
#define CMD_I2C_IO_END    (1 << 1)

#define STATUS_IDLE        0
#define STATUS_ADDRESS_ACK 1
#define STATUS_ADDRESS_NAK 2

#define I2C_FUNC_I2C            0x00000001
#define I2C_FUNC_SMBUS_EMUL     0x0eff0000

void adapter_i2c_usb_init(void);
void adapter_i2c_usb_standby(void);
bool adapter_i2c_usb_auto_claim(void);
bool adapter_i2c_usb_control_xfer(uint8_t rhport, uint8_t stage, tusb_control_request_t const * request);

#endif

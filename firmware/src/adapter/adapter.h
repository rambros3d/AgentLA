#ifndef __AGENTLA_ADAPTER__
#define __AGENTLA_ADAPTER__

#include "pico/stdlib.h"
#include "hardware/i2c.h"
#include "hardware/uart.h"

#define I2C_ADAPTER_FREQ_HZ 10000
#define I2C_BUS_STUCK_TIMEOUT_MS 100
#define I2C_CLOCK_STRETCH_LIMIT_MS 100
#define UART_ADAPTER_DEFAULT_BAUD 115200
#define UART_RX_BUFFER_SIZE 256

typedef enum {
    ADAPTER_IDLE,
    ADAPTER_I2C_BUSY,
    ADAPTER_UART_BUSY,
} ADAPTER_STATE;

void adapter_init(void);
void adapter_init_i2c_only(void);
void adapter_i2c_init(void);
void adapter_uart_init(void);
void adapter_uart_init_lazy(void);
void adapter_poll(void);

bool adapter_i2c_read(uint8_t addr, uint8_t reg, uint8_t len, uint8_t *data, uint32_t timeout_ms);
bool adapter_i2c_write(uint8_t addr, uint8_t reg, const uint8_t *data, uint8_t len, uint32_t timeout_ms);
bool adapter_i2c_bind_claim(uint32_t handle);
bool adapter_uart_bind_claim(uint32_t handle);
bool adapter_i2c_clear_bus(void);
bool adapter_uart_write(const uint8_t *data, uint8_t len, uint32_t baud);
int adapter_uart_read(uint8_t *data, uint8_t max_len, uint32_t baud, uint32_t timeout_ms);

#endif
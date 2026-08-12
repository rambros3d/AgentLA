#ifndef __AGENTLA_DECODE__
#define __AGENTLA_DECODE__

#include "structs.h"
#include <stdint.h>
#include <stdbool.h>

#define DECODE_MAX_EVENTS 512
#define DECODE_TEXT_LEN 64

typedef enum {
    EVT_I2C_START,
    EVT_I2C_STOP,
    EVT_I2C_ADDR,
    EVT_I2C_ACK,
    EVT_I2C_NACK,
    EVT_I2C_DATA,
    EVT_SPI_CS_ASSERT,
    EVT_SPI_CS_DEASSERT,
    EVT_SPI_MOSI,
    EVT_SPI_MISO,
    EVT_UART_DATA,
    EVT_UART_BREAK,
    EVT_UART_ERR_PARITY,
    EVT_UART_ERR_FRAMING,
    EVT_PWM_SIGNAL,
    EVT_PWM_NO_SIGNAL
} EVENT_TYPE;

typedef struct {
    EVENT_TYPE type;
    float timestamp;
    uint8_t pin;
    uint8_t value;
    uint8_t ack;
    uint8_t rw;
    char text[DECODE_TEXT_LEN];
} DECODE_EVENT;

typedef struct {
    DECODE_EVENT events[DECODE_MAX_EVENTS];
    uint16_t count;
} DECODE_RESULT;

void i2c_decode(const uint8_t* buffer, uint32_t length, uint32_t sample_rate,
                uint8_t sda_pin, uint8_t scl_pin, uint8_t pin_base,
                uint32_t buf_offset, DECODE_RESULT* result);

void spi_decode(const uint8_t* buffer, uint32_t length, uint32_t sample_rate,
                uint8_t cs_pin, uint8_t sclk_pin, uint8_t mosi_pin, uint8_t miso_pin,
                uint8_t pin_base, uint32_t buf_offset, DECODE_RESULT* result);

void uart_decode(const uint8_t* buffer, uint32_t length, uint32_t sample_rate,
                 uint8_t rx_pin, uint32_t baud_rate, uint8_t pin_base,
                 uint32_t buf_offset, DECODE_RESULT* result);

void pwm_decode(const uint8_t* buffer, uint32_t length, uint32_t sample_rate,
                uint8_t pin, uint8_t pin_base, uint32_t buf_offset, DECODE_RESULT* result);

#endif

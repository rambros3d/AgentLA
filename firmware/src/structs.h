#ifndef __AGENTLA_STRUCTS__
#define __AGENTLA_STRUCTS__

#include "pico/stdlib.h"
#include "board_settings.h"

typedef enum {
    MODE_8_CHANNEL,
    MODE_16_CHANNEL
} CHANNEL_MODE;

typedef enum {
    TRIGGER_NONE,
    TRIGGER_RISING,
    TRIGGER_FALLING,
    TRIGGER_PATTERN
} TRIGGER_TYPE;

typedef struct {
    uint32_t frequency;
    uint32_t pre_samples;
    uint32_t post_samples;
    uint8_t channels[MAX_CHANNELS];
    uint8_t channel_count;
    TRIGGER_TYPE trigger_type;
    uint8_t trigger_pin;
    uint16_t trigger_pattern;
    uint8_t trigger_pattern_count;
    CHANNEL_MODE capture_mode;
} CAPTURE_CONFIG;

typedef enum {
    DECODE_NONE,
    DECODE_I2C,
    DECODE_SPI,
    DECODE_UART,
    DECODE_PWM,
    DECODE_CAN,
    DECODE_MODBUS,
    DECODE_DMX,
    DECODE_LED_1W,
    DECODE_LED_2W,
    DECODE_1WIRE,
    DECODE_I3C,
    DECODE_SWD,
    DECODE_SWO,
    DECODE_LIN,
    DECODE_SPIFLASH
} DECODE_PROTOCOL;

typedef struct {
    DECODE_PROTOCOL protocol;
    uint8_t sda_pin;
    uint8_t scl_pin;
    uint8_t cs_pin;
    uint8_t sclk_pin;
    uint8_t mosi_pin;
    uint8_t miso_pin;
    uint8_t rx_pin;
    uint32_t baud_rate;
} DECODE_CONFIG;

typedef enum {
    DEVICE_IDLE,
    DEVICE_CAPTURING,
    DEVICE_DECODING
} DEVICE_STATE;

#endif

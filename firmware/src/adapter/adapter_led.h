#ifndef __AGENTLA_ADAPTER_LED_H__
#define __AGENTLA_ADAPTER_LED_H__

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
#include "pico/stdlib.h"

#define LED_MAX_PIXELS 256

typedef enum {
    LED_PROTO_1WIRE = 0,
    LED_PROTO_SPI   = 1
} led_protocol_t;

typedef enum {
    LED_ORDER_GRB = 0,
    LED_ORDER_RGB = 1,
    LED_ORDER_BGR = 2,
    LED_ORDER_GRBW = 3,
    LED_ORDER_RGBW = 4
} led_color_order_t;

typedef struct {
    uint8_t r;
    uint8_t g;
    uint8_t b;
    uint8_t w;
} led_color_t;

/* Transmit addressable LED stream to target pin */
bool adapter_led_transmit(uint8_t pin, uint8_t clk_pin,
                          led_protocol_t proto,
                          led_color_order_t order,
                          bool is_rgbw,
                          float freq,
                          uint8_t brightness,
                          uint32_t reset_us,
                          const led_color_t *pixels,
                          uint16_t num_pixels);

#endif /* __AGENTLA_ADAPTER_LED_H__ */

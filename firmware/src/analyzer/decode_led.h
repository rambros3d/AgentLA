#ifndef __AGENTLA_DECODE_LED__
#define __AGENTLA_DECODE_LED__

#include "decode.h"
#include <stdint.h>
#include <stdbool.h>

typedef enum {
    LED_ORDER_GRB = 0,
    LED_ORDER_RGB,
    LED_ORDER_BGR,
    LED_ORDER_RGBW,
    LED_ORDER_GRBW
} LED_COLOR_ORDER;

void led_decode_1w(const uint8_t* buffer, uint32_t length, uint32_t sample_rate,
                   uint8_t data_pin, uint8_t color_order, uint8_t is_rgbw,
                   uint8_t pin_base, uint32_t buf_offset, DECODER_EVENT_ARENA* arena);

void led_decode_2w(const uint8_t* buffer, uint32_t length, uint32_t sample_rate,
                   uint8_t clk_pin, uint8_t data_pin, uint8_t pin_base,
                   uint32_t buf_offset, DECODER_EVENT_ARENA* arena);

#endif

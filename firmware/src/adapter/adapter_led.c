#include "adapter_led.h"
#include "hardware/pio.h"
#include "hardware/gpio.h"
#include "neopixel.pio.h"
#include <string.h>

static void bitbang_spi_byte(uint8_t mosi, uint8_t sck, uint8_t b) {
    for (int i = 7; i >= 0; i--) {
        gpio_put(mosi, (b >> i) & 1);
        gpio_put(sck, 1);
        tight_loop_contents();
        gpio_put(sck, 0);
        tight_loop_contents();
    }
}

bool adapter_led_transmit(uint8_t pin, uint8_t clk_pin,
                          led_protocol_t proto,
                          led_color_order_t order,
                          bool is_rgbw,
                          float freq,
                          uint8_t brightness,
                          uint32_t reset_us,
                          const led_color_t *pixels,
                          uint16_t num_pixels) {
    if (!pixels || num_pixels == 0 || num_pixels > LED_MAX_PIXELS) return false;
    if (pin >= 30) return false;
    if (reset_us < 50) reset_us = 300;
    if (brightness == 0) brightness = 255;

    if (proto == LED_PROTO_1WIRE) {
        if (freq <= 0.0f) freq = 800000.0f; /* 800 kHz default */

        PIO pio = pio1;
        int sm = pio_claim_unused_sm(pio, false);
        if (sm < 0) {
            pio = pio0;
            sm = pio_claim_unused_sm(pio, false);
            if (sm < 0) return false;
        }

        if (!pio_can_add_program(pio, &neopixel_program)) {
            pio_sm_unclaim(pio, (uint)sm);
            return false;
        }

        uint offset = pio_add_program(pio, &neopixel_program);
        neopixel_program_init(pio, (uint)sm, offset, pin, freq, is_rgbw);

        for (uint16_t i = 0; i < num_pixels; i++) {
            uint32_t r = ((uint32_t)pixels[i].r * brightness) / 255;
            uint32_t g = ((uint32_t)pixels[i].g * brightness) / 255;
            uint32_t b = ((uint32_t)pixels[i].b * brightness) / 255;
            uint32_t w = ((uint32_t)pixels[i].w * brightness) / 255;

            uint32_t val = 0;
            if (is_rgbw) {
                if (order == LED_ORDER_RGBW) {
                    val = (r << 24) | (g << 16) | (b << 8) | w;
                } else { /* Default GRBW */
                    val = (g << 24) | (r << 16) | (b << 8) | w;
                }
            } else {
                if (order == LED_ORDER_RGB) {
                    val = (r << 24) | (g << 16) | (b << 8);
                } else if (order == LED_ORDER_BGR) {
                    val = (b << 24) | (g << 16) | (r << 8);
                } else { /* Default GRB (WS2812B) */
                    val = (g << 24) | (r << 16) | (b << 8);
                }
            }
            pio_sm_put_blocking(pio, (uint)sm, val);
        }

        /* Wait for FIFO and output shift register to drain */
        while (!pio_sm_is_tx_fifo_empty(pio, (uint)sm)) {
            tight_loop_contents();
        }
        sleep_us(10);

        pio_sm_set_enabled(pio, (uint)sm, false);
        pio_remove_program(pio, &neopixel_program, offset);
        pio_sm_unclaim(pio, (uint)sm);

        /* Hardware latch reset gap */
        gpio_init(pin);
        gpio_set_dir(pin, GPIO_OUT);
        gpio_put(pin, 0);
        sleep_us(reset_us);

        return true;
    } else if (proto == LED_PROTO_SPI) {
        if (clk_pin >= 30) return false;

        gpio_init(pin);
        gpio_set_dir(pin, GPIO_OUT);
        gpio_put(pin, 0);

        gpio_init(clk_pin);
        gpio_set_dir(clk_pin, GPIO_OUT);
        gpio_put(clk_pin, 0);

        /* 32-bit Start Frame */
        for (int i = 0; i < 4; i++) {
            bitbang_spi_byte(pin, clk_pin, 0x00);
        }

        /* LED Frames */
        uint8_t global_br = (brightness >> 3) & 0x1F;
        if (global_br == 0) global_br = 1;

        for (uint16_t i = 0; i < num_pixels; i++) {
            bitbang_spi_byte(pin, clk_pin, (uint8_t)(0xE0 | global_br));
            bitbang_spi_byte(pin, clk_pin, pixels[i].b);
            bitbang_spi_byte(pin, clk_pin, pixels[i].g);
            bitbang_spi_byte(pin, clk_pin, pixels[i].r);
        }

        /* End Frame */
        int end_bytes = (num_pixels + 15) / 16;
        for (int i = 0; i < end_bytes; i++) {
            bitbang_spi_byte(pin, clk_pin, 0xFF);
        }

        return true;
    }

    return false;
}

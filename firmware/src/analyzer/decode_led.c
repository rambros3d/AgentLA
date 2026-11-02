#include "decode_led.h"
#include "board_settings.h"
#include <string.h>

static inline bool get_pin(const uint8_t* buf, uint32_t idx, uint8_t pin) {
    return (buf[idx] >> pin) & 1;
}

static inline uint32_t circ_idx(uint32_t offset, uint32_t i, uint32_t size) {
    return (offset + i) % size;
}

static void map_led_colors(const uint8_t* raw, uint8_t order, bool is_rgbw,
                           uint8_t* out_r, uint8_t* out_g, uint8_t* out_b, uint8_t* out_w) {
    *out_r = 0; *out_g = 0; *out_b = 0; *out_w = 0;
    if (is_rgbw) {
        if (order == LED_ORDER_GRBW) {
            *out_g = raw[0];
            *out_r = raw[1];
            *out_b = raw[2];
            *out_w = raw[3];
        } else { /* Default RGBW */
            *out_r = raw[0];
            *out_g = raw[1];
            *out_b = raw[2];
            *out_w = raw[3];
        }
    } else {
        if (order == LED_ORDER_RGB) {
            *out_r = raw[0];
            *out_g = raw[1];
            *out_b = raw[2];
        } else if (order == LED_ORDER_BGR) {
            *out_b = raw[0];
            *out_g = raw[1];
            *out_r = raw[2];
        } else { /* Default GRB (WS2812B) */
            *out_g = raw[0];
            *out_r = raw[1];
            *out_b = raw[2];
        }
    }
}

void led_decode_1w(const uint8_t* buffer, uint32_t length, uint32_t sample_rate,
                   uint8_t data_pin, uint8_t color_order, uint8_t is_rgbw,
                   uint8_t pin_base, uint32_t buf_offset, DECODER_EVENT_ARENA* arena) {
    if (!buffer || length < 50 || !arena || sample_rate == 0) return;

    uint8_t pin = data_pin - pin_base;
    uint32_t reset_threshold = (uint32_t)(50e-6f * sample_rate); /* 50 us reset gap */
    uint32_t violation_threshold = (uint32_t)(1.4e-6f * sample_rate); /* > 1.4 us */
    uint32_t threshold_0_1 = (uint32_t)(0.55e-6f * sample_rate);

    uint8_t bits_per_pixel = is_rgbw ? 32 : 24;
    uint8_t bytes_per_pixel = is_rgbw ? 4 : 3;

    uint16_t pixel_index = 0;
    uint8_t pixel_bytes[4];
    uint8_t bit_count = 0;
    uint8_t cur_byte = 0;
    uint32_t pixel_start_sample = 0;

    uint32_t i = 0;
    while (i < length) {
        /* Wait for rising edge */
        while (i < length && get_pin(buffer, circ_idx(buf_offset, i, CAPTURE_BUFFER_SIZE), pin) == 0) {
            i++;
        }
        if (i >= length) break;

        uint32_t high_start = i;
        /* Measure HIGH pulse duration */
        while (i < length && get_pin(buffer, circ_idx(buf_offset, i, CAPTURE_BUFFER_SIZE), pin) == 1) {
            i++;
        }
        uint32_t high_dur = i - high_start;

        /* Measure LOW duration */
        uint32_t low_start = i;
        while (i < length && get_pin(buffer, circ_idx(buf_offset, i, CAPTURE_BUFFER_SIZE), pin) == 0) {
            i++;
        }
        uint32_t low_dur = i - low_start;

        /* Timing violation check (T_high > 1.4 us) */
        if (high_dur > violation_threshold) {
            float v_ts = (float)high_start / sample_rate;
            float v_end = (float)(high_start + high_dur) / sample_rate;
            uint16_t pulse_ns = (uint16_t)((float)high_dur / sample_rate * 1e9f);
            DECODER_EVENT_V2* ev = decoder_arena_add(arena, DECODE_LED_1W, EVT_LED_TIMING_VIOLATION,
                                                     v_ts, v_end, pin, 1);
            if (ev) {
                ev->led.pulse_ns = pulse_ns;
                ev->led.flags = 1;
            }
        }

        /* Decode bit */
        uint8_t bit = (high_dur >= threshold_0_1) ? 1 : 0;
        if (bit_count == 0) {
            pixel_start_sample = high_start;
            memset(pixel_bytes, 0, sizeof(pixel_bytes));
            cur_byte = 0;
        }

        cur_byte = (cur_byte << 1) | bit;
        bit_count++;

        if ((bit_count % 8) == 0) {
            pixel_bytes[(bit_count / 8) - 1] = cur_byte;
            cur_byte = 0;
        }

        if (bit_count >= bits_per_pixel) {
            /* Pixel complete! */
            uint8_t r, g, b, w;
            map_led_colors(pixel_bytes, color_order, is_rgbw != 0, &r, &g, &b, &w);

            float start_ts = (float)pixel_start_sample / sample_rate;
            float end_ts = (float)i / sample_rate;

            DECODER_EVENT_V2* ev = decoder_arena_add(arena, DECODE_LED_1W, EVT_LED_PIXEL,
                                                     start_ts, end_ts, pin, is_rgbw ? 2 : 0);
            if (ev) {
                ev->led.pixel_index = pixel_index++;
                ev->led.r = r;
                ev->led.g = g;
                ev->led.b = b;
                ev->led.w = w;
                ev->led.brightness = 255;
            }

            bit_count = 0;
        }

        /* Check for Reset gap (> 50 us low) */
        if (low_dur >= reset_threshold) {
            pixel_index = 0;
            bit_count = 0;
        }
    }
}

void led_decode_2w(const uint8_t* buffer, uint32_t length, uint32_t sample_rate,
                   uint8_t clk_pin, uint8_t data_pin, uint8_t pin_base,
                   uint32_t buf_offset, DECODER_EVENT_ARENA* arena) {
    if (!buffer || length < 50 || !arena || sample_rate == 0) return;

    uint8_t clk = clk_pin - pin_base;
    uint8_t dat = data_pin - pin_base;

    bool prev_clk = get_pin(buffer, circ_idx(buf_offset, 0, CAPTURE_BUFFER_SIZE), clk);
    uint32_t cur_word = 0;
    uint8_t bit_count = 0;
    uint16_t pixel_index = 0;
    bool start_frame_seen = false;
    uint32_t word_start_sample = 0;

    for (uint32_t i = 1; i < length; i++) {
        uint32_t idx = circ_idx(buf_offset, i, CAPTURE_BUFFER_SIZE);
        bool cur_clk = get_pin(buffer, idx, clk);

        /* Sample on rising edge of clock */
        if (!prev_clk && cur_clk) {
            bool bit = get_pin(buffer, idx, dat);
            if (bit_count == 0) {
                word_start_sample = i;
            }
            cur_word = (cur_word << 1) | bit;
            bit_count++;

            if (bit_count == 32) {
                if (cur_word == 0x00000000) {
                    start_frame_seen = true;
                    pixel_index = 0;
                } else if (start_frame_seen) {
                    /* Check for APA102 LED frame (starts with 3 bits 111) */
                    if ((cur_word >> 29) == 0x07) {
                        uint8_t brightness = (cur_word >> 24) & 0x1F;
                        uint8_t b = (cur_word >> 16) & 0xFF;
                        uint8_t g = (cur_word >> 8) & 0xFF;
                        uint8_t r = cur_word & 0xFF;

                        float start_ts = (float)word_start_sample / sample_rate;
                        float end_ts = (float)i / sample_rate;

                        DECODER_EVENT_V2* ev = decoder_arena_add(arena, DECODE_LED_2W, EVT_LED_PIXEL,
                                                                 start_ts, end_ts, dat, 0);
                        if (ev) {
                            ev->led.pixel_index = pixel_index++;
                            ev->led.r = r;
                            ev->led.g = g;
                            ev->led.b = b;
                            ev->led.w = 0;
                            ev->led.brightness = brightness;
                        }
                    } else if (cur_word == 0xFFFFFFFF) {
                        start_frame_seen = false;
                    }
                }
                bit_count = 0;
                cur_word = 0;
            }
        }
        prev_clk = cur_clk;
    }
}

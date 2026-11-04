/**
 * @file servo_pwm.c
 * @brief High-Resolution Microsecond PWM & Servo Control Engine for AgentLA
 */

#include "servo_pwm.h"
#include "pin_owner.h"
#include "hardware/pwm.h"
#include "hardware/clocks.h"
#include "hardware/gpio.h"
#include <string.h>
#include <stdio.h>

static servo_channel_t servo_channels[SERVO_MAX_CHANNELS];

void servo_pwm_init(void) {
    memset(servo_channels, 0, sizeof(servo_channels));
}

void servo_pwm_quiesce_all(void) {
    for (int i = 0; i < SERVO_MAX_CHANNELS; i++) {
        if (servo_channels[i].active) {
            servo_detach(servo_channels[i].pin);
        }
    }
}

static servo_channel_t *find_channel(uint8_t pin) {
    for (int i = 0; i < SERVO_MAX_CHANNELS; i++) {
        if (servo_channels[i].active && servo_channels[i].pin == pin) {
            return &servo_channels[i];
        }
    }
    return NULL;
}

static servo_channel_t *find_free_slot(void) {
    for (int i = 0; i < SERVO_MAX_CHANNELS; i++) {
        if (!servo_channels[i].active) {
            return &servo_channels[i];
        }
    }
    return NULL;
}

const servo_channel_t *servo_pwm_get_slot(uint8_t pin) {
    return find_channel(pin);
}

bool servo_pwm_claim(uint8_t pin, uint32_t *claim_handle) {
    if (!pin_owner_is_flexible(pin)) return false;

    servo_channel_t *ch = find_channel(pin);
    if (ch) {
        if (claim_handle) *claim_handle = ch->claim_handle;
        return true;
    }

    ch = find_free_slot();
    if (!ch) return false; /* All 8 channels full */

    uint32_t handle = 0;
    if (!pin_owner_claim_pins(&pin, 1, GPIO_FUNC_PWM, 0, &handle)) {
        return false;
    }

    gpio_set_function(pin, GPIO_FUNC_PWM);
    ch->pin = pin;
    ch->claim_handle = handle;
    ch->active = true;
    ch->freq = SERVO_DEFAULT_FREQ;
    ch->pulse_us = SERVO_DEFAULT_CENTER_US;
    ch->min_us = SERVO_DEFAULT_MIN_US;
    ch->max_us = SERVO_DEFAULT_MAX_US;
    ch->continuous = false;
    ch->angle = 90;

    if (claim_handle) *claim_handle = handle;
    return true;
}

bool servo_pwm_set(uint8_t pin, uint32_t freq, uint32_t pulse_us) {
    if (!pin_owner_is_flexible(pin)) return false;

    servo_channel_t *ch = find_channel(pin);
    if (!ch) {
        if (!servo_pwm_claim(pin, NULL)) {
            return false;
        }
        ch = find_channel(pin);
        if (!ch) return false;
    }

    if (freq < 1) freq = 1;
    if (freq > 100000) freq = 100000;

    uint32_t f_sys = clock_get_hz(clk_sys);
    if (f_sys == 0) f_sys = 150000000;

    float div;
    uint32_t wrap;
    uint32_t level;

    if (freq == 50) {
        /* Standard 1 MHz tick resolution (1 tick = 1 microsecond) */
        div = (float)f_sys / 1000000.0f;
        if (div < 1.0f) div = 1.0f;
        if (div > 255.0f) div = 255.0f;
        wrap = 20000 - 1; /* 20,000 us = 20 ms = 50 Hz */
        level = pulse_us;
        if (level > 20000) level = 20000;
    } else {
        div = (float)f_sys / ((float)freq * 65536.0f);
        if (div < 1.0f) div = 1.0f;
        if (div > 255.0f) div = 255.0f;
        wrap = (uint32_t)((float)f_sys / ((float)freq * div));
        if (wrap > 0) wrap -= 1;
        if (wrap > 65534) wrap = 65534;
        if (wrap < 1) wrap = 1;

        uint32_t period_us = 1000000UL / freq;
        if (period_us == 0) period_us = 1;
        level = ((wrap + 1) * pulse_us) / period_us;
        if (level > wrap + 1) level = wrap + 1;
    }

    uint slice = pwm_gpio_to_slice_num(pin);
    uint chan = pwm_gpio_to_channel(pin);

    pwm_set_clkdiv(slice, div);
    pwm_set_wrap(slice, (uint16_t)wrap);
    pwm_set_chan_level(slice, chan, (uint16_t)level);
    pwm_set_enabled(slice, true);

    ch->freq = freq;
    ch->pulse_us = pulse_us;
    return true;
}

bool servo_pwm_release(uint8_t pin) {
    if (!pin_owner_is_flexible(pin)) return false;

    servo_channel_t *ch = find_channel(pin);
    if (!ch) return true;

    uint slice = pwm_gpio_to_slice_num(pin);
    pwm_set_enabled(slice, false);
    gpio_set_function(pin, GPIO_FUNC_SIO);
    gpio_set_dir(pin, GPIO_IN);

    pin_owner_result_t res = PIN_OWNER_OK;
    pin_owner_release_claim(ch->claim_handle, &res);

    memset(ch, 0, sizeof(*ch));
    return true;
}

bool servo_attach(uint8_t pin, uint16_t min_us, uint16_t max_us, bool continuous) {
    if (!servo_pwm_claim(pin, NULL)) {
        return false;
    }

    servo_channel_t *ch = find_channel(pin);
    if (!ch) return false;

    ch->min_us = (min_us >= 400 && min_us <= 1500) ? min_us : SERVO_DEFAULT_MIN_US;
    ch->max_us = (max_us >= 1500 && max_us <= 3000) ? max_us : SERVO_DEFAULT_MAX_US;
    ch->continuous = continuous;
    ch->freq = SERVO_DEFAULT_FREQ;

    /* Center servo on attach */
    uint32_t center = SERVO_DEFAULT_CENTER_US;
    ch->angle = continuous ? 0 : 90;
    return servo_pwm_set(pin, SERVO_DEFAULT_FREQ, center);
}

bool servo_write_angle(uint8_t pin, int16_t angle) {
    if (!pin_owner_is_flexible(pin)) return false;

    servo_channel_t *ch = find_channel(pin);
    if (!ch) {
        if (!servo_attach(pin, SERVO_DEFAULT_MIN_US, SERVO_DEFAULT_MAX_US, false)) {
            return false;
        }
        ch = find_channel(pin);
        if (!ch) return false;
    }

    uint32_t pulse_us = SERVO_DEFAULT_CENTER_US;

    if (ch->continuous) {
        /* Continuous rotation: angle is throttle (-100 to +100) */
        if (angle < -100) angle = -100;
        if (angle > 100) angle = 100;
        ch->angle = angle;

        /* Deadband between -5% and +5% */
        if (angle >= -5 && angle <= 5) {
            pulse_us = SERVO_DEFAULT_CENTER_US;
        } else if (angle < -5) {
            /* Reverse: -100..-5 -> min_us..1480 */
            pulse_us = ch->min_us + (uint32_t)((int32_t)(angle + 100) * (1480 - ch->min_us) / 95);
        } else {
            /* Forward: 5..100 -> 1520..max_us */
            pulse_us = 1520 + (uint32_t)((int32_t)(angle - 5) * (ch->max_us - 1520) / 95);
        }
    } else {
        /* Positional: angle is 0 to 180 degrees */
        if (angle < 0) angle = 0;
        if (angle > 180) angle = 180;
        ch->angle = angle;

        pulse_us = ch->min_us + (uint32_t)(((int32_t)angle * (ch->max_us - ch->min_us)) / 180);
    }

    return servo_pwm_set(pin, ch->freq, pulse_us);
}

bool servo_write_us(uint8_t pin, uint32_t pulse_us) {
    if (!pin_owner_is_flexible(pin)) return false;

    servo_channel_t *ch = find_channel(pin);
    if (!ch) {
        if (!servo_attach(pin, SERVO_DEFAULT_MIN_US, SERVO_DEFAULT_MAX_US, false)) {
            return false;
        }
        ch = find_channel(pin);
        if (!ch) return false;
    }

    if (pulse_us < ch->min_us) pulse_us = ch->min_us;
    if (pulse_us > ch->max_us) pulse_us = ch->max_us;

    /* Calculate angle equivalent */
    if (ch->max_us > ch->min_us) {
        if (ch->continuous) {
            int32_t diff = (int32_t)pulse_us - SERVO_DEFAULT_CENTER_US;
            ch->angle = (int16_t)(diff * 100 / (ch->max_us - SERVO_DEFAULT_CENTER_US));
        } else {
            ch->angle = (int16_t)(((pulse_us - ch->min_us) * 180) / (ch->max_us - ch->min_us));
        }
    }

    return servo_pwm_set(pin, ch->freq, pulse_us);
}

bool servo_detach(uint8_t pin) {
    return servo_pwm_release(pin);
}

int servo_get_active_channels(servo_channel_t *out_channels, int max_count) {
    if (!out_channels || max_count <= 0) return 0;
    int count = 0;
    for (int i = 0; i < SERVO_MAX_CHANNELS && count < max_count; i++) {
        if (servo_channels[i].active) {
            out_channels[count++] = servo_channels[i];
        }
    }
    return count;
}

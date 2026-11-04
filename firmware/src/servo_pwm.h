/**
 * @file servo_pwm.h
 * @brief High-Resolution Microsecond PWM & Servo Control Engine for AgentLA
 */

#ifndef __AGENTLA_SERVO_PWM_H__
#define __AGENTLA_SERVO_PWM_H__

#include <stdint.h>
#include <stdbool.h>

#define SERVO_MAX_CHANNELS 8
#define SERVO_DEFAULT_FREQ 50
#define SERVO_DEFAULT_MIN_US 500
#define SERVO_DEFAULT_MAX_US 2500
#define SERVO_DEFAULT_CENTER_US 1500

typedef struct {
    uint8_t pin;            /* GPIO pin (10-29) */
    uint32_t claim_handle;  /* Pin ownership claim handle */
    uint32_t freq;          /* Frame rate in Hz (default 50) */
    uint32_t pulse_us;      /* Active pulse width in microseconds */
    int16_t angle;          /* 0-180 (or -100 to +100 for continuous), -1 if raw pulse */
    uint16_t min_us;        /* Min pulse width in microseconds */
    uint16_t max_us;        /* Max pulse width in microseconds */
    bool continuous;        /* True if continuous rotation mode */
    bool active;            /* True if actively claimed & generating pulses */
} servo_channel_t;

void servo_pwm_init(void);
void servo_pwm_quiesce_all(void);

/* Low-level PWM */
bool servo_pwm_claim(uint8_t pin, uint32_t *claim_handle);
bool servo_pwm_set(uint8_t pin, uint32_t freq, uint32_t pulse_us);
bool servo_pwm_release(uint8_t pin);
const servo_channel_t *servo_pwm_get_slot(uint8_t pin);

/* High-level Servo */
bool servo_attach(uint8_t pin, uint16_t min_us, uint16_t max_us, bool continuous);
bool servo_write_angle(uint8_t pin, int16_t angle);
bool servo_write_us(uint8_t pin, uint32_t pulse_us);
bool servo_detach(uint8_t pin);
int servo_get_active_channels(servo_channel_t *out_channels, int max_count);

#endif /* __AGENTLA_SERVO_PWM_H__ */

#ifndef HARDWARE_GPIO_H
#define HARDWARE_GPIO_H
#include <stdbool.h>
#include <stdint.h>
typedef unsigned int uint;
typedef enum gpio_function_t {
    GPIO_FUNC_SPI = 0,
    GPIO_FUNC_UART = 1,
    GPIO_FUNC_I2C = 2,
    GPIO_FUNC_PWM = 3,
    GPIO_FUNC_SIO = 5,
    GPIO_FUNC_PIO0 = 6,
    GPIO_FUNC_PIO1 = 7,
    GPIO_FUNC_PIO2 = 8,
} gpio_function_t;

enum gpio_irq_level {
    GPIO_IRQ_LEVEL_LOW = 0x1u,
    GPIO_IRQ_LEVEL_HIGH = 0x2u,
    GPIO_IRQ_EDGE_FALL = 0x4u,
    GPIO_IRQ_EDGE_RISE = 0x8u,
};

static inline void gpio_set_irq_enabled(uint pin, uint32_t event_mask, bool enabled) {
    (void)pin; (void)event_mask; (void)enabled;
}
#endif

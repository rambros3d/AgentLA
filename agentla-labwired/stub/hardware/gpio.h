#ifndef HARDWARE_GPIO_H
#define HARDWARE_GPIO_H
#include <stdbool.h>
#include <stdint.h>
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
#endif

#include <stdio.h>
#include <string.h>
#include "pico/stdlib.h"
#include "hardware/clocks.h"
#include "hardware/pwm.h"
#include "hardware/uart.h"
#include "hardware/pio.h"
#include "hardware/gpio.h"
#include "hardware/i2c.h"
#include "pico/i2c_slave.h"

#define PICO2_LED_PIN       25
#define PIN_PWM_10KHZ_CLK   20
#define PIN_PWM_1KHZ_TEST   21
#define PIN_ANALOG_REF      26
#define PIN_UART_TX         12
#define PIN_UART_RX         13
#define PIN_I2C_SDA         10
#define PIN_I2C_SCL         11

#define I2C_SLAVE_ADDR      0x3C
#define UART_BAUD           115200

/* --- PIO Square Wave Program for GPIO 20 (10 kHz, 50% duty) --- */
static const uint16_t square_wave_instructions[] = {
    0xe001, // 0: set pins, 1
    0xe000  // 1: set pins, 0
};

static const struct pio_program square_wave_program = {
    .instructions = square_wave_instructions,
    .length = 2,
    .origin = -1,
};

static void init_gpio20_clock(void) {
    PIO pio = pio0;
    uint sm = 0;
    uint offset = pio_add_program(pio, &square_wave_program);
    pio_sm_config c = pio_get_default_sm_config();
    sm_config_set_wrap(&c, offset, offset + 1);
    sm_config_set_set_pins(&c, PIN_PWM_10KHZ_CLK, 1);

    // 2 instructions per period -> 20,000 instructions/sec for 10 kHz
    float div = (float)clock_get_hz(clk_sys) / 20000.0f;
    sm_config_set_clkdiv(&c, div);

    pio_gpio_init(pio, PIN_PWM_10KHZ_CLK);
    pio_sm_set_consecutive_pindirs(pio, sm, PIN_PWM_10KHZ_CLK, 1, true);
    pio_sm_init(pio, sm, offset, &c);
    pio_sm_set_enabled(pio, sm, true);
}

/* --- PWM Test Output on GPIO 21 (1 kHz, 30% duty) --- */
static void init_gpio21_pwm(void) {
    uint32_t sys_clk = clock_get_hz(clk_sys);
    gpio_set_function(PIN_PWM_1KHZ_TEST, GPIO_FUNC_PWM);
    uint slice = pwm_gpio_to_slice_num(PIN_PWM_1KHZ_TEST);
    uint chan = pwm_gpio_to_channel(PIN_PWM_1KHZ_TEST);

    pwm_config cfg = pwm_get_default_config();
    // 1000 Hz with 10,000 steps wrap (0-9999)
    float div = (float)sys_clk / 10000000.0f;
    pwm_config_set_clkdiv(&cfg, div);
    pwm_config_set_wrap(&cfg, 9999);
    pwm_init(slice, &cfg, false);
    pwm_set_chan_level(slice, chan, 3000); // 30% duty
    pwm_set_enabled(slice, true);
}

/* --- Analog Reference Output on GPIO 26 (1 kHz PWM, 50% duty reference) --- */
static void init_gpio26_analog_ref(void) {
    uint32_t sys_clk = clock_get_hz(clk_sys);
    gpio_set_function(PIN_ANALOG_REF, GPIO_FUNC_PWM);
    uint slice = pwm_gpio_to_slice_num(PIN_ANALOG_REF);
    uint chan = pwm_gpio_to_channel(PIN_ANALOG_REF);

    pwm_config cfg = pwm_get_default_config();
    float div = (float)sys_clk / 10000000.0f;
    pwm_config_set_clkdiv(&cfg, div);
    pwm_config_set_wrap(&cfg, 9999);
    pwm_init(slice, &cfg, false);
    pwm_set_chan_level(slice, chan, 5000); // 50% duty -> 1.65V average, 0-3.3V swing
    pwm_set_enabled(slice, true);
}

/* --- I2C Slave Responder on GPIO 10 (SDA) / GPIO 11 (SCL) --- */
static uint8_t i2c_regs[256];
static uint8_t i2c_reg_addr = 0;
static bool i2c_reg_addr_set = false;

static void i2c_slave_cb(i2c_inst_t *i2c, i2c_slave_event_t event) {
    switch (event) {
    case I2C_SLAVE_RECEIVE:
        if (!i2c_reg_addr_set) {
            i2c_reg_addr = i2c_read_byte_raw(i2c);
            i2c_reg_addr_set = true;
        } else {
            i2c_regs[i2c_reg_addr++] = i2c_read_byte_raw(i2c);
        }
        break;
    case I2C_SLAVE_REQUEST:
        i2c_write_byte_raw(i2c, i2c_regs[i2c_reg_addr++]);
        break;
    case I2C_SLAVE_FINISH:
        i2c_reg_addr_set = false;
        break;
    }
}

static void init_i2c_slave_responder(void) {
    // Populate some default register values
    memset(i2c_regs, 0, sizeof(i2c_regs));
    i2c_regs[0x00] = 0x23; // RP2350 ID
    i2c_regs[0x01] = 0x50;
    i2c_regs[0x02] = 0x42; // Answer to life
    i2c_regs[0x10] = 0xAA;
    i2c_regs[0x11] = 0x55;

    gpio_init(PIN_I2C_SDA);
    gpio_set_function(PIN_I2C_SDA, GPIO_FUNC_I2C);
    gpio_pull_up(PIN_I2C_SDA);

    gpio_init(PIN_I2C_SCL);
    gpio_set_function(PIN_I2C_SCL, GPIO_FUNC_I2C);
    gpio_pull_up(PIN_I2C_SCL);

    i2c_init(i2c1, 100000);
    i2c_slave_init(i2c1, I2C_SLAVE_ADDR, i2c_slave_cb);
}

/* --- UART0 Transmitter and Full-Duplex Echo on GPIO 12 (TX) / GPIO 13 (RX) --- */
#define RX_FIFO_SIZE 512
static volatile uint8_t rx_fifo[RX_FIFO_SIZE];
static volatile uint16_t rx_head = 0;
static volatile uint16_t rx_tail = 0;
static volatile bool beacon_enabled = true;

static void on_uart_rx(void) {
    while (uart_is_readable(uart0)) {
        uint8_t ch = uart_getc(uart0);
        uint16_t next = (rx_head + 1) % RX_FIFO_SIZE;
        if (next != rx_tail) {
            rx_fifo[rx_head] = ch;
            rx_head = next;
        }
    }
}

static void init_uart(void) {
    uart_init(uart0, UART_BAUD);
    gpio_set_function(PIN_UART_TX, GPIO_FUNC_UART);
    gpio_set_function(PIN_UART_RX, GPIO_FUNC_UART);

    uart_set_hw_flow(uart0, false, false);
    uart_set_format(uart0, 8, 1, UART_PARITY_NONE);
    uart_set_fifo_enabled(uart0, true);

    int irq = uart0 == uart0 ? UART0_IRQ : UART1_IRQ;
    irq_set_exclusive_handler(irq, on_uart_rx);
    irq_set_enabled(irq, true);
    uart_set_irq_enables(uart0, true, false);
}

int main(void) {
    stdio_init_all();

    // LED Heartbeat
    gpio_init(PICO2_LED_PIN);
    gpio_set_dir(PICO2_LED_PIN, GPIO_OUT);
    gpio_put(PICO2_LED_PIN, 1);

    // Initialize all signal generators
    init_gpio20_clock();
    init_gpio21_pwm();
    init_gpio26_analog_ref();
    init_i2c_slave_responder();
    init_uart();

    uint32_t beacon_counter = 0;
    absolute_time_t next_beacon_time = make_timeout_time_ms(100);
    absolute_time_t next_led_time = make_timeout_time_ms(250);
    bool led_state = true;

    char cmd_buffer[64];
    uint8_t cmd_idx = 0;

    while (true) {
        // Process any received UART bytes for echo
        while (rx_tail != rx_head) {
            uint8_t b = rx_fifo[rx_tail];
            rx_tail = (rx_tail + 1) % RX_FIFO_SIZE;

            // Echo byte back immediately
            uart_putc_raw(uart0, b);

            // Command parser for test hooks
            if (b == '\n' || b == '\r') {
                if (cmd_idx > 0) {
                    cmd_buffer[cmd_idx] = '\0';
                    if (strcmp(cmd_buffer, "BEACON_OFF") == 0) {
                        beacon_enabled = false;
                        uart_puts(uart0, "OK BEACON_OFF\r\n");
                    } else if (strcmp(cmd_buffer, "BEACON_ON") == 0) {
                        beacon_enabled = true;
                        uart_puts(uart0, "OK BEACON_ON\r\n");
                    }
                    cmd_idx = 0;
                }
            } else if (cmd_idx < sizeof(cmd_buffer) - 1) {
                cmd_buffer[cmd_idx++] = (char)b;
            }
        }

        // Periodic UART test frame
        if (beacon_enabled && absolute_time_diff_us(get_absolute_time(), next_beacon_time) <= 0) {
            char beacon[64];
            snprintf(beacon, sizeof(beacon), "AGENTLA_TEST_BEACON %lu\r\n", (unsigned long)beacon_counter++);
            uart_puts(uart0, beacon);
            next_beacon_time = make_timeout_time_ms(100);
        }

        // LED toggle
        if (absolute_time_diff_us(get_absolute_time(), next_led_time) <= 0) {
            led_state = !led_state;
            gpio_put(PICO2_LED_PIN, led_state);
            next_led_time = make_timeout_time_ms(250);
        }

        tight_loop_contents();
    }
}

#include "agentla_adapter.h"
#include "agentla_board_settings.h"
#include <string.h>

static ADAPTER_STATE adapter_state;
static uart_inst_t *uart_adapter;
static uint8_t uart_rx_buffer[UART_RX_BUFFER_SIZE];
static volatile uint8_t uart_rx_head;
static volatile uint8_t uart_rx_tail;
static uint32_t current_baud = 0;

void adapter_init(void) {
    adapter_state = ADAPTER_IDLE;
    uart_rx_head = 0;
    uart_rx_tail = 0;
    adapter_i2c_init();
    adapter_uart_init();
}

void adapter_i2c_init(void) {
    i2c_init(i2c1, I2C_ADAPTER_FREQ_HZ);
    gpio_set_function(ADAPTER_I2C_SCL_PIN, GPIO_FUNC_I2C);
    gpio_set_function(ADAPTER_I2C_SDA_PIN, GPIO_FUNC_I2C);
    gpio_pull_up(ADAPTER_I2C_SCL_PIN);
    gpio_pull_up(ADAPTER_I2C_SDA_PIN);
}

void adapter_uart_init(void) {
    uart_adapter = uart1;
    uart_init(uart_adapter, 115200);
    gpio_set_function(ADAPTER_UART_TX_PIN, GPIO_FUNC_UART);
    gpio_set_function(ADAPTER_UART_RX_PIN, GPIO_FUNC_UART);
}

void adapter_poll(void) {
    if (uart_adapter) {
        while (uart_is_readable(uart_adapter)) {
            uint8_t byte = uart_getc(uart_adapter);
            uart_rx_buffer[uart_rx_head] = byte;
            uart_rx_head = (uart_rx_head + 1) % UART_RX_BUFFER_SIZE;
        }
    }
}

bool adapter_i2c_read(uint8_t addr, uint8_t reg, uint8_t len, uint8_t *data, uint32_t timeout_ms) {
    if (len == 0 || len > 64) return false;

    uint8_t buf[1] = {reg};
    int result = i2c_write_timeout_us(i2c1, addr, buf, 1, true, timeout_ms * 1000);
    if (result < 0) return false;

    result = i2c_read_timeout_us(i2c1, addr, data, len, false, timeout_ms * 1000);
    if (result < 0) return false;

    return true;
}

bool adapter_i2c_write(uint8_t addr, uint8_t reg, const uint8_t *data, uint8_t len, uint32_t timeout_ms) {
    if (len == 0 || len > 64) return false;

    uint8_t buf[65];
    if (len + 1 > sizeof(buf)) return false;

    buf[0] = reg;
    memcpy(&buf[1], data, len);

    int result = i2c_write_timeout_us(i2c1, addr, buf, len + 1, false, timeout_ms * 1000);
    if (result < 0) return false;

    return true;
}

bool adapter_i2c_clear_bus(void) {
    i2c_inst_t *i2c_inst = i2c1;
    uint sda_pin = ADAPTER_I2C_SDA_PIN;
    uint scl_pin = ADAPTER_I2C_SCL_PIN;

    gpio_set_function(sda_pin, GPIO_FUNC_SIO);
    gpio_set_function(scl_pin, GPIO_FUNC_SIO);

    gpio_set_dir(sda_pin, GPIO_IN);
    gpio_set_dir(scl_pin, GPIO_IN);

    gpio_pull_up(sda_pin);
    gpio_pull_up(scl_pin);

    sleep_ms(10);

    for (int i = 0; i < 9; i++) {
        gpio_put(scl_pin, 0);
        sleep_ms(1);
        gpio_put(scl_pin, 1);
        sleep_ms(1);
    }

    gpio_put(sda_pin, 0);
    sleep_ms(1);

    gpio_set_function(sda_pin, GPIO_FUNC_I2C);
    gpio_set_function(scl_pin, GPIO_FUNC_I2C);

    sleep_ms(10);

    return true;
}

bool adapter_uart_write(const uint8_t *data, uint8_t len, uint32_t baud) {
    if (baud == 0) return false;
    if (baud < 1200 || baud > 921600) return false;

    if (baud != current_baud) {
        uart_set_baudrate(uart_adapter, baud);
        current_baud = baud;
    }

    for (uint8_t i = 0; i < len; i++) {
        uart_putc(uart_adapter, data[i]);
    }

    return true;
}

int adapter_uart_read(uint8_t *data, uint8_t max_len, uint32_t baud, uint32_t timeout_ms) {
    if (baud == 0) return -1;
    if (baud < 1200 || baud > 921600) return -1;

    if (baud != current_baud) {
        uart_set_baudrate(uart_adapter, baud);
        current_baud = baud;
    }

    int count = 0;
    int32_t start_time = make_timeout_time_ms(timeout_ms);

    while (count < max_len) {
        if (uart_is_readable(uart_adapter)) {
            data[count++] = uart_getc(uart_adapter);
        } else {
            if (time_reached(start_time)) {
                break;
            }
            tight_loop_contents();
        }
    }

    return count;
}
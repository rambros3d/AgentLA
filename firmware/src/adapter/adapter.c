#include "adapter.h"
#include "board_settings.h"
#include "pin_owner.h"
#include "programmer/swd.h"
#include "tusb.h"
#include <string.h>

static ADAPTER_STATE adapter_state;
static uart_inst_t *uart_adapter;
static uint8_t uart_rx_buffer[UART_RX_BUFFER_SIZE];
static volatile uint8_t uart_rx_head;
static volatile uint8_t uart_rx_tail;
static uint32_t current_baud = 0;

static bool uart_initialized = false;

static uint8_t adapter_i2c_sda_pin = 0;
static uint8_t adapter_i2c_scl_pin = 0;
static uint32_t adapter_i2c_active_handle = 0;
static bool adapter_i2c_pinned = false;
static bool adapter_i2c_ready = false;

static uint8_t adapter_uart_tx_pin = 0;
static uint8_t adapter_uart_rx_pin = 0;
static uint32_t adapter_uart_active_handle = 0;
static bool adapter_uart_pinned = false;

void adapter_init(void) {
    adapter_state = ADAPTER_IDLE;
    uart_rx_head = 0;
    uart_rx_tail = 0;
    adapter_i2c_init();
    adapter_uart_init();
}

void adapter_init_i2c_only(void) {
    adapter_state = ADAPTER_IDLE;
    uart_rx_head = 0;
    uart_rx_tail = 0;
    adapter_i2c_init();
    /* Skip UART init - GPIO 4/5 are shared with SWD PIO1 */
    /* UART will be initialized lazily on first use via adapter_uart_init_lazy() */
}

void adapter_i2c_init(void) {
    adapter_i2c_standby();
}

void adapter_i2c_standby(void) {
    if (adapter_i2c_active_handle != 0) {
        pin_owner_result_t res = PIN_OWNER_OK;
        pin_owner_release_claim(adapter_i2c_active_handle, &res);
        adapter_i2c_active_handle = 0;
    }
    adapter_i2c_pinned = false;
    adapter_i2c_ready = false;

    /* Tri-state default I2C pins into High-Z standby */
    gpio_init(10);
    gpio_set_dir(10, GPIO_IN);
    gpio_disable_pulls(10);
    gpio_init(11);
    gpio_set_dir(11, GPIO_IN);
    gpio_disable_pulls(11);
}

bool adapter_i2c_is_pinned(void) {
    return adapter_i2c_pinned;
}

bool adapter_i2c_auto_claim(void) {
    if (adapter_i2c_pinned && adapter_i2c_ready) return true;
    if (adapter_i2c_pinned) {
        i2c_init(i2c1, I2C_ADAPTER_FREQ_HZ);
        gpio_set_function(adapter_i2c_scl_pin, GPIO_FUNC_I2C);
        gpio_set_function(adapter_i2c_sda_pin, GPIO_FUNC_I2C);
        gpio_pull_up(adapter_i2c_scl_pin);
        gpio_pull_up(adapter_i2c_sda_pin);
        adapter_i2c_ready = true;
        return true;
    }
    /* Auto-claim default flexible pins GP10 (SDA) and GP11 (SCL) */
    const uint8_t pins[2] = { 10, 11 };
    uint32_t handle = 0;
    if (!pin_owner_claim_pins(pins, 2, GPIO_FUNC_I2C, 0, &handle)) {
        return false;
    }
    return adapter_i2c_bind_claim(handle);
}

int adapter_i2c_raw_read(uint8_t addr, uint8_t *data, size_t len, bool nostop, uint32_t timeout_us) {
    if (!adapter_i2c_auto_claim()) return -1;
    if (!adapter_i2c_ready) {
        i2c_init(i2c1, I2C_ADAPTER_FREQ_HZ);
        gpio_set_function(adapter_i2c_scl_pin, GPIO_FUNC_I2C);
        gpio_set_function(adapter_i2c_sda_pin, GPIO_FUNC_I2C);
        gpio_pull_up(adapter_i2c_scl_pin);
        gpio_pull_up(adapter_i2c_sda_pin);
        adapter_i2c_ready = true;
    }
    if (len == 0) {
        uint8_t dummy = 0;
        return i2c_read_timeout_us(i2c1, addr, &dummy, 1, false, timeout_us);
    }
    return i2c_read_timeout_us(i2c1, addr, data, len, nostop, timeout_us);
}

int adapter_i2c_raw_write(uint8_t addr, const uint8_t *data, size_t len, bool nostop, uint32_t timeout_us) {
    if (!adapter_i2c_auto_claim()) return -1;
    if (!adapter_i2c_ready) {
        i2c_init(i2c1, I2C_ADAPTER_FREQ_HZ);
        gpio_set_function(adapter_i2c_scl_pin, GPIO_FUNC_I2C);
        gpio_set_function(adapter_i2c_sda_pin, GPIO_FUNC_I2C);
        gpio_pull_up(adapter_i2c_scl_pin);
        gpio_pull_up(adapter_i2c_sda_pin);
        adapter_i2c_ready = true;
    }
    if (len == 0) {
        uint8_t dummy = 0;
        return i2c_read_timeout_us(i2c1, addr, &dummy, 1, false, timeout_us);
    }
    return i2c_write_timeout_us(i2c1, addr, data, len, nostop, timeout_us);
}

bool adapter_i2c_bind_claim(uint32_t handle) {
    const pin_owner_claim_t *claim = pin_owner_get_claim(handle);
    if (claim == NULL || claim->observer_only != 0 || claim->pin_count != 2) {
        return false;
    }
    if (!pin_owner_is_flexible(claim->pins[0]) || !pin_owner_is_flexible(claim->pins[1])) {
        return false;
    }
    if (claim->function != GPIO_FUNC_I2C && claim->function != GPIO_FUNC_SIO) {
        return false;
    }

    adapter_i2c_sda_pin = claim->pins[0];
    adapter_i2c_scl_pin = claim->pins[1];
    adapter_i2c_active_handle = handle;
    adapter_i2c_pinned = true;
    adapter_i2c_ready = false;
    return true;
}

void adapter_uart_init(void) {
    adapter_uart_active_handle = 0;
    adapter_uart_pinned = false;
    adapter_uart_init_lazy();
}

static inline uart_inst_t* get_uart_for_pin(uint8_t pin) {
    if (pin == 12 || pin == 13 || pin == 14 || pin == 15 ||
        pin == 16 || pin == 17 || pin == 18 || pin == 19 ||
        pin == 28 || pin == 29 || pin == 0  || pin == 1  ||
        pin == 2  || pin == 3) {
        return uart0;
    }
    return uart1;
}

bool adapter_uart_bind_claim(uint32_t handle) {
    const pin_owner_claim_t *claim = pin_owner_get_claim(handle);
    if (claim == NULL || claim->observer_only != 0 || claim->pin_count != 2) {
        return false;
    }
    if (!pin_owner_is_flexible(claim->pins[0]) || !pin_owner_is_flexible(claim->pins[1])) {
        return false;
    }
    if (claim->function != GPIO_FUNC_UART && claim->function != GPIO_FUNC_SIO && claim->function != 1) {
        return false;
    }

    adapter_uart_tx_pin = claim->pins[0];
    adapter_uart_rx_pin = claim->pins[1];
    adapter_uart_active_handle = handle;
    adapter_uart_pinned = true;
    uart_initialized = false;
    return true;
}

void adapter_uart_init_lazy(void) {
    if (uart_initialized) return;
    /* UART pins are flexible claims, so uninitialized ownership is intentional. */
    if (!adapter_uart_pinned || adapter_uart_tx_pin == 0 || adapter_uart_rx_pin == 0) {
        return;
    }
    /* Check if SWD is active - if so, avoid changing shared PIO pins. */
    if (swd_is_initialized()) {
        return;
    }
    adapter_uart_bind_claim(adapter_uart_active_handle);
    uart_adapter = get_uart_for_pin(adapter_uart_tx_pin);
    uart_init(uart_adapter, 115200);
    gpio_set_function(adapter_uart_tx_pin, UART_FUNCSEL_NUM(uart_adapter, adapter_uart_tx_pin));
    gpio_set_function(adapter_uart_rx_pin, UART_FUNCSEL_NUM(uart_adapter, adapter_uart_rx_pin));
    uart_initialized = true;
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
    if (!adapter_i2c_pinned || adapter_i2c_sda_pin == 0 || adapter_i2c_scl_pin == 0) return false;
    if (len == 0 || len > 64) return false;

    if (!adapter_i2c_ready) {
        i2c_init(i2c1, I2C_ADAPTER_FREQ_HZ);
        gpio_set_function(adapter_i2c_scl_pin, GPIO_FUNC_I2C);
        gpio_set_function(adapter_i2c_sda_pin, GPIO_FUNC_I2C);
        gpio_pull_up(adapter_i2c_scl_pin);
        gpio_pull_up(adapter_i2c_sda_pin);
        adapter_i2c_ready = true;
    }

    uint8_t buf[1] = {reg};
    int result = i2c_write_timeout_us(i2c1, addr, buf, 1, true, timeout_ms * 1000);
    if (result < 0) return false;

    result = i2c_read_timeout_us(i2c1, addr, data, len, false, timeout_ms * 1000);
    if (result < 0) return false;

    return true;
}

bool adapter_i2c_write(uint8_t addr, uint8_t reg, const uint8_t *data, uint8_t len, uint32_t timeout_ms) {
    if (!adapter_i2c_pinned || adapter_i2c_sda_pin == 0 || adapter_i2c_scl_pin == 0) return false;
    if (len == 0 || len > 64) return false;

    if (!adapter_i2c_ready) {
        i2c_init(i2c1, I2C_ADAPTER_FREQ_HZ);
        gpio_set_function(adapter_i2c_scl_pin, GPIO_FUNC_I2C);
        gpio_set_function(adapter_i2c_sda_pin, GPIO_FUNC_I2C);
        gpio_pull_up(adapter_i2c_scl_pin);
        gpio_pull_up(adapter_i2c_sda_pin);
        adapter_i2c_ready = true;
    }

    uint8_t buf[65];
    if (len + 1 > sizeof(buf)) return false;

    buf[0] = reg;
    memcpy(&buf[1], data, len);

    int result = i2c_write_timeout_us(i2c1, addr, buf, len + 1, false, timeout_ms * 1000);
    if (result < 0) return false;

    return true;
}

bool adapter_i2c_clear_bus(void) {
    if (!adapter_i2c_pinned || adapter_i2c_sda_pin == 0 || adapter_i2c_scl_pin == 0) return false;
    i2c_inst_t *i2c_inst = i2c1;
    uint sda_pin = adapter_i2c_sda_pin;
    uint scl_pin = adapter_i2c_scl_pin;

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
    if (baud == 0 || !adapter_uart_pinned || !uart_initialized || uart_adapter == NULL) return false;
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
    absolute_time_t timeout_time = make_timeout_time_ms(timeout_ms);

    while (count < max_len) {
        if (uart_is_readable(uart_adapter)) {
            data[count++] = uart_getc(uart_adapter);
        } else {
            if (time_reached(timeout_time)) {
                break;
            }
            tud_task();
            tight_loop_contents();
        }
    }

    return count;
}
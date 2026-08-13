#include "programmer.h"
#include "board_settings.h"
#include "swd.h"
#include "pico/stdlib.h"
#include "hardware/uart.h"
#include "hardware/gpio.h"
#include "hardware/pio.h"

volatile bool programmer_mode_active = false;
volatile bool raw_passthrough = false;
static uart_inst_t *prog_uart = uart1;



static void prog_uart_drain_rx(void) {
    while (uart_is_readable(prog_uart)) {
        (void)uart_getc(prog_uart);
    }
}

static void prog_uart_write_byte(uint8_t byte) {
    uart_putc(prog_uart, byte);
}

static uint8_t prog_uart_read_byte(uint32_t timeout_ms) {
    absolute_time_t deadline = make_timeout_time_ms(timeout_ms);
    while (!time_reached(deadline)) {
        if (uart_is_readable(prog_uart)) {
            return uart_getc(prog_uart);
        }
    }
    return 0xFF;
}

static bool prog_uart_write(const uint8_t *data, uint32_t len) {
    for (uint32_t i = 0; i < len; i++) {
        prog_uart_write_byte(data[i]);
    }
    return true;
}

static bool prog_uart_read(uint8_t *data, uint32_t len, uint32_t timeout_ms) {
    absolute_time_t deadline = make_timeout_time_ms(timeout_ms);
    for (uint32_t i = 0; i < len; i++) {
        while (!uart_is_readable(prog_uart)) {
            if (time_reached(deadline)) return false;
        }
        data[i] = uart_getc(prog_uart);
    }
    return true;
}

static void prog_send_ack(void) {
    prog_uart_write_byte(0x79);
}

static bool prog_wait_ack(uint32_t timeout_ms) {
    uint8_t resp = prog_uart_read_byte(timeout_ms);
    return (resp == 0x79);
}

void prog_init(void) {
    programmer_mode_active = false;
}

bool prog_enter_mode(void) {
    if (programmer_mode_active) return true;

    gpio_init(PROG_I2C_SDA_PIN);
    gpio_set_dir(PROG_I2C_SDA_PIN, GPIO_IN);
    gpio_init(PROG_I2C_SCL_PIN);
    gpio_set_dir(PROG_I2C_SCL_PIN, GPIO_IN);
    gpio_init(PROG_SWDCLK_PIN);
    gpio_set_dir(PROG_SWDCLK_PIN, GPIO_IN);
    gpio_init(PROG_SWDIO_PIN);
    gpio_set_dir(PROG_SWDIO_PIN, GPIO_IN);
    gpio_init(PROG_NRESET_PIN);
    gpio_set_dir(PROG_NRESET_PIN, GPIO_IN);
    gpio_init(PROG_UART_TX_PIN);
    gpio_set_dir(PROG_UART_TX_PIN, GPIO_IN);
    gpio_init(PROG_UART_RX_PIN);
    gpio_set_dir(PROG_UART_RX_PIN, GPIO_IN);

    uart_init(prog_uart, PROG_BAUD_RATE);
    gpio_set_function(PROG_UART_TX_PIN, GPIO_FUNC_UART);
    gpio_set_function(PROG_UART_RX_PIN, GPIO_FUNC_UART);

    programmer_mode_active = true;
    return true;
}

bool prog_exit_mode(void) {
    if (!programmer_mode_active) return true;

    uart_deinit(prog_uart);

    gpio_init(PROG_I2C_SDA_PIN);
    gpio_set_dir(PROG_I2C_SDA_PIN, GPIO_IN);
    gpio_init(PROG_I2C_SCL_PIN);
    gpio_set_dir(PROG_I2C_SCL_PIN, GPIO_IN);
    gpio_init(PROG_SWDCLK_PIN);
    gpio_set_dir(PROG_SWDCLK_PIN, GPIO_IN);
    gpio_init(PROG_SWDIO_PIN);
    gpio_set_dir(PROG_SWDIO_PIN, GPIO_IN);
    gpio_init(PROG_NRESET_PIN);
    gpio_set_dir(PROG_NRESET_PIN, GPIO_IN);
    gpio_init(PROG_UART_TX_PIN);
    gpio_set_dir(PROG_UART_TX_PIN, GPIO_IN);
    gpio_init(PROG_UART_RX_PIN);
    gpio_set_dir(PROG_UART_RX_PIN, GPIO_IN);

    programmer_mode_active = false;
    return true;
}

bool prog_is_active(void) {
    return programmer_mode_active;
}

bool prog_power_on(void) {
    /* GPIO7 power control removed — target must be powered externally.
     * RP2350 GPIO7 max ~12mA insufficient for most targets.
     */
    sleep_ms(PROG_POWER_DELAY_MS);
    return true;
}

bool prog_power_off(void) {
    /* GPIO7 power control removed — target must be powered externally */
    return true;
}

bool prog_power_status(void) {
    /* GPIO7 power control removed — always report powered */
    return true;
}

bool prog_nreset_low(void) {
    gpio_init(PROG_NRESET_PIN);
    gpio_set_dir(PROG_NRESET_PIN, GPIO_OUT);
    gpio_put(PROG_NRESET_PIN, 0);
    return true;
}

bool prog_nreset_high(void) {
    gpio_init(PROG_NRESET_PIN);
    gpio_set_dir(PROG_NRESET_PIN, GPIO_IN);
    return true;
}

bool prog_nreset_pulse(void) {
    prog_nreset_low();
    sleep_ms(PROG_NRESET_PULSE_MS);
    prog_nreset_high();
    return true;
}

bool prog_uart_test(void) {
    uint8_t tx_data = 0x55;
    uint8_t rx_data = 0;

    prog_uart_drain_rx();
    prog_uart_write_byte(tx_data);
    sleep_ms(10);

    if (uart_is_readable(prog_uart)) {
        rx_data = uart_getc(prog_uart);
        return (rx_data == tx_data);
    }
    return false;
}

bool prog_stm32_sync(void) {
    prog_uart_drain_rx();
    prog_uart_write_byte(0x7F);
    return prog_wait_ack(1000);
}

bool prog_stm32_write(uint32_t addr, const uint8_t *data, uint32_t len) {
    uint8_t buf[256];
    uint32_t i;
    uint8_t xor_chk;

    if (len == 0 || len > 256) return false;

    prog_uart_drain_rx();
    prog_uart_write_byte(0x31);
    prog_uart_write_byte(0xCE);
    if (!prog_wait_ack(1000)) return false;

    buf[0] = addr & 0xFF;
    buf[1] = (addr >> 8) & 0xFF;
    buf[2] = (addr >> 16) & 0xFF;
    buf[3] = (addr >> 24) & 0xFF;
    xor_chk = buf[0] ^ buf[1] ^ buf[2] ^ buf[3];
    buf[4] = xor_chk;
    prog_uart_write(buf, 5);
    if (!prog_wait_ack(1000)) return false;

    buf[0] = len & 0xFF;
    buf[1] = (len >> 8) & 0xFF;
    xor_chk = buf[0] ^ buf[1];
    buf[2] = xor_chk;
    prog_uart_write(buf, 3);
    if (!prog_wait_ack(1000)) return false;

    xor_chk = 0;
    for (i = 0; i < len; i++) {
        buf[i] = data[i];
        xor_chk ^= data[i];
    }
    buf[len] = xor_chk;
    prog_uart_write(buf, len + 1);
    if (!prog_wait_ack(1000)) return false;

    return true;
}

bool prog_stm32_verify(uint32_t addr, uint32_t len) {
    (void)addr;
    (void)len;
    return false;
}

bool prog_stm32_go(uint32_t addr) {
    prog_uart_drain_rx();
    prog_uart_write_byte(0x21);
    prog_uart_write_byte(0xDE);
    if (!prog_wait_ack(1000)) return false;

    uint8_t buf[6];
    buf[0] = addr & 0xFF;
    buf[1] = (addr >> 8) & 0xFF;
    buf[2] = (addr >> 16) & 0xFF;
    buf[3] = (addr >> 24) & 0xFF;
    buf[4] = buf[0] ^ buf[1] ^ buf[2] ^ buf[3];
    buf[5] = 0x00;  /* AN3155 requires 0x00 terminator */
    prog_uart_write(buf, 6);
    return prog_wait_ack(2000);
}

bool prog_rp2350_sync(void) {
    prog_uart_drain_rx();
    prog_uart_write_byte(0x7E);
    return prog_wait_ack(1000);
}

bool prog_rp2350_load(uint32_t addr, const uint8_t *data, uint32_t len) {
    (void)addr;
    (void)data;
    (void)len;
    return false;
}

bool prog_nrf_sync(void) {
    return false;
}

bool prog_ch32_sync(void) {
    prog_uart_drain_rx();
    prog_uart_write_byte(0x7F);
    prog_uart_write_byte(0x7F);
    return prog_wait_ack(1000);
}

void prog_raw_passthrough_on(void) {
    raw_passthrough = true;
}

void prog_raw_passthrough_off(void) {
    raw_passthrough = false;
}

bool prog_raw_passthrough_getc(uint8_t *byte, uint32_t timeout_ms) {
    if (!raw_passthrough) return false;
    if (!uart_is_readable(prog_uart)) return false;
    *byte = uart_getc(prog_uart);
    return true;
}

void prog_raw_passthrough_putc(uint8_t byte) {
    if (!raw_passthrough) return;
    prog_uart_write_byte(byte);
}

void prog_raw_passthrough_flush_rx(void) {
    if (!raw_passthrough) return;
    prog_uart_drain_rx();
}

void prog_passthrough_forward(void) {
    if (!raw_passthrough) return;
    while (uart_is_readable(prog_uart)) {
        uint8_t b = uart_getc(prog_uart);
        uart_putc(uart_default, b);
    }
}

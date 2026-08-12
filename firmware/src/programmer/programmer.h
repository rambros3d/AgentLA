#ifndef __AGENTLA_PROGRAMMER__
#define __AGENTLA_PROGRAMMER__

#include "pico/stdlib.h"
#include "hardware/uart.h"

/* Transport driver selection — bitbang is the proven default; the PIO engines
 * remain selectable for the SWD CAPTURE / PIO_IDCODE diagnostics. */
typedef enum {
    PROG_DRIVER_BITBANG = 0,
    PROG_DRIVER_PIO
} prog_driver_t;

extern prog_driver_t prog_driver;

/* Take GPIO4/5/6 out of the PIO engines and into raw SIO bitbang ownership.
 * Safe to call repeatedly (idempotent). */
void enter_bitbang_mode(void);

/* Return pins to the PIO engine(s): re-run swd/swio/rvswd init as needed. */
void exit_bitbang_mode(void);

void prog_init(void);
bool prog_enter_mode(void);
bool prog_exit_mode(void);
bool prog_is_active(void);

bool prog_power_on(void);
bool prog_power_off(void);
bool prog_power_status(void);

bool prog_nreset_low(void);
bool prog_nreset_high(void);
bool prog_nreset_pulse(void);

bool prog_uart_test(void);

bool prog_stm32_sync(void);
bool prog_stm32_write(uint32_t addr, const uint8_t *data, uint32_t len);
bool prog_stm32_verify(uint32_t addr, uint32_t len);
bool prog_stm32_go(uint32_t addr);

bool prog_rp2350_sync(void);
bool prog_rp2350_load(uint32_t addr, const uint8_t *data, uint32_t len);

bool prog_nrf_sync(void);

bool prog_ch32_sync(void);

void prog_raw_passthrough_on(void);
void prog_raw_passthrough_off(void);
bool prog_raw_passthrough_getc(uint8_t *byte, uint32_t timeout_ms);
void prog_raw_passthrough_putc(uint8_t byte);
void prog_raw_passthrough_flush_rx(void);
void prog_passthrough_forward(void);

extern volatile bool raw_passthrough;

#endif
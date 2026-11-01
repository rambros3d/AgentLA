#ifndef __AGENTLA_PROGRAMMER__
#define __AGENTLA_PROGRAMMER__

#include "pico/stdlib.h"
#include "hardware/uart.h"

/* Transport driver selection — PIO (debugprobe hardware engine). */
typedef enum {
    PROG_DRIVER_PIO = 0
} prog_driver_t;



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
bool prog_ch592_unlock(char *err_msg, size_t err_size);

void prog_raw_passthrough_on(void);
void prog_raw_passthrough_off(void);
bool prog_raw_passthrough_getc(uint8_t *byte, uint32_t timeout_ms);
void prog_raw_passthrough_putc(uint8_t byte);
void prog_raw_passthrough_flush_rx(void);
void prog_passthrough_forward(void);

extern volatile bool raw_passthrough;

#endif
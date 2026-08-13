/*
 * rvswd.c - WCH RVSWD (2-wire RISC-V Debug) Top-Level Interface
 *
 * Delegates to the encapsulated PIO transport driver in rvswd_pio_transport.c.
 * License: MIT
 */

#include "rvswd.h"
#include "rvswd/rvswd_pio_transport.h"

rvswd_result_t rvswd_init(void) {
    return rvswd_transport_init();
}

void rvswd_deinit(void) {
    rvswd_transport_deinit();
}

bool rvswd_is_initialized(void) {
    return rvswd_transport_is_initialized();
}

rvswd_result_t rvswd_reset(void) {
    return rvswd_transport_reset();
}

rvswd_result_t rvswd_write(uint8_t reg, uint32_t value) {
    return rvswd_transport_write(reg, value);
}

rvswd_result_t rvswd_read(uint8_t reg, uint32_t *value) {
    return rvswd_transport_read(reg, value);
}

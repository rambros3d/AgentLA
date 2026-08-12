/*
 * tap_pio.h - PIO/DMA Integration Layer for SWD Engine
 */

#ifndef AGENTLA_TAP_PIO_H
#define AGENTLA_TAP_PIO_H

#include <stdint.h>
#include <stdbool.h>
#include "pico/stdlib.h"
#include "hardware/pio.h"
#include "hardware/dma.h"

#define AGENTLA_TAP_PIO          (pio0)
#define AGENTLA_TAP_SM           (0)
#define AGENTLA_TAP_DMA_BUF_SIZE (32)

/* Timeout for PIO blocking reads (microseconds) */
#define PIO_READ_TIMEOUT_US      10000  /* 10ms */

/* DMA transfer timeout (milliseconds) — if DMA doesn't finish in this time,
 * capture PIO SM state for diagnostics and return instead of blocking. */
#define DMA_TIMEOUT_MS           100

/* TXSTALL wait timeout (milliseconds) — bounded busy-wait so a stalled PIO SM
 * (e.g. target not connected) can never freeze the single-threaded firmware. */
#define TX_STALL_TIMEOUT_MS      100

/* Diagnostic state captured on DMA timeout */
typedef struct {
    uint32_t pc;            /* PIO SM program counter */
    uint32_t tx_fifo_level; /* TX FIFO entries (0-4) */
    uint32_t rx_fifo_level; /* RX FIFO entries (0-4) */
    uint32_t txstall;       /* TXSTALL debug bit */
    uint32_t exec_stall;    /* EXECCTRL stall bit */
    const uint32_t *buf_addr; /* Last buffer address being sent */
    uint32_t buf_len;       /* Last buffer length */
    bool     timed_out;     /* True if a DMA timeout occurred */
} swd_diag_state_t;

extern swd_diag_state_t swd_diag_state;

static inline bool tap_pio_is_not_tx_stalled(PIO pio, uint32_t sm)
{
    pio->fdebug = (1UL << (PIO_FDEBUG_TXSTALL_LSB + sm));
    return ((pio->fdebug & (1UL << (PIO_FDEBUG_TXSTALL_LSB + sm))) == 0);
}

static inline void tap_pio_wait_for_tx_stall(PIO pio, uint32_t sm)
{
    absolute_time_t deadline = make_timeout_time_ms(10);
    while (!pio_sm_is_tx_fifo_empty(pio, sm)) {
        if (time_reached(deadline)) {
            swd_diag_state.timed_out = true;
            return;
        }
    }
}

/**
 * Read from PIO RX FIFO with timeout.
 * @return true if data received, false on timeout
 */
static inline bool tap_pio_get_blocking_timeout(PIO pio, uint32_t sm, uint32_t *value)
{
    uint32_t timeout = PIO_READ_TIMEOUT_US;
    while (pio_sm_is_rx_fifo_empty(pio, sm)) {
        if (--timeout == 0) return false;
        busy_wait_us_32(1);
    }
    *value = pio_sm_get(pio, sm);
    return true;
}

void tap_pio_init(PIO pio, uint32_t sm);
void tap_pio_reset_dma(void);
bool tap_pio_dma_send_uint32(PIO pio, uint32_t sm, const uint32_t *buffer_send, uint32_t data_amount);
uint32_t tap_pio_dma_send_recv_uint32(PIO pio, uint32_t sm, const uint32_t *buffer_send, uint32_t *buffer_recv,
                                     uint32_t data_amount, uint32_t data_amount_to_read);
uint32_t tap_pio_set_sm_freq(PIO pio, uint32_t sm, uint32_t freq, uint32_t max_interface_freq);

#endif /* AGENTLA_TAP_PIO_H */

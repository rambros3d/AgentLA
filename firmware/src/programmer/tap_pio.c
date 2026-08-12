/*
 * tap_pio.c - PIO/DMA Integration Layer for SWD Engine
 */

#include "tap_pio.h"
#include "hardware/dma.h"
#include "hardware/pio.h"
#include "hardware/clocks.h"
#include "pico/stdlib.h"
#include "swd.pio.h"
#include <assert.h>
#include <stddef.h>

static int pio_dma_channel = -1;

/* Diagnostic state captured on DMA timeout */
swd_diag_state_t swd_diag_state = { .timed_out = false };

void tap_pio_reset_dma(void)
{
    /* Abort any pending DMA transfer and release the channel.
     * Called before reinitializing PIO (e.g., after LOOPBACK test). */
    if (pio_dma_channel >= 0) {
        dma_channel_abort(pio_dma_channel);
        dma_channel_unclaim(pio_dma_channel);
        pio_dma_channel = -1;
    }
}

void tap_pio_init(PIO pio, uint32_t sm)
{
    /* Reset DMA channel to ensure clean state after PIO reinit */
    tap_pio_reset_dma();
    pio_dma_channel = dma_claim_unused_channel(true);

    /* Initialize SWD GPIO pins and state machine */
    swd_gpio_init();

    pio_sm_config c = swd_start_program_get_default_config(0);
    swd_sm_init(&c);

    /* Load PIO program into PIO memory */
    pio_clear_instruction_memory(pio);

    /* Add all programs at their fixed origins.
     * NOTE: pio_add_program_at_offset() rejects (PICO_ERROR_BAD_ALIGNMENT) any
     * program whose declared .origin differs from the load offset here, so these
     * MUST stay in sync with the .pio file. check_ack is loaded at 20 (declared
     * origin 20); turnaround_float_to_drive at 27, directly after check_ack's
     * 7 instructions (20-26), so the clear_tx_fifo loop can fall through into it. */
    int rc;
    rc = pio_add_program_at_offset(pio, &swd_start_program, 0);
    assert(rc == 0);
    rc = pio_add_program_at_offset(pio, &swd_seq_out_turnaround_program, 2);
    assert(rc == 0);
    rc = pio_add_program_at_offset(pio, &swd_seq_out_program, 4);
    assert(rc == 0);
    rc = pio_add_program_at_offset(pio, &swd_seq_in_turnaround_program, 10);
    assert(rc == 0);
    rc = pio_add_program_at_offset(pio, &swd_seq_in_program, 12);
    assert(rc == 0);
    rc = pio_add_program_at_offset(pio, &swd_adiv5_check_ack_program, 20);
    assert(rc == 0);
    rc = pio_add_program_at_offset(pio, &swd_turnaround_float_to_drive_program, 27);
    assert(rc == 0);

    pio_sm_init(pio, sm, 0, &c);
    pio_sm_set_enabled(pio, sm, true);
}

bool tap_pio_dma_send_uint32(PIO pio, uint32_t sm, const uint32_t *buffer_send, uint32_t data_amount)
{
    if (buffer_send == NULL || data_amount == 0) {
        return true;
    }

    if (pio_dma_channel == -1) {
        pio_dma_channel = dma_claim_unused_channel(true);
    }

    dma_channel_config tx_config = dma_channel_get_default_config(pio_dma_channel);
    channel_config_set_transfer_data_size(&tx_config, DMA_SIZE_32);
    channel_config_set_read_increment(&tx_config, true);
    channel_config_set_write_increment(&tx_config, false);
    channel_config_set_dreq(&tx_config, pio_get_dreq(pio, sm, true));

    dma_channel_configure(pio_dma_channel, &tx_config, &(pio->txf[sm]), buffer_send, data_amount, true);

    /* Wait for DMA with timeout instead of blocking forever */
    absolute_time_t deadline = make_timeout_time_ms(DMA_TIMEOUT_MS);
    while (dma_channel_is_busy(pio_dma_channel)) {
        if (time_reached(deadline)) {
            /* Capture PIO SM state for diagnostics */
            swd_diag_state.pc = pio_sm_get_pc(pio, sm);
            swd_diag_state.tx_fifo_level = (pio->fstat >> (PIO_FSTAT_TXFULL_LSB + sm)) & 1
                                          ? 4 : ((pio->fstat >> PIO_FSTAT_TXEMPTY_LSB + sm) & 1) ? 0 : 1;
            swd_diag_state.rx_fifo_level = (pio->fstat >> (PIO_FSTAT_RXEMPTY_LSB + sm)) & 1 ? 0 : 1;
            swd_diag_state.txstall = (pio->fdebug >> (PIO_FDEBUG_TXSTALL_LSB + sm)) & 1;
            swd_diag_state.exec_stall = (pio->sm[sm].execctrl >> PIO_SM0_EXECCTRL_EXEC_STALLED_LSB) & 1;
            swd_diag_state.buf_addr = buffer_send;
            swd_diag_state.buf_len = data_amount;
            swd_diag_state.timed_out = true;
            /* Abort the DMA transfer */
            dma_channel_abort(pio_dma_channel);
            return false;
        }
    }
    swd_diag_state.timed_out = false;
    return true;
}

uint32_t tap_pio_dma_send_recv_uint32(PIO pio, uint32_t sm, const uint32_t *buffer_send, uint32_t *buffer_recv,
                                     uint32_t data_amount, uint32_t data_amount_to_read)
{
    if (buffer_send == NULL || data_amount == 0) {
        return 0;
    }

    if (pio_dma_channel == -1) {
        pio_dma_channel = dma_claim_unused_channel(true);
    }

    dma_channel_config tx_config = dma_channel_get_default_config(pio_dma_channel);
    channel_config_set_transfer_data_size(&tx_config, DMA_SIZE_32);
    channel_config_set_read_increment(&tx_config, true);
    channel_config_set_write_increment(&tx_config, false);
    channel_config_set_dreq(&tx_config, pio_get_dreq(pio, sm, true));

    dma_channel_configure(pio_dma_channel, &tx_config, &(pio->txf[sm]), buffer_send, data_amount, true);

    uint32_t recv_data_amount = 0;
    absolute_time_t deadline = make_timeout_time_ms(DMA_TIMEOUT_MS);
    while (dma_channel_is_busy(pio_dma_channel) || recv_data_amount < data_amount_to_read) {
        if (time_reached(deadline)) {
            /* Capture PIO SM state for diagnostics */
            swd_diag_state.pc = pio_sm_get_pc(pio, sm);
            swd_diag_state.tx_fifo_level = (pio->fstat >> (PIO_FSTAT_TXFULL_LSB + sm)) & 1
                                          ? 4 : ((pio->fstat >> PIO_FSTAT_TXEMPTY_LSB + sm) & 1) ? 0 : 1;
            swd_diag_state.rx_fifo_level = (pio->fstat >> (PIO_FSTAT_RXEMPTY_LSB + sm)) & 1 ? 0 : 1;
            swd_diag_state.txstall = (pio->fdebug >> (PIO_FDEBUG_TXSTALL_LSB + sm)) & 1;
            swd_diag_state.exec_stall = (pio->sm[sm].execctrl >> PIO_SM0_EXECCTRL_EXEC_STALLED_LSB) & 1;
            swd_diag_state.buf_addr = buffer_send;
            swd_diag_state.buf_len = data_amount;
            swd_diag_state.timed_out = true;
            dma_channel_abort(pio_dma_channel);
            return recv_data_amount;
        }
        if (!pio_sm_is_rx_fifo_empty(pio, sm)) {
            uint32_t read_value = pio_sm_get(pio, sm);
            if (buffer_recv != NULL && recv_data_amount < data_amount_to_read) {
                buffer_recv[recv_data_amount] = read_value;
            }
            recv_data_amount++;
        }
    }
    swd_diag_state.timed_out = false;

    __compiler_memory_barrier();
    return recv_data_amount;
}

uint32_t tap_pio_set_sm_freq(PIO pio, uint32_t sm, uint32_t freq, uint32_t max_interface_freq)
{
    const uint32_t min_freq = (max_interface_freq >> 16);
    const uint32_t max_freq = max_interface_freq;

    if (freq < min_freq) {
        freq = min_freq;
    } else if (freq > max_freq) {
        freq = max_freq;
    }

    uint32_t clkdiv_int = (max_interface_freq / freq);
    uint32_t clkdiv_fraq = ((((uint64_t)max_interface_freq) << 8) / freq) & 0xFF;

    if (clkdiv_int >= (((uint32_t)UINT16_MAX) + 1)) {
        clkdiv_int = 0;
        clkdiv_fraq = 0;
    }

    pio_sm_set_clkdiv_int_frac(pio, sm, clkdiv_int, clkdiv_fraq);
    pio_sm_clkdiv_restart(pio, sm);

    return freq;
}

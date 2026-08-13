/*
 * The MIT License (MIT)
 *
 * Copyright (c) 2021 Raspberry Pi (Trading) Ltd.
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in
 * all copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
 * THE SOFTWARE.
 *
 */

/* ============================================================================
 * Vendored from raspberrypi/debugprobe @ commit 3fff5b2 (src/probe.c).
 * PIO transport primitives for SWD on PIO0 SM0 (SWCLK GPIO4, SWDIO GPIO5).
 * Direction is encoded in-band in each FIFO command word (no host-side
 * direction tracking); the clock never stops; turnaround is just read/hi-Z
 * clock cycles. See swd_probe.pio for the program.
 * ========================================================================== */

#ifndef AGENTLA_PROBE_PIO_TRANSPORT_H
#define AGENTLA_PROBE_PIO_TRANSPORT_H

#include <stdint.h>

/* Init/deinit the PIO transport (loads swd_probe.pio on PIO0 SM0). */
void probe_init(void);
void probe_deinit(void);

/* Set SWCLK frequency in kHz (integer SM divider derived from clk_sys). */
void probe_set_swclk_freq(uint32_t freq_khz);

/* Drive n bits (LSB-first) on SWDIO with continuous SWCLK. Returns
 * immediately after queueing (commands are pipelined in the FIFO). */
void probe_write_bits(uint32_t bit_count, uint32_t data_byte);

/* Clock n cycles with SWDIO released (hi-Z) — turnaround/idle. */
void probe_hiz_clocks(uint32_t bit_count);

/* Read n bits (LSB-first), blocking until the SM pushes the result.
 * Bounded by an internal timeout (returns 0 on a dead SM). */
uint32_t probe_read_bits(uint32_t bit_count);

/* Force the SM into a known write/read idle direction (for clean deinit). */
void probe_read_mode(void);
void probe_write_mode(void);

#endif /* AGENTLA_PROBE_PIO_TRANSPORT_H */

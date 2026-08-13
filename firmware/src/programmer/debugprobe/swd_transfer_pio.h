/*
 * Copyright (c) 2013-2022 ARM Limited. All rights reserved.
 * Copyright (c) 2022 Raspberry Pi Ltd
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * Licensed under the Apache License, Version 2.0 (the License); you may
 * not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 * www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an AS IS BASIS, WITHOUT
 * WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

/* ============================================================================
 * Ported from raspberrypi/debugprobe @ commit 3fff5b2 (src/sw_dp_pio.c).
 * The proven CMSIS-DAP SWD_Transfer transaction engine, made dependency-free:
 *   - DAP_Data.swd_conf.turnaround  -> swd_pio_conf_t.turnaround
 *   - DAP_Data.transfer.idle_cycles -> swd_pio_conf_t.idle_cycles
 *   - DAP_Data.swd_conf.data_phase  -> swd_pio_conf_t.data_phase
 *   - clock delay handling / timestamp capture removed (transport owns clock)
 * SWJ_Sequence / SWD_Sequence are NOT ported (AgentLA line-reset and
 * JTAG-to-SWD switch live in swd.c and are wire-proven).
 * ========================================================================== */

#ifndef AGENTLA_SWD_TRANSFER_PIO_H
#define AGENTLA_SWD_TRANSFER_PIO_H

#include <stdint.h>
#include <stdbool.h>

/* Configuration for the SWD transaction engine. */
typedef struct {
    uint8_t turnaround;   /* SWD turnaround cycles (1 or 2) */
    uint8_t idle_cycles;  /* idle cycles driven after a successful transfer */
    bool    data_phase;   /* consume a dummy data phase on WAIT/FAULT */
} swd_pio_conf_t;

/**
 * Perform one SWD transfer (request + optional data phase).
 *
 * @param request ADIv5 request bits: bit0=APnDP, bit1=RnW, bit2=A2, bit3=A3.
 * @param data    In/out 32-bit data word (write data / read result).
 * @return ACK: 1=OK, 2=WAIT, 4=FAULT; 5 (DAP_TRANSFER_ERROR) on a data-parity
 *         error; or the raw garbage ACK (0/7) on a protocol error after
 *         backing off the data phase.
 */
uint8_t swd_pio_transfer(const swd_pio_conf_t *conf, uint32_t request, uint32_t *data);

#endif /* AGENTLA_SWD_TRANSFER_PIO_H */

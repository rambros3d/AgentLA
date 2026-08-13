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
 * This is a shim between the SW_DP functions and the PIO implementation
 * vendored from Debugprobe. Instead of calling bitbash functions, hand off
 * the bit sequences to a SM for asynchronous completion.
 * ========================================================================== */

#include "swd_transfer_pio.h"
#include "probe_pio_transport.h"
#include <stdio.h>

/* SWD ACK values (DAP_TRANSFER_OK / WAIT / FAULT / ERROR). */
#define SWD_ACK_OK_VAL    0x01U
#define SWD_ACK_WAIT_VAL  0x02U
#define SWD_ACK_FAULT_VAL 0x04U
#define SWD_ACK_ERR_VAL   0x05U  /* parity error */
#define SWD_REQ_RnW       (1U << 1)

// SWD Transfer I/O
//   request: A[3:2] RnW APnDP
//   data:    DATA[31:0]
//   return:  ACK[2:0]
uint8_t swd_pio_transfer(const swd_pio_conf_t *conf, uint32_t request, uint32_t *data) {
  uint8_t prq = 0;
  uint8_t ack;
  uint8_t bit;
  uint32_t val = 0;
  uint32_t parity = 0;
  uint32_t n;

  /* Generate the request packet */
  prq |= (1 << 0); /* Start Bit */
  for (n = 1; n < 5; n++) {
    bit = (request >> (n - 1)) & 0x1;
    prq |= bit << n;
    parity += bit;
  }
  prq |= (parity & 0x1) << 5; /* Parity Bit */
  prq |= (0 << 6); /* Stop Bit */
  prq |= (1 << 7); /* Park bit */
  probe_write_bits(8, prq);

  /* Turnaround (ignore read bits) */
  ack = (uint8_t)(probe_read_bits(conf->turnaround + 3) >> conf->turnaround);

  if (ack == SWD_ACK_OK_VAL) {
    /* Data transfer phase */
    if (request & SWD_REQ_RnW) {
      /* Read RDATA[0:31] - note probe_read shifts to LSBs */
      val = probe_read_bits(32);
      bit = (uint8_t)probe_read_bits(1);
      parity = __builtin_popcount(val);
      if ((parity ^ bit) & 1U) {
        /* Parity error */
        ack = SWD_ACK_ERR_VAL;
      }
      if (data)
        *data = val;
      /* Turnaround for line idle */
      probe_hiz_clocks(conf->turnaround);
    } else {
      /* Turnaround for write */
      probe_hiz_clocks(conf->turnaround);

      /* Write WDATA[0:31] */
      val = data ? *data : 0;
      probe_write_bits(32, val);
      parity = __builtin_popcount(val);
      /* Write Parity Bit */
      probe_write_bits(1, parity & 0x1);
    }

    /* Idle cycles - drive 0 for N clocks */
    if (conf->idle_cycles) {
      probe_write_bits(conf->idle_cycles, 0);
    }
    return ((uint8_t)ack);
  }

  if ((ack == SWD_ACK_WAIT_VAL) || (ack == SWD_ACK_FAULT_VAL)) {
    if (conf->data_phase && ((request & SWD_REQ_RnW) != 0U)) {
      /* Dummy Read RDATA[0:31] + Parity */
      probe_read_bits(33);
    }
    probe_hiz_clocks(conf->turnaround);
    if (conf->data_phase && ((request & SWD_REQ_RnW) == 0U)) {
      /* Dummy Write WDATA[0:31] + Parity */
      probe_write_bits(32, 0);
      probe_write_bits(1, 0);
    }
    return ((uint8_t)ack);
  }

  /* Protocol error */
  n = conf->turnaround + 32U + 1U;
  /* Back off data phase */
  probe_read_bits(n);
  return ((uint8_t)ack);
}

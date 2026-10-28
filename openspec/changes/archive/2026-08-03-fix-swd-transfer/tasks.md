# Tasks: Fix SWD Transfer PIO Buffer Structure

## Task 1: Rewrite swd_transfer() PIO buffer builder
Rewrite the PIO buffer construction in `swd_transfer()` to match MioLink's `swdtap_adiv5_prepare_pio_seq()` pattern.

**Files:** `firmware/src/agentla_swd.c`

- [x] For WRITE path: send data (32 bits) and parity (1 bit) as **separate** seq_out operations with idle cycles — NOT combined into single word (combined packing corrupts `out pc, 32` due to stale OSR bits after `pull block` no-op with autopull ON)
- [x] Remove `SWD_ORIGIN_START` from the end of write buffers
- [x] For READ path: read data+parity via `seq_in` (33 bits), then turnaround+idle via `seq_out_turnaround` + `seq_out`
- [x] Ensure the `push` instruction in seq_in correctly pushes 3-bit ACK to RX FIFO
- [x] Verify `pio_sm_get_blocking()` reads ACK correctly after the PIO buffer is sent

## Task 2: Remove debug printf statements
Clean up all debug `printf` calls added during investigation.

**Files:** `firmware/src/agentla_swd.c`, `firmware/src/agentla_target.c`

- [x] Remove `printf("SWD_DP_WR reg=...` from `swd_write_dp()`
- [x] Remove `printf("SWD: DP_SELECT write failed...` from `swd_write_ap()`
- [x] Remove `printf("SWD: AP_WRITE reg=...` from `swd_write_ap()`
- [x] Remove all `printf("UNLOCK:...` from `stm32f1_unlock_flash()`
- [x] Remove all `printf("FLASH ERASE:...` from `stm32f1_flash_erase()`
- [x] Remove `printf("SWD_DP_WR...` debug from `swd_transfer()` if present

## Task 3: Clean up flash algorithm code
Remove redundant and dead code from `agentla_target.c`.

**Files:** `firmware/src/agentla_target.c`

- [x] Remove redundant CSW/TAR writes in `stm32f1_flash_erase()` post-BSY SR check (TAR already set by `stm32f1_wait_flash_bsy()`)
- [x] Remove dead RP detection branch (`dpidr_identity == 0x6F`) from `target_detect()`
- [x] Add `#include "pico/time.h"` for `sleep_us()` in `stm32f1_wait_flash_bsy()`

## Task 4: Rebuild firmware
Rebuild the AgentLA firmware with all fixes applied.

- [x] Run `make -j$(nproc)` in build directory
- [x] Verify no compilation errors or warnings

## Task 5: Flash and test
Flash the updated firmware and verify SWD operations work.

- [x] Send BOOTSEL command to put RP2350 in BOOTSEL mode
- [x] Flash via `picotool load -f build/AgentLA.uf2`
- [x] Reboot with `picotool reboot`
- [x] Verify IDENTIFY returns correct firmware version
- [x] Test SWD IDCODE read via `SWD IDCODE` command
- [x] Test BluePill flash erase/write via `test/test_blink_bluepill.py`

## Task 5b: Critical root cause fix — separate data/parity FIFO entries
**Root cause:** The original code packed data+parity into separate FIFO words but with a combined bit count (`32 + 1 + idle - 1`). The `seq_out` PIO program's `pull block` instruction is a no-op when autopull is ON (OSR already has data), leaving 8 stale parity bits in the OSR. When `out pc, 32` executes, it shifts out these stale bits first, then autopull loads the idle data, constructing a corrupted PC value — the PIO jumps to a wrong origin.

**Fix:** Send data (32 bits) and parity (1 bit) as **separate** `seq_out` operations:
- First `seq_out`: bit count = 31, data = val (32 data bits)
- Second `seq_out`: bit count = idle, data = parity (1 parity bit + idle cycles)

**Files:** `firmware/src/agentla_swd.c`

- [x] Rewrite write path to use two separate seq_out operations
- [x] Verify read path parity extraction is correct (parity at bit[31] of second push)
- [x] Build firmware and verify compilation

## Task 6: Code review
Review all changes for correctness and cleanliness.

- [x] Spawn code-reviewer-mimo to review the full set of changes
- [x] Address any feedback from code review

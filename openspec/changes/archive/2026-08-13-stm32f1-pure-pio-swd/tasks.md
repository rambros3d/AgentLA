## 1. 100 kHz PIO Clock Scaling & Sub-program Alignment

- [ ] 1.1 Set default PIO state machine clock divider in `swd_sm_init()` for 100 kHz operation
- [ ] 1.2 Align `swd_seq_in_turnaround` and `swd_seq_out_turnaround` side-set states in `firmware/pio/swd.pio`
- [ ] 1.3 Build firmware and flash to RP2350 probe using `picotool`

## 2. Waveform Verification & Empirical Testing

- [ ] 2.1 Run `SWD CAPTURE` over `/dev/ttyACM0` to log 100 kHz PIO SWD request/ACK waveform on GPIO20/21
- [ ] 2.2 Verify PIO edge timing against bitbanging reference capture
- [ ] 2.3 Test `SWD PIO_IDCODE` over CDC serial `/dev/ttyACM0` until it returns `OK SWD PIO_IDCODE 0x1BA01477`

---

## PARKED (2026-08-12)

This change is **parked** in favor of `stm32f1-bitbang-debugger` (bitbang-first debugger/flasher).
Findings, fixes, verified results, and the open cross-transaction boundary issue are documented
in `design.md` under **PARKED (2026-08-12) — Findings & State Handoff**. Tasks 1.1-2.2 are
superseded by fixes already in the working tree; resume at 2.3 after the bitbang driver ships.

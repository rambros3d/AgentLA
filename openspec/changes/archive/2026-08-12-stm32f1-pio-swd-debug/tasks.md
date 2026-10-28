## 1. PIO Engine Isolation & FIFO Framing

- [x] 1.1 Refactor `swd_transfer` FIFO command buffer framing in `firmware/src/programmer/swd.c`
- [x] 1.2 Update `swd_seq_in` and `adiv5_check_ack` in `firmware/pio/swd.pio` for exact 3-bit ACK push and matching
- [x] 1.3 Build firmware and flash to RP2350 probe using `picotool`

## 2. Empirical Logic Analyzer Waveform Verification

- [x] 2.1 Run `SWD CAPTURE` over `/dev/ttyACM0` to record pure PIO SWD request/ACK waveform on GPIO20/21
- [x] 2.2 Compare PIO request/ACK clock edge alignment against bitbanging reference waveform
- [x] 2.3 Adjust PIO delay cycles (`[3]`) and pin direction timing until PIO produces identical edges

## 3. Pure PIO Verification & Clean-up

- [x] 3.1 Run `SWD PIO_IDCODE` over CDC serial `/dev/ttyACM0` and verify return of `0x1BA01477`
- [x] 3.2 Update `swd_get_idcode_with_fallback()` to rely on pure PIO SWD engine
- [x] 3.3 Execute `python3 test/test_stm32f1_autonomous_debug.py /dev/ttyACM0` and confirm 100% PASS

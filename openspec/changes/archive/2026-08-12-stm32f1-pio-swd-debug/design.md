## Context

Bitbanging proved that SWD ACK and DPIDR reads work when bit timings and pin directions are strictly controlled. The PIO engine returns `0xE0000000` because `swd_transfer` sends a multi-stage FIFO command sequence (`request` -> `seq_in_turnaround` -> `seq_in (3 ACK bits)` -> `adiv5_check_ack` -> `seq_in (33 data+parity bits)` -> `seq_out_turnaround (idle)` -> `start (park)`).

## Goals / Non-Goals

**Goals:**
- Fix PIO state machine instruction sequence and FIFO framing in `swd.pio` and `swd.c`.
- Ensure `swd_read_idcode()` (pure PIO) returns `SWD_OK` with `idcode = 0x1BA01477`.
- Validate pure PIO SWD across `SWD IDCODE`, `STM32 TEST`, and full autonomous test suite.

**Non-Goals:**
- Modifying target reset pin hardware wiring.

## Decisions

1. **PIO Direct Execution & FIFO Sync**:
   - Streamline `swd_transfer` so that request output (`seq_out`), ACK read (`seq_in`), and data read (`seq_in`) execute cleanly with explicit bit shifts and zero FIFO stalls.
   - Use `SWD CAPTURE` to oversample PIO clock edges at 100MHz to verify exact bit alignment.

## Risks / Trade-offs

- **[Risk]**: PIO autopush threshold mismatch on 3-bit ACK vs 33-bit data.
  - **Mitigation**: Perform explicit `push` in PIO after shifting 3 ACK bits, and rely on 32-bit autopush + 1 explicit push for 33 data/parity bits.

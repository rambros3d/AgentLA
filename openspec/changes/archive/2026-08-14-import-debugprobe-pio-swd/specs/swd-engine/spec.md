## MODIFIED Requirements

### Requirement: SWD Write Bits
The SWD engine SHALL write 1-32 bits to SWDIO with SWCLK generation via the debugprobe PIO transport's `probe_write_bits`, which uses an explicit `pull` of the data word and a continuous clock with direction driven in-band (no `current_swdio_dir` C-side cache, no seq_out/seq_out_turnaround sub-programs).

#### Scenario: Write 8-bit request
- **WHEN** swd_write_bits(8, request) is called on the PIO branch
- **THEN** the command word and 8-bit data word are pushed to the SM FIFO and 8 bits are clocked out on SWDIO with correct continuous SWCLK

#### Scenario: Write with turnaround
- **WHEN** a write follows a read/turnaround (SWDIO currently released)
- **THEN** the SM SHALL set SWDIO to output as part of the command dispatch (in-band direction), with no host-side direction tracking

### Requirement: SWD Read Bits
The SWD engine SHALL read 1-32 bits from SWDIO using the debugprobe PIO transport's `probe_read_bits` with explicit `push` (autopush disabled) and continuous SWCLK.

#### Scenario: Read 3-bit ACK
- **WHEN** swd_read_bits(3) is called during ACK phase
- **THEN** 3 bits are clocked in and returned as the captured word, shifted to LSBs

#### Scenario: Read 32-bit data
- **WHEN** swd_read_bits(32) is called during data phase
- **THEN** 32 bits are clocked in, the full word is pushed via explicit `push`, and returned unshifted

### Requirement: SWD Transfer PIO sequence
The `swd_transfer()` PIO branch SHALL execute the ported debugprobe `SWD_Transfer` sequence: request (8 bits out), ACK as `turnaround + 3` read bits with the first `turnaround` discarded, data phase on ACK OK (read 32+1 with parity verification / hi-Z turnaround then write 32+1), WAIT/FAULT dummy data-phase back-off, protocol-error re-sync, and idle cycles — replacing the MioLink buffer-structure (`SWD_ORIGIN_*` origin words, autopull filler words, combined data+parity+idle bit counts) approach.

#### Scenario: Write transfer ACK OK
- **WHEN** swd_transfer() performs a write with ACK OK
- **THEN** after a hi-Z turnaround the 32 data bits + 1 parity bit are written, followed by idle cycles, and the ACK is returned

#### Scenario: Read transfer ACK OK
- **WHEN** swd_transfer() performs a read with ACK OK
- **THEN** 32 data bits + 1 parity bit are read, parity verified, and the data word is returned with ACK OK

#### Scenario: Non-OK ACK leaves a defined line state
- **WHEN** the ACK is WAIT, FAULT, or a protocol error
- **THEN** the engine consumes the appropriate dummy/back-off bits (no `pio_sm_restart` + `jmp 0` recovery) and the next transaction starts from a clean state

### Requirement: SWD error recovery
The SWD engine SHALL recover from non-OK ACKs without destroying the PIO state machine: WAIT/FAULT consume the dummy data phase, protocol errors back off `turnaround + 32 + 1` read bits, and the SM continues its command loop — removing the previous `pio_sm_restart()`/`pio_sm_clear_fifos()`/`pio_sm_exec(jmp 0)` recovery path.

#### Scenario: FAULT on write followed by retry
- **WHEN** a write returns FAULT
- **THEN** the engine consumes the dummy data phase and a subsequent transaction on the same SM SHALL execute correctly without re-initialization

## ADDED Requirements

### Requirement: ARM SWD Protocol Sniffer
The firmware SHALL monitor SWCLK and SWDIO lines, decode DP/AP register reads and writes (DPIDR, CTRL/STAT, SELECT, TAR, DRW), 3-bit ACK status, and verify data parity bits.

#### Scenario: Decoding DPIDR read transaction
- **WHEN** an SWD read of DP register 0x00 occurs returning ACK OK (0b001) and IDCODE `0x0BC12477` with valid parity
- **THEN** the decoder records an `SWD_DP_READ` event with Register `DPIDR` and Value `0x0BC12477`

### Requirement: ARM SWO / ITM Trace Printf Decoder
The firmware SHALL decode asynchronous UART trace frames from the Cortex-M SWO pin and extract ITM Stimulus Port 0 payload bytes into an ASCII debug stream.

#### Scenario: Reconstructing ITM printf text
- **WHEN** SWO line receives ITM packets on stimulus port 0 containing ASCII bytes "Boot OK\n"
- **THEN** the decoder emits the decoded string "Boot OK\n" to the serial trace output

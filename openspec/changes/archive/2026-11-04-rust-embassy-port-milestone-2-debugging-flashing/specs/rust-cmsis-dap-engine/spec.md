## ADDED Requirements

### Requirement: Full CMSIS-DAP Command Dispatch
The firmware SHALL implement the standard CMSIS-DAP command suite across both CMSIS-DAP v1 (HID) and CMSIS-DAP v2 (WinUSB Bulk) endpoints.

#### Scenario: DAP_Transfer block read
- **WHEN** a host tool (such as PyOCD or probe-rs) issues a `DAP_TransferBlock` request to read multiple memory words via AP 0
- **THEN** the firmware streams back the requested data words over USB with `DAP_TRANSFER_OK` status

#### Scenario: DAP_SWJ_Pins pin control
- **WHEN** a host tool issues a `DAP_SWJ_Pins` command to assert or deassert `nRST` or `SWCLK`
- **THEN** the firmware drives the specified GPIO lines according to the pin mask and output value

#### Scenario: DAP_SWJ_Sequence raw sequence
- **WHEN** a host tool sends a `DAP_SWJ_Sequence` command with raw clock bit patterns
- **THEN** the firmware clocks the exact bit stream out on `SWDIO`/`SWCLK` via the PIO transport

### Requirement: High-Throughput Bulk Streaming
The firmware SHALL support WinUSB bulk transfers up to the maximum endpoint packet size to maximize flash read and write throughput.

#### Scenario: Continuous flash programming burst
- **WHEN** `probe-rs` sends consecutive 512-byte `DAP_TransferBlock` write packets to target SRAM
- **THEN** the firmware buffers and executes each block without dropping USB packets or timing out

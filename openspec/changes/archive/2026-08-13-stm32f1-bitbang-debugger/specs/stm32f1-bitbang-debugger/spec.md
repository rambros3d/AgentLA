# Delta Spec: Bitbang Debugger & Flasher Driver

## Added Requirements

### Requirement: Runtime Driver Selection
The programmer layer SHALL expose a runtime-selected transport driver (`PROG_DRIVER_BITBANG` default, `PROG_DRIVER_PIO` selectable) via the `PROG DRIVER BITBANG|PIO` CLI command, switching the active transport backend without a firmware rebuild.

#### Scenario: Switch to Bitbang Default
- **WHEN** the probe boots
- **THEN** the transport driver SHALL be `BITBANG` and `PROG DRIVER` SHALL report `OK PROG DRIVER BITBANG`

#### Scenario: Switch to PIO Mode
- **WHEN** user issues `PROG DRIVER PIO` over CDC serial
- **THEN** the response SHALL be `OK PROG DRIVER PIO` and subsequent transfers use the PIO engines

### Requirement: Bitbang SWD Transport
The SWD engine SHALL perform full DP/AP read and write transactions (request, turnaround, ACK, data+parity, idle) using the bitbang backend, returning ACK codes mapped to `SWD_OK` / `SWD_ERR_ACK_WAIT` / `SWD_ERR_ACK_FAULT` / `SWD_ERR_PROTOCOL`.

#### Scenario: SWD DP Read after Connect
- **WHEN** the probe is connected to an STM32F103 target and user issues `SWD PIO_IDCODE` followed by `SWD READ DP 0x0`
- **THEN** the response SHALL be `OK SWD DP[0x00] = 0x1BA01477`

#### Scenario: SWD DP Write
- **WHEN** user issues `SWD WRITE DP 0x4 0x50000000` on a connected STM32F103
- **THEN** the response SHALL be `OK SWD DP[0x04] = 0x50000000`

### Requirement: Fast Bitbang Connect
The SWD connect sequence SHALL be bitbang-first (line reset → JTAG-to-SWD switch → DPIDR read as the first transaction → ABORT/CTRLSTAT writes after) and SHALL complete IDCODE reads in milliseconds, without the PIO retry loop.

#### Scenario: IDCODE Command is Fast
- **WHEN** user issues `SWD IDCODE` on a connected STM32F103
- **THEN** the response SHALL be `OK SWD IDCODE 0x1BA01477` within 5 seconds

### Requirement: Full Debugger & Flasher on Bitbang
`DEBUG HALT/RUN/STEP/REGS/READ/WRITE`, `FLASH ERASE/WRITE/READ/VERIFY`, and `target_detect` SHALL succeed over the bitbang transport on the STM32F103 target.

#### Scenario: Cortex-M3 Halt
- **WHEN** user issues `DEBUG HALT` then `DEBUG REGS` on a connected STM32F103
- **THEN** the responses SHALL be `OK DEBUG HALT` and `OK DEBUG REGS` with register values

#### Scenario: Flash Write and Verify
- **WHEN** user issues `FLASH WRITE <addr> <hex>` then `FLASH VERIFY <addr> <hex>` on a connected STM32F103
- **THEN** the responses SHALL be `OK FLASH WROTE ...` and `OK FLASH VERIFY PASS`

### Requirement: Bitbang SWIO Transport (CH32)
The SWIO engine SHALL perform WCH 1-wire reads/writes (begin, 8 data bits LSB-first, complement, end) using the bitbang backend, and CH32 programming commands SHALL succeed.

#### Scenario: CH32 Target Probe
- **WHEN** the probe targets a CH32V203 over SWIO and user issues the CH32 probe/read command
- **THEN** the response SHALL report the CH32 device ID successfully

### Requirement: Bitbang RVSWD Transport (RISC-V)
The RVSWD engine SHALL perform RISC-V debug access using the bitbang backend, and RV target detection (CH32V003) SHALL succeed.

#### Scenario: CH32V003 RV Probe
- **WHEN** the probe targets a CH32V003 and issues the RV probe command
- **THEN** the response SHALL report the RV target successfully

### Requirement: PIO Diagnostic Mode Preserved
While bitbang is the default, the PIO engines SHALL remain available for diagnostics (`SWD PIO_IDCODE`, `SWD CAPTURE`) and SHALL not be corrupted by bitbang sessions.

#### Scenario: Capture after Bitbang Session
- **WHEN** user runs a bitbang session then issues `SWD CAPTURE PIOREAD`
- **THEN** the capture SHALL report `res=0 idcode=0x1BA01477` and the waveform SHALL show request/ACK/data

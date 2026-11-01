## 1. PIO Program & Build System Integration

- [x] 1.1 Copy and adapt `i3c.pio` into `firmware/pio/i3c.pio` with RP2350 compatibility
- [x] 1.2 Update `CMakeLists.txt` to run `pico_generate_pio_header` for `firmware/pio/i3c.pio`
- [x] 1.3 Add dynamic PIO program allocation macros targeting `pio2` in `firmware/src/board_settings.h`

## 2. I3C Controller Driver Implementation

- [x] 2.1 Create `firmware/src/adapter/adapter_i3c.h` defining status codes, transfer structures, and public API
- [x] 2.2 Implement `firmware/src/adapter/adapter_i3c.c` with `adapter_i3c_init` targeting `PIO2` and Adapter pins (GPIO 10 & 11)
- [x] 2.3 Implement dynamic clock rate divisor calculation using `clock_get_hz(clk_sys)` supporting frequencies up to 12.5 MHz
- [x] 2.4 Implement Dynamic Address Assignment (`adapter_i3c_entdaa`) and target reset / DAA reset (`adapter_i3c_rstdaa`)
- [x] 2.5 Implement Common Command Codes (Direct CCC read/write, Broadcast CCC write)
- [x] 2.6 Implement private SDR read, write, and combined write-read with T-bit parity validation
- [x] 2.7 Implement In-Band Interrupt (IBI) polling and arbitration header handler

## 3. CLI & Command Bridge Integration

- [x] 3.1 Integrate `pin_owner` claim and release for `GPIO 10` & `GPIO 11` when entering/exiting I3C adapter mode
- [x] 3.2 Add CLI command handlers in `firmware/src/main.c` (`I3C_INIT`, `I3C_SCAN`, `I3C_ENTDAA`, `I3C_RSTDAA`, `I3C_READ`, `I3C_WRITE`, `I3C_CCC`, `I3C_POLL`, `I3C_RESET`)
- [x] 3.3 Wire I3C commands through `network_execute_command` for Web UI and REST/SSE bridge access

## 4. STM32H503 Target Test Fixture

- [x] 4.1 Create lightweight STM32H503 I3C Target firmware test fixture in `test/fixtures/stm32h503_i3c_target/`
- [x] 4.2 Configure STM32H503 `I3C1` peripheral on PB6 (SCL) and PB7 (SDA) in Target mode with provisional ID
- [x] 4.3 Implement target-side dynamic address assignment acceptance, 32-byte test register memory, and IBI trigger button/command

## 5. Automated Verification Suite

- [x] 5.1 Implement automated test runner `test/test_i3c_stm32h503_suite.py` that connects to AgentLA over serial
- [x] 5.2 Validate target flash provisioning: connect via SWD, identify STM32H503 (0x474), flash target test binary, and reset
- [x] 5.3 Validate I3C bus bring-up: run `I3C_INIT`, execute `I3C_ENTDAA`, and verify assigned dynamic address and target PID
- [x] 5.4 Validate private SDR read/write: perform 32-byte register write and read-back loopback verification at 100 kHz, 1 MHz, and 12.5 MHz
- [x] 5.5 Validate CCC and IBI: send direct CCC `GETSTATUS`, trigger target IBI, and verify AgentLA detects MDB without bus hang

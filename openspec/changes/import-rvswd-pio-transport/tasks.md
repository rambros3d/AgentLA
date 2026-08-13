# Tasks: Import RVSWD PIO Transport & CH32V203 Hardware Verification

## 1. Transport Vendor & PIO Driver Setup

- [ ] 1.1 Create `firmware/src/programmer/rvswd/` directory structure
- [ ] 1.2 Vendor `references/RVSWD_pico/RVSWD_full.pio` $\rightarrow$ `firmware/src/programmer/rvswd/rvswd.pio`
- [ ] 1.3 Port `references/RVSWD_pico/RVSWD_pio.c` & `rvswd.h` $\rightarrow$ `firmware/src/programmer/rvswd/rvswd_pio_transport.c` and `rvswd_pio_transport.h`
- [ ] 1.4 Implement non-blocking bounded FIFO put/get routines (`rvswd_put_bounded`) with 1ms deadline
- [ ] 1.5 Update `CMakeLists.txt` to compile `rvswd.pio` header and link `rvswd_pio_transport.c`

## 2. Dynamic PIO0 SM0 Allocation & Pin Integration

- [ ] 2.1 Implement dynamic program unloading/loading between ARM SWD (`probe.pio`) and RVSWD (`rvswd.pio`) on `PIO0 SM0`
- [ ] 2.2 Wire `swd_deinit()` and `rvswd_deinit()` to call `pio_remove_program()` for clean instruction memory management
- [ ] 2.3 Ensure `PROG_MODE_IS_ACTIVE()` protection wraps RVSWD `PIO0` usage to prevent collisions with Logic Analyzer capture mode
- [ ] 2.4 Verify build succeeds cleanly without PIO memory allocation errors

## 3. RISC-V DMI & CH32 Debug Control Layer

- [ ] 3.1 Connect `riscv_dmi_read()` and `riscv_dmi_write()` in `firmware/src/programmer/riscv_dmi.c` to the new RVSWD transport
- [ ] 3.2 Add retry logic (up to 5 retries) for DMI register access failures
- [ ] 3.3 Implement `ch32v20x_halt_microprocessor()`, `ch32v20x_resume_microprocessor()`, and `ch32v20x_reset_microprocessor_and_run()` in `ch32.c`
- [ ] 3.4 Wire CPU register read/write (`COMMAND` / `DATA0` abstract command interface) for CH32V203

## 4. Primary Verification Phase 1: ARM SWD BluePill Regression Sweep

- [ ] 4.1 Build firmware and flash test probe
- [ ] 4.2 Run `test_swd_hardware.py`: Verify 12/12 hardware SWD tests PASS (SWD IDCODE `0x1BA01477`, DP/AP access, SWD reset)
- [ ] 4.3 Run `test_stm32f1_hardware_suite.py`: Verify 5/5 TIERS PASS (full flash erase $\rightarrow$ program 9920 bytes $\rightarrow$ byte-exact verify $\rightarrow$ reset $\rightarrow$ boot)
- [ ] 4.4 Confirm zero regressions to ARM SWD capability before proceeding to CH32 hardware validation

## 5. Primary Verification Phase 2: CH32V203 Hardware Read, Flash & Debug Validation

- [ ] 5.1 **CH32V203 Target Detection & Read**:
  - [ ] Connect probe to CH32V203 dev board (GPIO4=SWCLK, GPIO5=SWDIO, GND)
  - [ ] Run `CH32 DETECT mcu=ch32v203`
  - [ ] Verify `DMSTATUS` link up (`0x00000C82`), `chip_id=0x20305040`, `flash=64KB`, `page=256B`
- [ ] 5.2 **CH32V203 Flash Operations**:
  - [ ] Run `CH32 ERASE` at `0x08000000` (256 bytes) and full chip erase
  - [ ] Program firmware binary payload to `0x08000000`
  - [ ] Read back flash content and verify 100% byte-exact match with binary
- [ ] 5.3 **CH32V203 Debug Operations**:
  - [ ] Issue `CH32 HALT` / `DEBUG HALT`: Verify CPU halts (`DMSTATUS` halted bits set)
  - [ ] Read CPU registers (`R0`–`R15`, `PC`): Verify valid values returned
  - [ ] Issue `DEBUG STEP`: Single-step instruction, verify `PC` increments
  - [ ] Mutate CPU register / SRAM address: Write marker value, read back, verify match
  - [ ] Issue `CH32 RESUME` / `DEBUG RUN`: Verify target resumes execution smoothly
- [ ] 5.4 Document all test results and mark change complete

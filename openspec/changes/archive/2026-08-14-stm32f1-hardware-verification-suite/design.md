## Context

AgentLA combines an RP2350 high-speed logic analyzer with a multi-protocol hardware programmer (SWD, RISC-V RV-SWD, 1-wire SDI, CMSIS-DAP). To ensure full hardware reliability on STM32 microcontrollers, we require a dedicated 5-tier test suite combined with low-level SWD signal debugging and automatic `SWD_ACK_WAIT` protocol recovery.

## Goals / Non-Goals

**Goals:**
- Implement automated Python-based 5-tier test suite (`test/test_stm32f1_hardware_suite.py`) testing DP/AP, Cortex-M3 core control, SRAM memory bus, STM32F1 FPEC flash driver, and real-time PC13 LED signal capture.
- Add hardware debugging pipeline: SWD clock fallback (1000 kHz → 500 kHz → 100 kHz), SWD signal line diagnostics via `SWD DIAG`, and 100MHz logic analyzer capture of SWCLK/SWDIO waveforms.
- Add firmware-side `STM32 TEST` and `SWD DIAG` CDC commands in `main.c` for fast CLI diagnostics.
- Ensure 100% regression safety across SWD clock rates (100 kHz, 500 kHz, 1 MHz, 2 MHz, 5 MHz).

**Non-Goals:**
- Testing non-ARM RISC-V architectures (covered by CH32 suite).
- Modifying underlying SWD PIO state machine binary layout (`swd.pio`).

## Decisions

1. **5-Tier Tiered Test Architecture**:
   - Split verification into 5 isolated tiers so failures pinpoint the exact layer (Transport → Core → Bus → Flash → Signal Output).

2. **Automatic SWD Clock Fallback & WAIT Recovery**:
   - On `SWD_ACK_WAIT` (`011`), automatically lower SWD clock frequency to 500 kHz and retry `DP_ABORT` + `DP_CTRLSTAT` power up before declaring connection failure.

3. **GPIO18 Real-Time Signal Validation**:
   - Route Blue Pill `PC13` LED pin to Probe `GPIO18` to verify that after flashing, the MCU executes code and blinks the LED at 1 Hz.

4. **CDC Command `STM32 TEST` & `SWD DIAG` Integration**:
   - Expose `STM32 TEST` and `SWD DIAG` in `firmware/src/main.c` to run DPIDR, `DP_CTRLSTAT`, `AHB-AP`, DBGMCU, Flash geometry, and pin level checks in single atomic CDC calls.

## Risks / Trade-offs

- **[Risk]**: User target board missing common GND reference causing SWD ACK floating.
  - **Mitigation**: Pre-flight test prints clear wiring requirements and checks SWDIO idle state before starting suite.
- **[Risk]**: Target stuck in `SWD_ACK_WAIT` due to low internal CPU clock rate (8 MHz HSI).
  - **Mitigation**: Auto-clock fallback to 500 kHz / 100 kHz with 8 idle cycles gives target internal bus matrix adequate synchronization cycles.

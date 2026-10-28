## Why

To verify both the programming and real-time hardware debugging capabilities of the AgentLA RP2350 probe on an STM32F103 (Blue Pill) target, we need a dedicated heartbeat blink firmware (double-pulse cardiac rhythm) coupled with interactive hardware debugging operations (HALT, RUN, STEP, REGISTER READ/WRITE, MEMORY READ/WRITE, and BREAKPOINT).

## What Changes

- Update `test/bluepill_blink/src/main.cpp` to implement a double-beat cardiac pulse pattern (100ms ON / 100ms OFF / 100ms ON / 700ms OFF).
- Create `test/test_stm32f1_heartbeat_debug.py` test harness to flash the heartbeat payload via SWD, execute interactive Cortex-M3 hardware debugging commands over CDC, and sample PC13 LED output on `GPIO22` (L_CH2) with the AgentLA 100MHz logic analyzer.
- Expose Cortex-M hardware debug CDC commands (`DEBUG HALT`, `DEBUG RUN`, `DEBUG STEP`, `DEBUG REGS`, `DEBUG READ <addr>`, `DEBUG WRITE <addr> <val>`) in `firmware/src/main.c`.

## Capabilities

### New Capabilities
- `stm32f1-heartbeat-debug`: Defines requirements for heartbeat blink firmware compilation, interactive hardware debug operations (halt/step/regs/memory), and logic analyzer PC13 LED double-pulse signal verification.

### Modified Capabilities
- None.

## Impact

- `test/bluepill_blink/src/main.cpp`: Updated to double-pulse heartbeat pattern.
- `firmware/src/main.c`: Registered `DEBUG` hardware control subcommands over CDC serial.
- `test/test_stm32f1_heartbeat_debug.py`: Created 5-stage automated heartbeat & debug verification suite.

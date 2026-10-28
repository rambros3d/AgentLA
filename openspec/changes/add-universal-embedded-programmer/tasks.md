## 1. Spikes (validation, no production change)

- [ ] 1.1 Spike A: implement a Cortex-M algorithm execution helper (DCRSR/DCRDR) and reproduce the STM32F1 5/5 tier suite (test_stm32f1_hardware_suite.py) via RAM-resident algorithm instead of direct registers.
- [ ] 1.2 Spike B: select a dedicated BOOT0 GPIO, wire it on a Pico + Blue Pill bench, and complete one AN3155 flash + verify + boot cycle.
- [ ] 1.3 Spike C: verify optimistic mass-erase behavior on a locked F1 part and confirm post-erase re-identification reports protection=none.
- [x] 1.5 Research task: investigate recovery options for targets with permanently removed ROM bootloaders ( Level-2 / option-byte locks) and record findings; keep out of v1 scope.
- [x] 1.4 Document spike results (timing, reliability, failures) in design.md "Findings" before proceeding.

## 2. Universal target identification

- [x] 2.1 Refactor target_detect() to remove hardcoded density defaults and query factory info (STM32F1 option bytes, F4+ factory row) for flash/RAM sizes.
- [x] 2.2 Add protection-level detection for STM32, Nordic, and WCH targets.
- [ ] 2.3 Implement universal-target-detection spec scenarios with hardware tests on Blue Pill and a locked F1 part.
- [ ] 2.4 Provide an emulator/mock target path if available for CI checks.

## 3. Generic Cortex-M algorithm runner

- [x] 3.1 Implement swd_core_exec (halt, set registers, resume) primitives using CMSIS-DAP-level debug registers.
- [x] 3.2 Implement RAM download helper over SWD with chunked posted writes and verification readback.
- [x] 3.3 Define AgentLA blob format (descriptor + code) and metadata fields; document ABI.
- [ ] 3.4 Implement algo_runner.c: Init/EraseSector/ProgramPage/Verify/UnInit orchestration.
- [ ] 3.5 Add consent flag and CONSENT-required enforcement.
- [ ] 3.6 Benchmark generic-runner throughput against the direct F1 implementation and require parity or better before task 7.1 (performance gate).
- [x] 3.7 Add host-side converter tool in sdk/ for Keil .flm -> AgentLA blob + JSON.

## 4. Debug mass-erase recovery

- [x] 4.1 Implement family-specific mass-erase (STM32, Nordic, WCH) through ADIv5 AP writes. (STM32 first)
- [x] 4.2 Add TARGET MASS ERASE command with pre-erase identity and confirmation.
- [x] 4.3 Implement post-erase re-identification and protection=none verification.
- [ ] 4.4 Add hardware tests on RDP Level 1 parts.

## 5. Bootloader recovery (ST + WCH)

- [x] 5.1 Select and document a dedicated BOOT0 GPIO in board_settings.h; update pin allocation docs; keep a guard for builds without the wiring.
- [ ] 5.2 Implement ST AN3155 orchestration (enter/program/exit) reusing test_an3155 protocol.
- [ ] 5.3 Implement WCH bootloader recovery module.
- [x] 5.4 Add Nordic recovery via mass-erase (no ROM bootloader required).
- [x] 5.5 Add bootloader coverage documentation for ST/Nordic/WCH families.
- [ ] 5.6 Add recovery tests with Blue Pill in bootloader mode.

## 6. Safety and UX

- [x] 6.1 Enforce pre-destructive identity/geometry checks in all entry points.
- [ ] 6.2 Add confirmation tokens for mass erase and bootloader recovery.
- [x] 6.3 Emit informative banners (part, bytes, protection level) before destructive operations.
- [x] 6.4 Add range checks and clear error codes; document CLI responses.

## 7. Migration and cleanup

- [ ] 7.1 Replace STM32F1 direct algorithms with generic runner primary path.
- [ ] 7.2 Keep direct algorithms behind feature flag for transitional validation.
- [ ] 7.3 Update specs and docs to reflect generic-primary approach.
- [ ] 7.4 Remove direct algorithms after hardware parity confirmed.
- [x] 7.5 Re-run full BluePill ARM regression: test_swd_hardware.py and test_stm32f1_hardware_suite.py.

## 8. Coordination

- [x] 8.1 Ensure no overlap/collisions with openspec changes import-rvswd-pio-transport and vendor-picorvd-progbuf-mem-access before merge.
- [x] 8.2 Update note.txt and docs/pin-allocation.md for GPIO7 BOOT0 usage.
- [ ] 8.3 Final review and archive after all hardware gates pass.
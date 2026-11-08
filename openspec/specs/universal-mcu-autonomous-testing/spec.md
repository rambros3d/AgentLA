# universal-mcu-autonomous-testing Specification

## Purpose
Provides a structured autonomous hardware testing methodology and closed-loop execution workflow for AI agents interacting with microcontroller targets over AgentLA MCP tools. Coordinates target electrical rail verification, silicon architecture auto-discovery, vector table invariant validation, interactive bus stimulus, and autonomous CoreSight register fault triage.

## Requirements

### Requirement: Target Electrical & Silicon Auto-Discovery
The testing skill SHALL automatically inspect target rail voltage, test pin continuity, and identify target silicon family and memory bounds prior to executing any destructive operations.

#### Scenario: Discovering nominal target rail
- **WHEN** agent runs the auto-discovery phase against a target MCU
- **THEN** target voltage is confirmed $\ge 2.7\text{V}$, no pins are shorted to ground, and target architecture (ARM CoreSight or RISC-V DM) is identified with valid flash base and SRAM sizes

#### Scenario: Aborting on unpowered or brownout rail
- **WHEN** target rail voltage is measured below $2.7\text{V}$
- **THEN** the skill aborts flash programming immediately and emits an electrical rail warning advising power injection

### Requirement: Pre-Flash Invariant Validation
The testing skill SHALL validate firmware vector table invariants against target memory architecture before streaming flash blocks.

#### Scenario: Validating stack pointer and Thumb mode
- **WHEN** agent inspects an ARM Cortex-M firmware binary
- **THEN** Initial SP is verified to fall within the target's physical SRAM range and the Reset Vector LSB is verified to be 1 (Thumb mode active)

#### Scenario: Detecting out-of-bounds stack pointer
- **WHEN** Initial SP points outside the physical SRAM range identified by `catalog_lookup`
- **THEN** flash deployment is halted with an explicit boundary error preventing target boot lockup

### Requirement: Closed-Loop Boot & Interactive Bus Stimulus
The testing skill SHALL enforce a post-reset settling delay and execute multi-protocol stimulus (UART, CAN, I2C, SPI) against the running target.

#### Scenario: Executing UART console regression test
- **WHEN** target is reset following flash verification
- **THEN** the skill waits at least 100 ms for bootloader settling, transmits expected command sequences over `uart_command`, and validates response substrings

### Requirement: Autonomous HardFault Triage & Diagnostic Reporting
Upon detecting a stimulus timeout, NACK, or abnormal target state, the testing skill SHALL autonomously execute CoreSight register triage and synthesize the root cause of the failure.

#### Scenario: Triaging an unaligned access HardFault
- **WHEN** a target stops responding during bus communication
- **THEN** the skill queries `target_diagnose`, captures CFSR/HFSR/BFAR registers, synthesizes the exact fault reason (e.g., `UsageFault: Unaligned memory access at PC 0x08000120`), and formats the final test report with failure triage

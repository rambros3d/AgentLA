# protocol-hardware-trigger-matrix Specification

## Purpose
Provides a real-time, low-latency predicate evaluation matrix evaluating incoming decoded protocol frames against user-defined rule patterns, firing immediate hardware actions including capture freeze, instrument trigger pin assertion, SWD target halting, and WebSocket push alerts.

## Requirements

### Requirement: Programmable Protocol Trigger Rule Engine
The system SHALL provide a Hardware Trigger Matrix that evaluates decoded protocol events in real time against up to 8 user-configured predicate rules without allocating heap memory.

#### Scenario: Rule matching on I2C NACK
- **WHEN** rule specifies `{ bus: "i2c", match_addr: 0x48, match_ack: false }`
- **THEN** engine evaluates true when a `DecodedEvent::I2cTransaction` matching address `0x48` with `ack == false` is processed

#### Scenario: Rule matching on CAN arbitration ID
- **WHEN** rule specifies `{ bus: "can", match_id: 0x7DF, match_id_mask: 0x7FF }`
- **THEN** engine evaluates true when a CAN frame with standard ID `0x7DF` is decoded

### Requirement: Multi-Action Trigger Dispatcher
The trigger engine SHALL dispatch configured hardware actions within 1 microsecond of a rule evaluation match:
1. `FreezeCapture`: Stop incoming logic analyzer DMA buffers while preserving pre-trigger samples.
2. `AssertTriggerPin`: Drive a designated probe GPIO pin high to trigger external lab instruments.
3. `HaltSwdTarget`: Issue an immediate SWD halt command via `agentla-programmer` to pause the target microcontroller.
4. `EmitWsAlert`: Push a high-priority JSON alert event over the WebSocket stream.

#### Scenario: Trigger asserts capture freeze and SWD halt
- **WHEN** a trigger rule configured with `[FreezeCapture, HaltSwdTarget]` evaluates true
- **THEN** logic analyzer transitions to Captured state within the configured post-sample count and target CPU core is halted via SWD

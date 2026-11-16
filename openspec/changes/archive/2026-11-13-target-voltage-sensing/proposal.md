## Why

When debugging embedded targets, knowing whether the target MCU is actually powered—and at what voltage (e.g. 1.8V, 3.3V, or 5.0V)—is critical. Currently, AgentLA's ST-Link command `0xF7` (`STLINK_GET_TARGET_VOLTAGE`) hardcodes `3300 mV`, which prevents host IDEs (STM32CubeIDE, STM32CubeProgrammer, OpenOCD) from detecting unpowered boards, power dips, or incorrect supply rails.

Adapting the target voltage sensing mechanism from `ref/hybridlink` ([`app/stlink.c`](file:///home/sun/Filelink/Firmwares/AgentLA-rust/ref/hybridlink/app/stlink.c)) by connecting RP2350's hardware ADC target rail channel directly to the probe protocol and telemetry stack enables real-time voltage reporting, unpowered board detection, and power safety alerts.

## What Changes

- **Live ADC Target Rail Hook**: Connect `StLinkHandler`'s `STLINK_GET_TARGET_VOLTAGE` (`0xF7`) command to a live hardware voltage provider instead of returning a hardcoded `3300 mV`.
- **Target Voltage Sensing Engine**: Provide a clean provider abstraction in `agentla-programmer` allowing the firmware to inject live target voltage readings (in millivolts) sampled from the RP2350 SAR ADC (ADC channel 2).
- **ST-Link F7 Response Compliance**: Return genuine 8-byte ST-Link voltage packets: 4 bytes ADC reference factor (`2400`), followed by 4 bytes target voltage in millivolts (`v_target_mv`), matching genuine ST-Link and `ref/hybridlink` behavior.
- **Unpowered Target Detection**: If target voltage is below 800 mV (e.g. 0V), return 0 mV so host IDEs immediately warn *"Target voltage not detected"* before failing SWD transactions.
- **REST & Telemetry Integration**: Ensure live target voltage is seamlessly queryable via `/api/v1/system/telemetry` and displayed on the WebUI dashboard top bar HUD.

## Capabilities

### New Capabilities
- `target-voltage-monitoring`: Precision hardware ADC acquisition and scaling of target board VCC rail with unpowered threshold detection.

### Modified Capabilities
- `stlink-probe-endpoint`: Update `STLINK_GET_TARGET_VOLTAGE` (`0xF7`) handling to report live measured millivolts.

## Impact

- **Crates**:
  - `crates/agentla-programmer`: Add target voltage callback / provider hook to `StLinkHandler`.
  - `firmware-rs`: Wire `telemetry::sample_telemetry().v_target_mv` into the active `StLinkHandler`.
- **User Experience**: STM32CubeIDE, CubeProgrammer, and OpenOCD instantly warn users when target boards lose power or run at unexpected supply voltages.

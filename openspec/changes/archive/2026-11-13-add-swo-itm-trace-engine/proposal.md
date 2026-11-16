## Why

ARM Cortex-M debugging relies heavily on Serial Wire Output (SWO) for non-intrusive Instrumentation Trace Macrocell (ITM) logging, printf console output, and statistical PC sampling. Currently, AgentLA's ST-Link personality stubs out trace commands and omits the USB trace endpoint (EP 0x82), preventing STM32CubeIDE Serial Wire Viewer (SWV) and OpenOCD trace capture from working.

By adapting the circular DMA UART / PIO trace architecture and live health monitoring from `ref/hybridlink`, AgentLA can capture sustained high-speed SWO trace up to 4.5 Mbaud without lost packets and surface real-time receiver health metrics.

## What Changes

- **SWO Hardware Receiver**: Implement a high-speed SWO NRZ/UART capture engine (using RP2350 UART/PIO) with a circular ring buffer, supporting baud rates up to 4.5 Mbaud (default 2 Mbaud).
- **Overrun Protection**: Implement overrun management that discards the oldest half of the circular buffer during host polling stalls without corrupting packet boundaries or causing USB stalls.
- **ST-Link Trace Endpoint & Protocol**: Add USB EP 0x82 (Bulk/Interrupt IN) to the ST-Link descriptor. Implement `DBG_APIV2_START_TRACE_RX` (`0xF2 0x40`), `DBG_APIV2_STOP_TRACE_RX` (`0xF2 0x41`), and `DBG_APIV2_GET_TRACE_NB` (`0xF2 0x42`) with polled and continuous streaming modes.
- **CMSIS-DAP SWO Support**: Connect the SWO receiver to `DAP_SWO_Data` (`0x17`) and `DAP_SWO_Control`.
- **Live Health Telemetry**: Track total bytes received from target, total bytes delivered to host, and bytes dropped. Expose via REST endpoint `/api/programmer/swo/status` and WebUI status indicator.

## Capabilities

### New Capabilities
- `swo-hardware-receiver`: Hardware UART/PIO capture of ARM Cortex-M SWO NRZ trace with configurable baud rate and overrun-resilient circular buffering.
- `swo-telemetry-status`: Live diagnostic telemetry tracking receiver state, bytes received, delivered, and dropped over REST and WebUI.

### Modified Capabilities
- `stlink-probe-endpoint`: Add physical USB EP 0x82 trace endpoint and implement trace control commands (`0xF2 0x40`, `0x41`, `0x42`).

## Impact

- **Crates**:
  - `crates/agentla-programmer`: Add `swo.rs` trace ring buffer, baud configuration, and telemetry structs; update `stlink.rs` to route trace queries and byte counts.
  - `firmware-rs`: Update `usb.rs` ST-Link descriptor to add EP 0x82; implement `swo_trace_task`; add `/api/programmer/swo/status` in `rest.rs`.
- **Hardware Resources**:
  - Uses 1 physical IN endpoint (EP 0x82) in ST-Link mode (total endpoints in ST-Link mode increases from 10 to 11, still well within the 16 hardware limit).
  - Assigns a dedicated pin for SWO input (e.g. GPIO 15 or flexible adapter pin via pin arbiter).

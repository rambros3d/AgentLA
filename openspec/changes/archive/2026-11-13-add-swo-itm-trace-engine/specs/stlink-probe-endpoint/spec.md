## MODIFIED Requirements

### Requirement: ST-Link/V2-1 USB Interface Enumeration
The system SHALL enumerate as an ST-Link/V2-1 compatible USB device when ST-Link mode is active, presenting Vendor ID `0x0483`, Product ID `0x3752`, and a vendor debug interface with Bulk OUT (EP 0x01), Bulk IN (EP 0x81), and Trace IN (EP 0x82).

#### Scenario: ST-Link Device Enumeration
- **WHEN** ST-Link programmer mode is activated
- **THEN** the USB device presents VID `0x0483`, PID `0x3752`, and vendor debug interface on EP 0x01 (out), EP 0x81 (in), and EP 0x82 (trace in)

## ADDED Requirements

### Requirement: ST-Link SWO Trace Protocol Commands
The system SHALL handle ST-Link trace commands `DBG_APIV2_START_TRACE_RX` (`0xF2 0x40`), `DBG_APIV2_STOP_TRACE_RX` (`0xF2 0x41`), and `DBG_APIV2_GET_TRACE_NB` (`0xF2 0x42`), and stream exact trace packet byte counts over EP 0x82.

#### Scenario: Start and stop trace commands
- **WHEN** host issues `0xF2 0x40` specifying baud rate and buffer parameters
- **THEN** system starts the SWO receiver, configures the baud rate, and returns status OK

#### Scenario: Polled trace query and streaming
- **WHEN** host issues `0xF2 0x42` (GET_TRACE_NB)
- **THEN** system returns the count of available trace bytes and immediately sends that exact number of bytes on EP 0x82

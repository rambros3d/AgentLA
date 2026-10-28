## Why

AgentLA currently exposes only passive analysis capabilities (capture + decode) over a single CDC serial interface. Agents can observe bus traffic but cannot actively drive I2C or UART transactions — read sensor registers, write configuration bytes, or talk to serial peripherals. Adding I2C and UART master adapter modes turns AgentLA into a programmable USB-to-bus bridge, enabling agents to interact directly with I2C sensors, EEPROMs, UART devices, and more.

## What Changes

- Add I2C master adapter mode using RP2350 hardware I2C1 peripheral (SDA=GP0, SCL=GP1)
- Add UART adapter mode using RP2350 hardware UART1 peripheral (TX=GP4, RX=GP5)
- Extend agent protocol with I2C_READ, I2C_WRITE, UART_WRITE, UART_READ commands
- Expose adapter capabilities in IDENTIFY response (`ADAPTER I2C+UART`)
- AgentLA enumerates as a USB CDC device with product string `AGENTLA Adapter v1.0` to distinguish from analyzer mode
- Keep SPI adapter mode deferred to a future change (uses SPI1 hardware peripheral)

## Capabilities

### New Capabilities
- `i2c-adapter`: I2C master mode for reading/writing I2C slave devices on the target bus
- `uart-adapter`: UART master mode for TX/RX communication with serial devices at configurable baud rates

### Modified Capabilities
- `agent-protocol`: IDENTIFY command returns adapter role; new I2C_READ/I2C_WRITE/UART_WRITE/UART_READ commands added
- `python-sdk`: SDK gains `I2CDevice` and `UARTDevice` adapter classes with read/write methods

## Impact

- **Firmware**: New I2C1 and UART1 peripheral initialization; adapter-mode command parser; mode-switching logic between analyzer and adapter states
- **Protocol**: New commands `I2C_READ`, `I2C_WRITE`, `UART_WRITE`, `UART_READ`; IDENTIFY returns adapter role
- **SDK**: New `I2CDevice` and `UARTDevice` classes in `sdk/agentla/adapter.py`; updated `connect()` to auto-detect adapter via USB product string
- **Hardware**: GPIO0/1 (I2C), GPIO4/5 (UART) reserved for adapter; existing capture pins (GPIO2-9) remain unchanged
- **Board**: Requires external 4.7kΩ pull-ups on I2C lines (GPIO0/GP1) — currently missing from board design
- **Files Modified**: `firmware/src/main.c`, `firmware/src/agentla_board_settings.h`, `sdk/agentla/adapter.py`, `sdk/agentla/device.py`, `sdk/pyproject.toml`
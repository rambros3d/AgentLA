## 1. Firmware: Hardware Peripheral Initialization

- [x] 1.1 Initialize I2C1 peripheral on GPIO0 (SCL) and GPIO1 (SDA) at 100kHz default
- [x] 1.2 Initialize UART1 peripheral on GPIO4 (TX) and GPIO5 (RX)
- [x] 1.3 Add adapter-mode state management (idle, active transaction)
- [x] 1.4 Add GPIO muxing for I2C1 and UART1 pins in board settings

## 2. Firmware: Adapter Command Protocol

- [x] 2.1 Add I2C_READ command parser (addr, reg, len parameters)
- [x] 2.2 Add I2C_WRITE command parser (addr, reg, data parameters)
- [x] 2.3 Add UART_WRITE command parser (baud, data parameters)
- [x] 2.4 Add UART_READ command parser (baud, len, timeout parameters)
- [x] 2.5 Update IDENTIFY command to include adapter role (`ADAPTER I2C+UART`)
- [x] 2.6 Add I2C bus clear command (I2C_CLEAR toggles SCL 9x)
- [x] 2.7 Add error responses for NACK, bus stuck, invalid baud rates

## 3. Firmware: CDC Serial Configuration

- [x] 3.1 Single CDC with protocol-level device role differentiation (analyzer/adapter distinguished via IDENTIFY response)
- [x] 3.2 Product string set to AGENTLA; role field in IDENTIFY distinguishes device type
- [x] 3.3 I2C1 and UART1 use hardware peripherals, no PIO state machines needed

## 4. SDK: Adapter Device Classes

- [x] 4.1 Create `sdk/agentla/adapter.py` with I2CDevice class
- [x] 4.2 Implement I2CDevice.read(addr, reg, len) method
- [x] 4.3 Implement I2CDevice.write(addr, reg, data) method
- [x] 4.4 Create UARTDevice class in adapter.py
- [x] 4.5 Implement UARTDevice.write(data, baud) method
- [x] 4.6 Implement UARTDevice.read(len, baud, timeout) method

## 5. SDK: Discovery and Connection

- [x] 5.1 Update Device.connect() to detect USB device by VID/PID (auto-detect)
- [x] 5.2 Add `adapter` method to Device class that returns I2CDevice/UARTDevice for adapter port
- [x] 5.3 Update SDK README with adapter mode usage examples

## 6. Testing and Validation

- [ ] 6.1 Build firmware and flash to RP2350
- [ ] 6.2 Verify IDENTIFY returns `ADAPTER I2C+UART` on adapter CDC
- [ ] 6.3 Test I2C_READ with a connected I2C sensor (e.g., temperature sensor)
- [ ] 6.4 Test I2C_WRITE to configure an I2C device register
- [ ] 6.5 Test I2C NACK handling (non-existent device)
- [ ] 6.6 Test UART_WRITE and UART_READ loopback (GPIO4→GPIO5 short)
- [ ] 6.7 Test UART at multiple baud rates (9600, 115200)
- [ ] 6.8 Verify analyzer CDC still works independently (dual-device test)

## 7. Documentation

- [x] 7.1 Document I2C adapter pin connections (GPIO0/GP1, pull-ups required)
- [x] 7.2 Document UART adapter pin connections (GPIO4/GP5)
- [x] 7.3 Add adapter mode examples to SDK README
- [ ] 7.4 Document I2C clock stretch timeout behavior
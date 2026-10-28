## 1. Project Setup and Fork

- [x] 1.1 Fork gusmanb/logicanalyzer repository and clone locally
- [x] 1.2 Strip gusmanb's custom binary protocol and Sigrok streaming code
- [x] 1.3 Set up Pico SDK build system with CMake for RP2350 target
- [x] 1.4 Create project directory structure: firmware/, sdk/, docs/
- [x] 1.5 Verify basic build compiles for RP2350 with pico-sdk

## 2. PIO Capture Engine

- [x] 2.1 Adapt gusmanb's PIO capture program for configurable 8-32 channel input
- [x] 2.2 Implement DMA ping-pong buffering with dual 128 KB SRAM buffers
- [x] 2.3 Add configurable sample rate support (100 kHz to 100 MHz)
- [x] 2.4 Implement edge trigger (RISING/FALLING) on any channel
- [x] 2.5 Implement pattern trigger (bit pattern match across channels)
- [x] 2.6 Add pre/post-trigger buffer configuration with default 50%
- [x] 2.7 Enforce maximum capture time limits based on sample rate and buffer size
- [x] 2.8 Verify capture at 10 MHz with 8 channels produces no gaps

## 3. Text Command Protocol

- [x] 3.1 Implement USB CDC serial initialization with TinyUSB
- [x] 3.2 Implement line-based command parser with \r\n delimiter
- [x] 3.3 Implement IDENTIFY command response
- [x] 3.4 Implement CONFIGURE command with channels, rate, trigger parameters
- [x] 3.5 Implement CAPTURE command with mode=RAW parameter
- [x] 3.6 Implement CAPTURE command with mode=DECODED parameter
- [x] 3.7 Implement MEASURE command for PWM
- [x] 3.8 Implement STATUS command
- [x] 3.9 Add human-readable error messages for all invalid inputs
- [x] 3.10 Add command echo (> COMMAND) before responses

## 4. ARM Protocol Decoders

- [x] 4.1 Implement I2C decoder: START/STOP detection via SDA edges on SCL high
- [x] 4.2 Implement I2C decoder: 7-bit address shift register with R/W bit
- [x] 4.3 Implement I2C decoder: ACK/NACK detection on 9th clock
- [x] 4.4 Implement I2C decoder: Data byte collection with clock stretching support
- [x] 4.5 Implement SPI decoder: CS assertion detection
- [x] 4.6 Implement SPI decoder: Clock edge sampling for MOSI/MISO (CPOL/CPHA modes)
- [x] 4.7 Implement SPI decoder: Multi-byte transfer tracking
- [x] 4.8 Implement UART decoder: Start bit falling edge detection
- [x] 4.9 Implement UART decoder: Baud rate timing and 8N1 frame decode
- [x] 4.10 Implement UART decoder: Parity calculation and framing error detection
- [x] 4.11 Implement UART decoder: Break condition detection
- [x] 4.12 Implement PWM decoder: Edge timestamp capture for period calculation
- [x] 4.13 Implement PWM decoder: Duty cycle measurement
- [x] 4.14 Implement DMA interrupt handler to trigger decode on capture completion
- [x] 4.15 Test decoders with synthetic test data from host

## 5. Dual-Mode Integration

- [x] 5.1 Wire CAPTURE mode=RAW to PIO capture + USB streaming
- [x] 5.2 Wire CAPTURE mode=DECODED to PIO capture + ARM decode + text output
- [x] 5.3 Implement capture status reporting (DONE samples=X time=Y)
- [x] 5.4 Implement decoded event timestamp formatting (0.000123 I2C START)
- [x] 5.5 Implement decode summary output (DONE decoded=N transactions=M)

## 6. Testing and Validation

- [x] 6.1 Create test harness with known signal generator (ESP32 firmware)
- [x] 6.2 Test I2C decode with real I2C device (e.g., EEPROM read)
- [x] 6.3 Test SPI decode with real SPI device
- [x] 6.4 Test UART decode with known baud rate loopback
- [x] 6.5 Test raw capture opens correctly in PulseView
- [x] 6.6 Benchmark maximum capture time at each sample rate
- [x] 6.7 Document Errata E9 workarounds and A4 silicon check procedure

## 7. Documentation and Release

- [x] 7.1 Write hardware setup guide (wiring, level shifting, probe connection)
- [x] 7.2 Write firmware build instructions (CMake, pico-sdk, flash procedure)
- [x] 7.3 Write LLM agent integration guide with example prompts
- [x] 7.4 Create LICENSE file (GPL-3.0)
- [x] 7.5 Tag v1.0 release with firmware binary

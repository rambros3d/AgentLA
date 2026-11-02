## Context

The AgentLA board features a multi-function instrumentation architecture:
- 100 MHz Logic Analyzer engine (RP2350 PIO + DMA).
- 500 kS/s 4-channel Oscilloscope (ADC0–ADC3 on GPIO 26–29 and internal temperature).
- Full-duplex hardware serial adapters (UART0, I2C1, SPI0).
- C/Python Protocol Decoders (UART framing, I2C addressing, SPI data).

To verify these subsystems end-to-end without expensive external test equipment, a second Raspberry Pi Pico 2 board is connected over USB and wired directly to AgentLA's flexible GPIO header. Using the target board as an autonomous signal generator provides deterministic, reproducible waveforms and transactions.

## Goals / Non-Goals

**Goals:**
- Provide a target test firmware for the second Pico 2 that generates known test waveforms across digital and analog pins.
- Test and verify `ADCSCOPE` commands and ADC accuracy across channels 0–3.
- Test and verify `CAPTURE` logic analyzer commands and sample streaming.
- Test and verify UART protocol decoder and full-duplex UART adapter echo.
- Automate all tests via a unified Python test runner (`test/test_autonomous_hardware_verification.py`).

**Non-Goals:**
- SWD flashing of RP2350 (deferred to follow-up change once physical SWD pinout is re-verified).
- Implementing new decoder protocols not already supported in `libdecoders`.
- High-voltage testing (strictly 0V–3.3V RP2350 IO limits).

## Decisions

### Decision 1: Target Firmware Architecture
Use a lightweight Pico SDK C application on the target Pico 2:
- **GPIO 20**: 10 kHz 50% square wave (generated via PWM slice or timer).
- **GPIO 21**: 1 kHz 30% duty cycle PWM.
- **GPIO 26**: 1 kHz PWM with RC / low-pass filter or stepped analog output for ADC scope.
- **GPIO 12 (TX) / 13 (RX)**: 115200 baud UART repeating a structured JSON telemetry line every 100ms, and echoing any received bytes back immediately.
- **GPIO 10 / 11**: I2C slave responder at address `0x3C`.
- *Rationale*: Running autonomous generators on hardware timer/PWM slices ensures jitter-free signals independent of host CPU load.

### Decision 2: Automated Test Harness
Write `test/test_autonomous_hardware_verification.py` using `pyserial`:
- Auto-detects AgentLA on `/dev/ttyACM0` via `IDENTIFY`.
- Executes Tier 1: ADC Oscilloscope check (`ADCSCOPE pin=26 samples=50`).
- Executes Tier 2: Logic Analyzer digital capture (`CAPTURE pin=20 count=500 rate=100000`).
- Executes Tier 3: Protocol Decoder validation on live UART packets.
- Executes Tier 4: Adapter echo loopback validation (`UART_WRITE` / `UART_READ`).

## Risks / Trade-offs

- [Risk] Ground loop or noise between the two USB ports.
  → Mitigation: Direct common ground jumper wired between AgentLA GND and Target GND pins.
- [Risk] I2C floating lines without pull-ups.
  → Mitigation: Target firmware and AgentLA firmware both enable internal weak pull-ups (`gpio_pull_up`) on I2C SDA and SCL pins.

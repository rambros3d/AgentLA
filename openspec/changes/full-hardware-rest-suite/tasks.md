# Tasks: Full Hardware REST Suite

## 1. Phase 1: Profiles, Pins, and Telemetry

- [x] 1.1 Expand `rest_response_buffer` in `firmware/src/rest_api.c` from 1280 B to 1536 B
- [x] 1.2 Implement `GET /api/v1/profiles` and alias `GET /api/profiles` with full 32-slot summary
- [x] 1.3 Implement `POST /api/v1/profiles/apply` and alias `POST /api/profiles/apply`
- [x] 1.4 Implement `POST /api/v1/profiles/boot` and `POST /api/v1/profiles/save`
- [x] 1.5 Expand `GET /api/v1/pins` to serialize the complete 30-pin hardware state table with compact JSON keys
- [x] 1.6 Implement `GET /api/v1/pins/{gpio}`, `POST /api/v1/pins/{gpio}/mode`, and `POST /api/v1/pins/{gpio}/write`
- [x] 1.7 Implement `POST /api/v1/pins/claim` and `POST /api/v1/pins/release`
- [x] 1.8 Implement `GET /api/v1/device/telemetry`, `POST /api/v1/device/power`, `POST /api/v1/device/reboot`, and `POST /api/v1/device/bootsel`
- [x] 1.9 Verify Web UI profile switcher in `firmware/html/apps.html` against simulated and hardware endpoints

## 2. Phase 2: Peripheral Bus Bridges & Addressable LED Driver

- [x] 2.1 Implement `POST /api/v1/bus/i2c/scan` returning responsive 7-bit addresses
- [x] 2.2 Implement `POST /api/v1/bus/i2c/transfer` for atomic write/read transactions
- [x] 2.3 Implement `POST /api/v1/bus/spi/transfer` with mode, clock rate, and full-duplex hex payload support
- [x] 2.4 Implement `POST /api/v1/bus/uart/write` and `GET /api/v1/bus/uart/read`
- [x] 2.5 Implement `POST /api/v1/bus/can/tx` and `GET /api/v1/bus/can/rx`
- [x] 2.6 Create `firmware/pio/neopixel.pio` with 10-cycle bit timing, 1-pin side-set, and 800k/400k clock divider scaling
- [x] 2.7 Implement universal addressable LED driver core with 256-pixel (1 KB) buffer and color swizzling (RGB, GRB, RGBW, GRBW, BGR)
- [x] 2.8 Implement 2-wire SPI framing for APA102 / DotStar / SK9822 with 32-bit start/end frames
- [x] 2.9 Wire `POST /api/v1/bus/led/transmit` in `firmware/src/rest_api.c`

## 3. Phase 3: Core Instruments (Programmer & Analyzer)

- [x] 3.1 Implement `GET /api/v1/programmer/target` with autonomous MCU family/flash identification
- [x] 3.2 Implement `POST /api/v1/programmer/reset` driving nRST and BOOT0 lines
- [x] 3.3 Implement `POST /api/v1/programmer/erase` and `POST /api/v1/programmer/flash`
- [x] 3.4 Implement `GET /api/v1/programmer/read` with chunked zero-RAM generator streaming
- [x] 3.5 Implement `POST /api/v1/analyzer/arm` and `GET /api/v1/analyzer/status`
- [x] 3.6 Implement `POST /api/v1/analyzer/stop` and `GET /api/v1/analyzer/data`
- [x] 3.7 Implement `GET /api/v1/analyzer/vcd` zero-RAM generator streaming directly from `captureBuffer`

## 4. Verification & Integration

- [x] 4.1 Create automated pytest test suite `test/test_hardware_rest_api.py` validating all REST endpoints, including autonomous NeoPixel loopback testing (GPIO 10 TX -> GPIO 20 `decode_led.c`)
- [x] 4.2 Verify memory map and confirm SRAM headroom remains within safe bounds
- [x] 4.3 Update API documentation in `docs/` and `README.md`

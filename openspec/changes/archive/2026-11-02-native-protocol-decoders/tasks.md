## 1. Phase 1: Automotive & Industrial Core

- [x] 1.1 Refactor `firmware/src/analyzer/decode.h` to define the compact union-based `DECODER_EVENT_V2` event arena.
- [x] 1.2 Implement CAN 2.0B / CAN-FD decoder in `firmware/src/analyzer/decode_can.c` with bit-destuffing, nominal/data phase bit timing, and CRC-15/17/21 calculation.
- [x] 1.3 Implement Modbus RTU / RS-485 decoder in `firmware/src/analyzer/decode_modbus.c` with $3.5\times t_{\text{char}}$ framing, standard function codes (0x01–0x10), and CRC-16 verification.
- [x] 1.4 Implement DMX-512 decoder in `firmware/src/analyzer/decode_dmx.c` validating Break ($\ge 88\,\mu\text{s}$), MAB, Start Code, and 512 dimmer channels.

## 2. Phase 2: Universal Addressable LED Engine

- [x] 2.1 Implement single-wire PWM LED decoder in `firmware/src/analyzer/decode_led.c` supporting WS2812B, SK6812 (RGBW), WS2811, WS2813/15, and UCS1903 with pulse timing diagnostics.
- [x] 2.2 Implement two-wire clocked SPI LED decoder supporting APA102 / DotStar, SK9822, WS2801, and LPD8806 with 5-bit Global Brightness extraction.
- [x] 2.3 Add color order mapping (GRB, RGB, BGR, RGBW) and CSS hex string formatting.

## 3. Phase 3: Sensors, Automotive LIN & Next-Gen Buses

- [x] 3.1 Implement 1-Wire Dallas decoder in `firmware/src/analyzer/decode_1wire.c` measuring Reset/Presence pulses and decoding 64-bit ROM registration IDs with DOW CRC.
- [x] 3.2 Add automatic DS18B20 temperature extraction in Celsius from Read Scratchpad (`0xBE`) responses.
- [x] 3.3 Implement LIN Bus decoder in `firmware/src/analyzer/decode_lin.c` with Sync Break, Sync `0x55`, Protected ID, and classic/enhanced checksums.
- [x] 3.4 Implement MIPI I3C SDR decoder in `firmware/src/analyzer/decode_i3c.c` with Broadcast Header `0x7E`, Common Command Codes (CCC: ENTDAA, SETDASA, etc.), and transition `T` parity.

## 4. Phase 4: Embedded Silicon Debug & Trace Sniffing

- [x] 4.1 Implement ARM SWD protocol sniffer in `firmware/src/analyzer/decode_swd.c` decoding SWCLK/SWDIO DP/AP register transactions, ACK, and parity.
- [x] 4.2 Implement ARM SWO ITM trace decoder in `firmware/src/analyzer/decode_swo.c` extracting Stimulus Port 0 ASCII printf text.
- [x] 4.3 Implement SPI Flash command interpreter in `firmware/src/analyzer/decode_spiflash.c` translating raw SPI bytes into high-level NOR Flash commands (JEDEC ID, Read, Write, Erase).

## 5. Phase 5: WebUI Visual Overlays, CLI & Test Suite

- [x] 5.1 Extend CLI grammar in `firmware/src/main.c` and `webui_command_bridge.c` for `CAPTURE mode=DECODED protocol=<CAN|MODBUS|DMX|LED_1W|LED_2W|1WIRE|I3C|SWD>`.
- [x] 5.2 Implement interactive Canvas protocol overlays in `firmware/html/app.js` and `web/demo/app.js` with color-coded bubbles and CSS RGB swatches for addressable LEDs.
- [x] 5.3 Connect decoded events to `/events` SSE stream for real-time live terminal logging in the browser.
- [x] 5.4 Add Python test suites in `test/test_protocol_decoders.py` verifying decoding accuracy and error detection against simulated waveforms.

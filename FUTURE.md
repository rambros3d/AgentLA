# AgentLA Future Roadmap & Planned Features

This document tracks upcoming milestones for the **AgentLA Hardware & Firmware System**.
> **Rule**: Once a feature is implemented and verified, REMOVE IT FROM THIS DOCUMENT.

---

## 1. Target Debugging & Flashing Expansions

- [ ] **Raspberry Pi RP2040 & RP2350 Target Bootrom Flashing**
  - **Scope**: SWD flash programming and rescue debugging for RP2040 and RP2350 targets using Raspberry Pi bootrom vectors and flash memory algorithms.

- [ ] **Espressif ESP32 Series Bootloader Support (ESP32, ESP32-C3, ESP32-C6)**
  - **Scope**: ESP-IDF serial bootloader protocol implementation over target UART pins (`/api/v1/bus/uart`).
  - **Capabilities**: ROM sync (`0x08 0x00`), stub loader injection, SPI flash write, and chip ID detection.

---

## 2. Factory Test & Bench Automation (FCT Station)

- [ ] **Autonomous Factory Test Station (`autonomous-factory-test-station`)**
  - **Scope**: Production programming and functional verification fixture:
    1. Continuous $V_{\text{target}}$ contact sensing with 150 ms debounce.
    2. Sub-millisecond short-circuit and overcurrent trip protection via onboard TI INA180 current sense.
    3. Premature unclamp safe High-Z pin release.
    4. Declarative `factory_recipe.json` test runner with dynamic serial number / MAC address patching.
    5. Automated UART boot banner verification, TCAN334 CAN-FD echo, and RS-485 ping.
    6. Full-screen production WebUI with 2-meter readable PASS/FAIL status cards and yield counters.
  - **Reference**: `openspec/changes/autonomous-factory-test-station/`.

---

## 3. BakuProbe Hardware Bring-Up (Custom PCB)

- [ ] **BakuProbe RP2350B QFN-80 Board Bring-Up**
  - **Scope**: Hardware driver activation for reference BakuProbe PCB:
    - Pod 4 & 5: 8-channel bidirectional logic analyzer + 8-channel MSO oscilloscope frontend with CH444G 4:1 analog attenuators.
    - 32-bit shift register bus (`SPI0`) for transceiver OE and attenuator gain switching.
    - Per-pin chained SK6805-EC15 / WS2812B micro-RGB LEDs reflecting real-time pin roles.
    - Front-panel tactile emergency stop trip button (`EMERGENCY_BTN`).

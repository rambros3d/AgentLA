## 1. Firmware Contact Sensing & Safety Core

- [ ] 1.1 Implement continuous VTARGET voltage polling and 150ms debounce state machine in firmware.
- [ ] 1.2 Implement sub-millisecond overcurrent protection interrupt using TI INA180 current sense (GP46/ADC6) to trip target power upon detecting a short.
- [ ] 1.3 Implement premature unclamp detection and safe High-Z pin release to protect probe and target during unexpected board removal.

## 2. Declarative Recipe Engine & Serialization

- [ ] 2.1 Define JSON schema for `factory_recipe.json` supporting target MCU definitions, pre-flight safety limits, serialization rules, and FCT test steps.
- [ ] 2.2 Implement dynamic serialization injector to patch unique serial numbers, MAC addresses, and manufacturing metadata into Flash/OTP sectors before programming.

## 3. Automated Functional Circuit Testing (FCT)

- [ ] 3.1 Implement automated UART boot banner sniffer asserting expected target startup string within timeout.
- [ ] 3.2 Implement automated CAN-FD transceiver echo test using BakuProbe's onboard TCAN334.
- [ ] 3.3 Implement automated RS-485 transceiver ping test using BakuProbe's onboard SN65HVD3082.
- [ ] 3.4 Implement active run and sleep current draw verification using the INA180 telemetry pipeline.

## 4. Production Station WebUI Surface

- [ ] 4.1 Build high-contrast, full-screen Production Station view with prominent PASS / FAIL status cards readable from 2 meters away.
- [ ] 4.2 Add audio alert feedback using the Web Audio API (success chime on PASS, error buzzer on FAIL).
- [ ] 4.3 Implement live production batch counters (Total, Passed, Failed, Yield Percentage, Average Cycle Time) with CSV/JSON export.

## 5. Headless Automation Runner & Verification

- [ ] 5.1 Develop headless Python runner CLI (`tools/production_runner.py`) communicating over USB RNDIS HTTP / JSON-RPC for factory MES integration.
- [ ] 5.2 Add automated tests simulating fixture contact, short circuit tripping, and end-to-end flash/FCT pass cycle.

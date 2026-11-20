## Why

The BareFOC firmware has evolved to support Silicon Hardware UIDs, Board Identity (Node ID, Board Name), 4x Joint Identity slots (ADDR 0..127, Name), and a 4-Axis non-volatile Motor Bank with safe disarmed boot and atomic runtime `ALIGN` auto-commit. The built-in BareFOC Studio web application in AgentLA (`webui/barefoc.html`) needs to reflect these identity registers, display joint names on motor tabs, update MultiBus register mappings, and enhance the Automated Calibration Wizard to provide full commissioning parity for the "Flash Once and Forget" architecture.

## What Changes

- Add Board Identity display (Hardware UID, Board Name, Node ID) to the BareFOC Studio header status strip in `webui/barefoc.html`.
- Add Joint Identity display and editable fields (Joint ADDR 0..127 and Joint Name) per active motor tab (M0..M3).
- Synchronize `MultiBusReg` constants with the updated ControlTable layout:
  - Identity registers: `HARDWARE_UID` (0x04), `BOARD_NODE_ID` (0x10), `BOARD_NAME` (0x11), `JOINT0_ADDR` (0x20)..`JOINT3_NAME` (0x51).
  - 4-Axis Motor Bank: `0x60..0xDF` (32 bytes per motor).
  - Calibration Command: `CALIB_CMD` (`0xB0` codes: 0x01 ALIGN, 0x02 SWEEP, 0x03 SAVE, 0x04 SET_ORIGIN).
- Update the Automated Calibration Wizard UI strip:
  - Display safe disarm status badge (`DISARMED_SAFE` vs `ARMED`).
  - Read out measured electrical zero angle and sensor direction.
  - Provide 1-click `Run Electrical ALIGN` with auto-commit status.
  - Provide `Set Mechanical Origin` (ZERO) button.
- Strictly adhere to AgentLA UI rules: DaisyUI 5 industrial styling, zero dot indicators, full-bleed viewport, and zero emojis.

## Capabilities

### Modified Capabilities
- `barefoc-studio-application`: Update identity headers, multi-axis motor register maps, and calibration wizard controls.

## Impact

- `AgentLA-rust/webui/barefoc.html`: Frontend UI layout, register definitions, and event handlers.
- End-to-end integration between AgentLA and BareFOC multi-axis nodes.

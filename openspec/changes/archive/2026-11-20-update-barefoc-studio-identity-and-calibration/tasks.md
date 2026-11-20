# Tasks: BareFOC Studio Identity & Calibration Wizard Update

## Phase 1: Identity & Header Integration
- [x] 1.1 Add Board Identity components (Hardware UID pill, Board Name input, Node ID input) to header strip in `webui/barefoc.html`.
- [x] 1.2 Update motor selector tabs (M0..M3) to display dynamic joint names.
- [x] 1.3 Add Joint Identity configuration controls (Joint Address 0..127 and Joint Name) in `webui/barefoc.html`.

## Phase 2: Register Map & Calibration Wizard
- [x] 2.1 Update `MultiBusReg` definitions with Identity offsets, 4-Axis Motor Bank (`0x60..0xDF`), and updated `CALIB_CMD` codes.
- [x] 2.2 Enhance the Automated Calibration Wizard strip:
  - Add safe disarm badge (`DISARMED_SAFE` vs `ARMED`).
  - Add electrical zero angle and direction readout.
  - Wire 1-click `Run Electrical ALIGN` (writes `0x01` to `0xB0`) with auto-commit status.
  - Wire `Set Mechanical Origin` (writes `0x04` to `0xB0`).

## Phase 3: Verification & Styling Audit
- [x] 3.1 Audit UI to ensure strict compliance with DaisyUI 5, zero CSS dot indicators (`.active::after`), full-bleed viewport, and zero emojis.
- [x] 3.2 Verify syntax and frontend tests (e.g. `pytest test/test_barefoc_runtime.py`).

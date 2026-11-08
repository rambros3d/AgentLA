## 1. Firmware Profile Storage & Flash Allocation

- [x] 1.1 Define `profile_slot_t` memory layout, CRC32 verification, and 64 KB flash sector offset (`PICO_FLASH_SIZE_BYTES - 64KB`) in `firmware/src/profile_manager.h`.
- [x] 1.2 Implement flash read, write, and erase routines in `firmware/src/profile_manager.c` supporting 32 slots with Slot 0 reserved for permanent Safe State.
- [x] 1.3 Implement boot scanner to auto-apply designated boot default profile if flagged, or strictly maintain 100% High-Z safe state.

## 2. Pin Ownership Safe Handover & Tri-State Abort

- [x] 2.1 Implement `pin_owner_quiesce_and_reset()` in `firmware/src/pin_owner.c` to atomically revoke claims and set all pins to High-Z inputs with pulls disabled.
- [x] 2.2 Implement `profile_apply_safe(slot_id)` in `firmware/src/profile_manager.c` executing safe instrument quiescence, 5ms power discharge, and atomic claim application.
- [x] 2.3 Implement immediate `profile_emergency_safe_state()` handler to cut VTARGET within 1ms and tri-state all physical GPIOs.

## 3. REST API & Command Bridge

- [x] 3.1 Register `/api/profiles` REST routes in `firmware/src/webui_command_bridge.c` and `network_stack.c` (`GET /api/profiles`, `POST /api/profiles`, `POST /apply`, `POST /boot`, `POST /safe_state`, `DELETE`).
- [x] 3.2 Wire JSON serialization and deserialization for workbench state payloads into `profile_manager.c`.

## 4. WebUI Header & Emergency Controls

- [x] 4.1 Add prominent `[ SAFE STATE ]` button and top-bar profile selector dropdown in `firmware/html/index.html` and `style.css`.
- [x] 4.2 Implement global `Shift+Esc` emergency hotkey in `firmware/html/app.js` triggering `/api/profiles/safe_state`.
- [x] 4.3 Implement dynamic profile dropdown population and active status badge in `firmware/html/app.js`.

## 5. Profile Management Modal & Cross-Board Logic

- [x] 5.1 Create Save Profile modal dialog allowing users to enter profile name, select target slot, and toggle boot default.
- [x] 5.2 Implement cross-board compatibility validator in `app.js` that applies compatible pins/instruments and displays an informative alert listing skipped hardware.

## 6. Verification & Automated Testing

- [x] 6.1 Add Python integration test in `test/test_hardware_profiles.py` validating slot CRC, REST endpoints, and safe state transitions.
- [x] 6.2 Test firmware build against Pico 2 and BakuProbe targets verifying flash boundary alignment and zero warnings.

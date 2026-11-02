## Why

Currently, reconfiguring AgentLA's instruments, pin assignments, baud rates, and voltage rails for different projects requires repetitive manual configuration every time the probe is connected. Furthermore, connecting an active probe to delicate target hardware without strict electrical safeguards risks pin contention, short circuits, or overvoltage if outputs remain active across reboots.

This change introduces hardware-resident Profiles and an Emergency Safe State system. Up to 32 full-state profiles are stored directly in the RP2350's non-volatile flash memory, enabling configurations to travel with the hardware across any workstation. On power-up and reset, the probe strictly defaults to a 100% High-Z Safe State with 0V target power (with optional boot-profile designation), complemented by an instant one-click Emergency Safe State header button and `Shift+Esc` hotkey.

## What Changes

- **32 On-Device Flash Profile Slots**: Allocates a dedicated 64 KB sector block (`FLASH_SIZE - 64KB`) in RP2350 flash storing up to 32 named profiles with Slot 0 reserved for factory Safe State.
- **Strict Safe State Boot Contract**: Firmware boots into High-Z inputs with target power disabled by default, supporting an optional user-designated boot default profile.
- **Safe Handover Engine**: Transits through a 5ms intermediate tri-state and power discharge window before applying a new profile to prevent drive contention.
- **Emergency Safe State Control**: Prominent `[ 🛡️ SAFE STATE ]` header button and `Shift+Esc` hotkey for instant hardware tri-state and target power shutoff.
- **Cross-Board Graceful Partial Loading**: Enables profiles created on BakuProbe (48 GPIOs, CAN-FD, RS-485, INA180) to load cleanly on Pico 2 (30 GPIOs), applying all physically present pins/instruments and flagging skipped hardware in a compatibility dialog.
- **Full Workbench State Snapshot**: Captures pin matrix assignments, logic analyzer sample rates/triggers, oscilloscope scales, bus adapter baud rates, power rails, and active protocol decoders.
- **REST Endpoints**: `/api/profiles`, `/api/profiles/{id}`, `/api/profiles/{id}/apply`, `/api/profiles/{id}/boot`, and `/api/profiles/safe_state`.

## Capabilities

### New Capabilities
- `hardware-profile-manager`: 32-slot non-volatile RP2350 flash profile storage, REST API, serialization, and atomic profile activation.
- `emergency-safe-state`: Hardware-enforced safe state (all GPIOs Hi-Z inputs, VTARGET 0V, transceivers isolated), default boot guarantee, header panic button, and `Shift+Esc` hotkey.

### Modified Capabilities
- `webui-control-surface`: Adds top navigation profile switcher dropdown, emergency safe state button, save/load profile modal, and cross-board compatibility warning dialog.
- `pin-ownership`: Adds safe atomic release and handover sequences to tri-state existing claims before applying new profile configurations.

## Impact

- **Firmware Flash & Memory**: Reserves 64 KB at top of flash (`PICO_FLASH_SIZE_BYTES - 64KB`); adds `firmware/src/profile_manager.c` and header.
- **WebUI Client**: Modifies `firmware/html/index.html`, `style.css`, and `app.js` to render the profile dropdown, emergency safe state button, and compatibility warnings.
- **REST & Command Bridge**: Extends `firmware/src/webui_command_bridge.c` and `network_stack.c` with `/api/profiles` endpoints.

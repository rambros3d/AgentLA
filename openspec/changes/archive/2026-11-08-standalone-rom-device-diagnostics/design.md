## Context

AgentLA decouples application assets from firmware ROM by serving the primary user interface (`/home`, `/scope`, etc.) dynamically from LittleFS `SYSTEM_FS`. However, if the flash filesystem is blank, unformatted, corrupted during an interrupted update, or failing to mount, the device becomes inaccessible over HTTP without an external SWD debugprobe.

To provide an infallible hardware recovery path and bench-side diagnostics cockpit, `device.html` will be embedded directly into Flash ROM within the firmware image. It is served with zero LittleFS filesystem dependency, self-contained inline styling and scripts, and direct low-level REST endpoints for hardware inspection, separate firmware/WebUI updating, and developer recovery operations.

## Goals / Non-Goals

**Goals:**
- Provide a zero-dependency ROM-embedded HTML interface (`device.html`) served on `/device` and `/device.html` that functions under all filesystem conditions.
- Implement live hardware telemetry rendering: MCU chip, core clock, SRAM, LittleFS partitions, supply voltages, currents, temperatures, and 30-GPIO pin ownership states.
- Implement a browser-mediated update checker querying GitHub Releases to offer separate Auto Update buttons for Firmware (`.bin`) and WebUI (`.tar`) alongside local file upload dropzones.
- Provide always-visible Developer Mode cards for hands-free BOOTSEL reboot (`reset_to_usb_boot`), watchdog reset, A/B slot rollback/commit, pin arbiter emergency releases, pin continuity probing, and partition formatting.
- Maintain a tiny footprint (< 6 KB raw HTML/CSS/JS) within the `#![no_std]` firmware crate.

**Non-Goals:**
- Implementing on-device outbound HTTPS clients or TLS stack (update checking is delegated to client browser).
- Replacing the full-featured WebUI workbench (`/scope`) or DaisyUI launcher (`/home`); `/device` is explicitly a lightweight diagnostic and rescue interface.

## Decisions

### 1. In-ROM Embedding (`include_str!`) vs LittleFS Asset
- **Decision**: Store `device.html` in `firmware-rs/src/device_html.rs` as a compile-time static string (`include_str!("../assets/device.html")` or raw string literal) and serve via picoserve `File::html`.
- **Rationale**: Guarantees availability even if flash partitions are formatted, unmounted, or erased.
- **Alternatives Considered**: Storing in LittleFS (fails recovery goal) or embedding full DaisyUI (exceeds ROM budget).

### 2. Browser-Mediated GitHub Update Checker
- **Decision**: Execute the GitHub Releases API request in client JavaScript running in the user's browser, comparing versions against device REST endpoints.
- **Rationale**: The RP2350 target only runs a local USB CDC-NCM network (`10.10.10.10`) without internet gateway or TLS certificates. The client browser has internet connectivity and can download binaries and POST them directly to the local device.
- **Alternatives Considered**: In-firmware HTTP client (requires DNS, default gateway routing, TLS stack, and excessive RAM).

### 3. Dedicated Low-Level REST Handlers
- **Decision**: Add minimal, dedicated `#![no_std]` endpoints in `firmware-rs/src/rest.rs` for developer operations:
  - `POST /api/v1/system/bootsel`: calls `embassy_rp::rom_data::reset_to_usb_boot(0, 0)`.
  - `POST /api/v1/system/reboot`: calls `embassy_rp::watchdog::Watchdog::trigger_reset()`.
  - `POST /api/v1/system/pins/release_all`: calls `pin_owner::emergency_release_all()`.
  - `POST /api/v1/system/pins/toggle`: acquires pin and sets drive state.
  - `POST /api/v1/system/fs/format`: safely formats partition.
- **Rationale**: Keeps developer operations explicit (POST method required), self-contained, and safe from inadvertent GET triggers.

## Risks / Trade-offs

- **Risk**: Calling `reset_to_usb_boot` drops the USB network connection immediately.
  - **Mitigation**: The UI warns the user before executing and sends an HTTP 200 response with a 500ms delay before triggering the ROM function.
- **Risk**: Forcibly releasing pins could disrupt active hardware peripherals.
  - **Mitigation**: Protected by UI confirmation dialog and labeled as an emergency override.
- **Risk**: Format commands erase user data or WebUI assets.
  - **Mitigation**: Endpoints require explicit JSON payload (`{"confirm": true}`) and modal confirmation in the UI.

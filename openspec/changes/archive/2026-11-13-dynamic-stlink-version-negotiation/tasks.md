## 1. Version Parsing & StLinkHandler State

- [x] 1.1 Implement `parse_version(s: &str) -> Option<u16>` and `format_version(v: u16, buf: &mut [u8]) -> &str` in `crates/agentla-programmer/src/stlink.rs`.
- [x] 1.2 Add `reported_version: u16` state and setter/getter methods to `StLinkHandler`.
- [x] 1.3 Update `StLinkHandler` command `0xF1` handler to dynamically output `self.reported_version`.
- [x] 1.4 Add unit tests in `crates/agentla-programmer` covering version parsing, formatting, and `F1` response verification.

## 2. LittleFS Persistence & REST Endpoints

- [x] 2.1 Implement `load_stlink_version()` and `save_stlink_version()` in `firmware-rs` reading/writing `/system/stlink_version.txt`.
- [x] 2.2 Add `GET /api/programmer/stlink/version` and `POST /api/programmer/stlink/version` in `firmware-rs/src/rest.rs` and route in `web.rs`.
- [x] 2.3 Initialize `StLinkHandler` with the persisted version during system startup.

## 3. WebUI Controls

- [x] 3.1 Add ST-Link Version selector (with presets `V2J46M27`, `V2J48M35`, `V2J49M36`, and custom input) in `webui/index.html`.
- [x] 3.2 Add JavaScript handlers in `webui/app.js` to query and save the configured version.

## 4. Verification & Testing

- [x] 4.1 Run unit tests with `cargo test --target x86_64-unknown-linux-gnu -p agentla-programmer`.
- [x] 4.2 Verify embedded build with `cargo check --target thumbv8m.main-none-eabihf`.

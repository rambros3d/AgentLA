## 1. Diagnostics WebUI Refactoring (device.html)

- [x] 1.1 Add the new 5th tab `[ STORAGE ]` (`#tabStorage`) and view panel (`#viewStorage`) immediately next to `[ HARDWARE ]`
- [x] 1.2 Move `SYSTEM_FS`, `USER_FS` cards and the full-width Flash Partitions bar and geometry table from `[ HARDWARE ]` into `[ STORAGE ]`
- [x] 1.3 Implement the full-width System Update Hero Card (`#cardSystemUpdate`) at the top of the `[ HARDWARE ]` panel
- [x] 1.4 Completely remove `#recoveryBanner` and all redundant `#rec*` elements from `device.html`
- [x] 1.5 Implement the 2-phase client update runner (`runFullSystemUpdate()`) streaming `webui.tar` first, then `agentla-pico2-ota.bin`, and auto-rebooting
- [x] 1.6 Update the GitHub release query URL in `device.html` to `api.github.com/repos/rambros3d/AgentLA/releases/latest`

## 2. Firmware HTTP Router & Redirection (web.rs & fs_http.rs)

- [x] 2.1 Register route aliases for `/device/storage` and `/device/storage/` in `firmware-rs/src/web.rs`
- [x] 2.2 Update fallback redirect in `firmware-rs/src/fs_http.rs` to redirect missing HTML page requests directly to `/device`

## 3. Multi-Segment All-in-One UF2 Packaging Tool

- [x] 3.1 Create `tools/package_uf2.py` capable of bundling `bootloader.bin` (`0x10000000`), `firmware.bin` (`0x10004000`), and `webui.img` (`0x10204000`) into a single `agentla-pico2.uf2`

## 4. GitHub Actions Release Workflow (.github/workflows/release.yml)

- [x] 4.1 Create `.github/workflows/release.yml` with triggers on tag push `v*` and manual `workflow_dispatch`
- [x] 4.2 Configure runner with Rust `thumbv8m.main-none-eabihf`, `gcc-arm-none-eabi`, `elf2uf2-rs`, and `littlefs-python`
- [x] 4.3 Configure build pipeline producing `agentla-pico2.uf2`, `agentla-pico2-ota.bin`, and `webui.tar`
- [x] 4.4 Configure GitHub Release creation publishing the generated artifacts to `rambros3d/AgentLA`

## 5. Verification & Tests

- [x] 5.1 Update `test/test_device_diagnostics.py` to assert the 5-tab structure (`tabStorage`, `viewStorage`), `#cardSystemUpdate`, and removal of `#recoveryBanner`
- [x] 5.2 Execute test suite and verify `tools/pack_webui.py`

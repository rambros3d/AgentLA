## 1. DaisyUI Standalone Distribution & Firmware Route

- [x] 1.1 Fetch official standalone `daisyui.css` (v5.7.x) into `webui/daisyui.css` and verify size and gzip ratio
- [x] 1.2 Register route `/daisyui.css` using `SystemWebFile` with MIME type `text/css` in `firmware-rs/src/web.rs`

## 2. Layout Utility Layer in `webui/style.css`

- [x] 2.1 Add CSS Grid utilities (`.grid`, `.grid-cols-1` through `.grid-cols-12`, `.col-span-*`, and responsive breakpoint variants `md:`, `lg:`, `xl:`)
- [x] 2.2 Add Flexbox utilities (`.flex`, `.inline-flex`, `.flex-col`, `.flex-row`, `.flex-wrap`, `.flex-1`, `.grow`, `.shrink-0`, `.items-*`, `.justify-*`)
- [x] 2.3 Add Spacing, Gap, Sizing, and Theme Opacity utilities (`.gap-*`, `.p-*`, `.px-*`, `.py-*`, `.m-*`, `.mx-*`, `.my-*`, `.w-*`, `.h-*`, `.max-w-*`, `.min-h-*`, color mixes)

## 3. User App Modernization & Package Regeneration

- [x] 3.1 Update `apps/servo-master/index.html` to link `/daisyui.css` before `/style.css` and verify layout rendering
- [x] 3.2 Update `apps/can-sniffer/index.html` to link `/daisyui.css` before `/style.css` and verify layout rendering
- [x] 3.3 Re-package user application tarballs into `apps/packages/servo-master.tar` and `apps/packages/can-sniffer.tar`

## 4. Packaging, Headroom Verification & Test Suite

- [x] 4.1 Run `python3 tools/pack_webui.py` to build `build/webui.img`, `build/webui.uf2`, and `build/webui.tar`, verifying >=35% partition headroom
- [x] 4.2 Update automated test assertions in `test/test_webui_assets.py` and `test/test_multi_app_store.py`
- [x] 4.3 Run `cargo check --manifest-path firmware-rs/Cargo.toml` and targeted pytest suite to verify zero regressions

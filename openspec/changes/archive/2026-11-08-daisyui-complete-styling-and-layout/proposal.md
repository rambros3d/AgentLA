## Why

User-installed apps (such as `servo-master` and `can-sniffer`) and custom bench instruments in AgentLA suffer from degraded, collapsed, and inconsistent visual styling because the system WebUI currently lacks the complete standalone daisyUI component distribution and the foundational Tailwind CSS atomic layout utility layer (`grid-cols-*`, `flex`, `gap-*`, spacing, and sizing). While daisyUI provides semantic components (`navbar`, `card`, `join`, `badge`, `table`, `drawer`, `dock`), it intentionally relies on Tailwind layout utilities to arrange these components into responsive grids. Serving the official standalone daisyUI bundle alongside a comprehensive layout utility layer directly from the 512 KB LittleFS system partition resolves this gap while maintaining over 40% partition headroom.

## What Changes

- Add official standalone `daisyui.css` (v5.7.x) to the WebUI distribution, served pre-gzipped (`Content-Encoding: gzip`) at `/daisyui.css` with 1-year browser cache headers (`Cache-Control: public, max-age=31536000`).
- Register `/daisyui.css` in `firmware-rs/src/web.rs` via `SystemWebFile` with MIME type `text/css`.
- Expand `webui/style.css` with a comprehensive, zero-runtime utility layer covering Grid (`grid-cols-1` through `grid-cols-12`, `col-span-*`, responsive `md:`, `lg:`, `xl:` variants), Flexbox (`flex`, `flex-col`, `flex-wrap`, `flex-1`, `items-*`, `justify-*`), Gap (`gap-0` through `gap-8`, `gap-x-*`, `gap-y-*`), Spacing (`p-*`, `px-*`, `py-*`, `m-*`, `mx-*`, `my-*`, `mx-auto`), Sizing (`w-*`, `h-*`, `min-h-*`, `max-w-*`), Typography, Borders, and color opacity modifiers (`bg-base-200/80`, `text-base-content/60`).
- Update user apps (`apps/servo-master/index.html` and `apps/can-sniffer/index.html`) to link `/daisyui.css` before `/style.css`, adopting daisyUI component semantics (`card`, `join`, `range`, `table`, `badge`) and inheriting system theme tokens (`oklch` forest/emerald palettes).
- Update system WebUI packager (`tools/pack_webui.py`) and verification tests (`test/test_webui_assets.py`, `test/test_multi_app_store.py`) to enforce daisyui.css inclusion and flash budget limits (< 512 KB).

## Capabilities

### New Capabilities
- `daisyui-standalone-distribution`: System WebUI distribution of pre-compressed daisyUI v5 bundle with HTTP gzip streaming, persistent browser caching, and system flash partition budget enforcement.

### Modified Capabilities
- `daisyui-bento-theme-system`: WebUI styling is updated to require full daisyUI v5 component support coupled with a unified atomic layout utility layer for responsive grid and flexbox arrangements.
- `servo-master-application`: The servo master app layout is updated to link `/daisyui.css` and use native daisyUI component styling and responsive grid layouts.

## Impact

- **WebUI Assets**: `webui/daisyui.css` added (~1.1 MB raw, ~98 KB gzip), `webui/style.css` expanded with utility classes.
- **Flash Storage**: Total gzipped WebUI footprint increases from ~215 KB to ~315 KB, remaining safely within the 512 KB LittleFS system partition (~200 KB free headroom).
- **Firmware Routing**: Route `/daisyui.css` registered in `firmware-rs/src/web.rs`.
- **User Applications**: `apps/servo-master` and `apps/can-sniffer` updated and re-packaged in `apps/packages/*.tar`.
- **Test Suite**: Web asset test assertions updated in `test/test_webui_assets.py` and `test/test_multi_app_store.py`.

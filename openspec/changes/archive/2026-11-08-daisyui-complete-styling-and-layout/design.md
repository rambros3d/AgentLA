## Context

AgentLA delivers an on-chip web workbench and app environment directly from the RP2350 microcontroller using an Embassy async TCP/HTTP web stack. The static WebUI assets are stored inside Partition 1 (`SYSTEM_FS`), a 512 KB LittleFS flash partition starting at address `0x10190000`.

User-installed apps (e.g., `servo-master`, `can-sniffer`) load in an iframe or standalone view and link the core stylesheets (`/style.css`, `/themes.css`). However, daisyUI 5.x separates component skins from atomic layout primitives: daisyUI provides components (`navbar`, `card`, `join`, `table`, `range`, `modal`, `drawer`, `dock`), but relies strictly on Tailwind CSS for layout utilities (`grid-cols-*`, `flex`, `gap-*`, spacing, and sizing).

Previously, `webui/style.css` only contained a small number of hand-copied classes and zero multi-column grid or gap layout utilities. Without these utilities, user app layouts collapsed into default vertical HTML block flows. Furthermore, user apps could not access the full 50+ component set of daisyUI.

By bundling the official standalone `daisyui.css` (v5.7.x) and appending a comprehensive, zero-runtime Tailwind-compatible utility layer directly into `webui/style.css`, user apps gain complete, responsive, and theme-harmonized layouts while keeping flash usage safely within the 512 KB budget.

## Goals / Non-Goals

**Goals:**
- Provide the official standalone `daisyui.css` bundle on the RP2350 web server at `/daisyui.css`.
- Serve `daisyui.css` pre-gzipped (`Content-Encoding: gzip`) with immutable caching headers (`max-age=31536000`).
- Provide a comprehensive utility layer in `webui/style.css` matching Tailwind CSS syntax for Grid (`grid-cols-1`..`grid-cols-12`, responsive `md:`, `lg:`, `xl:`), Flexbox (`flex`, `flex-col`, `flex-wrap`, `flex-1`, `items-*`, `justify-*`), Gap (`gap-0`..`gap-8`), Spacing (`p-*`, `m-*`), Sizing (`w-*`, `h-*`), and Opacity color-mixes.
- Update `apps/servo-master` and `apps/can-sniffer` to link `/daisyui.css` and use native daisyUI components and grid layouts.
- Guarantee that all 22+ WebUI assets compress and fit into the 512 KB LittleFS system partition with at least 35% free headroom.

**Non-Goals:**
- Running a live Node.js / JIT Tailwind compiler on the RP2350 microcontroller. All CSS must be static and pre-compiled.
- Modifying user LittleFS storage partition (`USER_FS`, Partition 2 at `0x10210000`, 1.93 MB) or flash firmware slots.

## Decisions

### Decision 1: Option A - Unified Utility Layer in `style.css` alongside `daisyui.css`
* **Choice**: Keep `webui/style.css` as the master utility and bench styling sheet, while adding `webui/daisyui.css` as the dedicated standalone component bundle.
* **Rationale**: This gives user apps and system workbench views clean separation: `daisyui.css` provides all component definitions (`.btn`, `.card`, `.badge`, `.table`, `.modal`, `.range`, `.join`), while `style.css` provides bench tokens and layout utilities (`.grid`, `.grid-cols-4`, `.gap-4`, `.flex-col`, `.p-4`). Applications only need two `<link>` tags:
  ```html
  <link rel="stylesheet" href="/daisyui.css">
  <link rel="stylesheet" href="/style.css">
  ```
* **Alternatives Considered**: Creating a third file `utilities.css`. Rejected because an extra HTTP request adds round-trip overhead on microcontrollers without significant maintenance benefits.

### Decision 2: Pre-Gzip LittleFS Packaging and HTTP Streaming
* **Choice**: `tools/pack_webui.py` compresses `daisyui.css` (1.14 MB uncompressed) into ~98 KB gzip at build time and stores it in LittleFS. `firmware-rs/src/fs_http.rs` serves `.css` with `Content-Encoding: gzip` directly from flash without decompressing in microcontroller RAM.
* **Rationale**: The RP2350 has only 520 KB of SRAM. Pre-compressing at host build time allows streaming straight from flash DMA to the network socket, saving SRAM while reducing HTTP transfer size by 91%.
* **Alternatives Considered**: Truncating daisyUI components. Unnecessary because the entire compressed WebUI fits in ~315 KB out of 512 KB (leaving >200 KB free headroom).

## Risks / Trade-offs

- **[Risk] Partition Overrun**: Adding a 1.14 MB CSS file could exceed the 512 KB LittleFS partition if not gzipped properly.
  → **Mitigation**: `tools/pack_webui.py` enforces gzip compression on all `.css` files and throws a build-time error if total image size exceeds 524,288 bytes. Automated tests verify partition headroom > 30%.
- **[Risk] Selector Specificity Clashes**: daisyUI classes might conflict with custom bench CSS in `style.css`.
  → **Mitigation**: HTML templates load `daisyui.css` first, followed by `style.css`, ensuring user-app overrides and bench utility classes take natural cascade precedence.

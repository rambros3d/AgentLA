## Context

AgentLA currently boots with a consumer-oriented squircle "Bento" layout styled after Apple macOS/iOS widget boards. While visually pleasant, hardware engineers, embedded developers, and lab technicians working on circuit validation, logic analysis, and target programming frequently favor a high-precision, tactical, mechanical design language.

The "Nothing OS" aesthetic—pioneered by Nothing and Teenage Engineering—combines OLED deep blacks, stark white typography, an iconic Signal Red accent (`#D71921`), dot-matrix display typography, and 1px technical outlines.

This design introduces a **dual-persona design engine** where **Industrial is the default persona**, while **Bento remains fully supported and switchable** in System Settings.

## Goals / Non-Goals

**Goals:**
- **Out-of-the-box Industrial Default**: Ship `data-style="industrial"` and the `nothing` DaisyUI theme as the default presentation across all web desktop pages.
- **Dual-Persona Architecture**: Provide an instantaneous persona switcher in `#modal-settings` allowing users to choose between `Industrial (Default)` and `Bento`.
- **Circular Dot-Matrix Typography (NDot57)**: Embed an ultra-compact subsetted `ndot57.woff2` font (~5.5 KB raw / ~3.2 KB gzipped) in `SYSTEM_FS` for clock displays, app titles, hero metrics (100 MS/s, 3.30V), and section headers.
- **High-Tracking Technical Monospace**: Use native system monospace fonts with `letter-spacing: 0.12em` and uppercase transforms for hardware pin numbers (`GP10`), memory offsets (`0x00204000`), baud rates, and status tags (`[RO]`, `[RW]`, `[SAFE]`).
- **Zero Framework Bloat**: Implement the entire system using pure DaisyUI 5, Tailwind utilities, and CSS custom properties with zero external JavaScript libraries.
- **FOUC Prevention**: Ensure that every HTML entry point (`index.html`, `lab.html`, `terminal.html`, `files.html`) includes the pre-render restoration script that binds `data-style` before first paint.

**Non-Goals:**
- Using NDot for small body paragraphs, code editors, or file names (maintaining high legibility is paramount).
- Re-architecting HTML layouts (the exact same markup renders both Bento and Industrial).

## Decisions

### 1. CSS Custom Properties & `data-style` Attribute
- **Decision**: Control the persona via `<html data-style="industrial|bento">` and CSS custom variables (`--font-display`, `--radius-box`, `--canvas-bg`).
- **Rationale**: Keeps HTML DOM 100% identical between styles. Switching styles takes 1 DOM attribute change with instantaneous client-side repaint.

### 2. Dedicated `themes/nothing.css` Plugin
- **Decision**: Create `themes/nothing.css` and compile it with `tools/compile_themes.py` into `webui/themes.css`.
- **Tokens**:
  - `--color-base-100: #000000;` (Pitch OLED black)
  - `--color-base-200: #0a0a0a;` (Titanium surface)
  - `--color-base-300: #1a1a1a;` (Crisp 1px border)
  - `--color-base-content: #f4f4f4;` (Stark white text)
  - `--color-primary: #D71921;` (Iconic Nothing Signal Red)
  - `--radius-box: 0.5rem;` (Industrial geometric box)
  - `--border: 1px;`

### 3. Font Delivery in `SYSTEM_FS`
- **Decision**: Place subsetted `ndot57.woff2` into `webui/fonts/ndot57.woff2` and serve via `picoserve` (`fs_http.rs` already supports `font/woff2`).
- **Rationale**: A subset of ~70 characters (uppercase `A-Z`, numbers `0-9`, symbols `. : - / _ [ ] # % ●`) occupies only ~5.5 KB, compressing to ~3.2 KB gzipped. Fits easily in the 512 KB `SYSTEM_FS` partition.

### 4. Global Settings Integration
- **Decision**: Add an "Interface Persona" segmented toggle directly in `#modal-settings` in `index.html` and `lab.html`.
- **Rationale**: Provides instant user discovery and choice. Persists to `localStorage.getItem('agentla-ui-style')`.

## Risks / Trade-offs

- **[Risk] Dot-matrix font legibility on low-resolution displays**:
  - *Mitigation*: Restrict dot-matrix styling exclusively to headers, hero digits, and section titles at or above 16px. Keep all body text, file paths, and logs in standard crisp sans/monospace.
- **[Risk] Flash of Unstyled Content (FOUC)**:
  - *Mitigation*: All pages execute an inline synchronous `<script>` in `<head>` that reads `localStorage.getItem('agentla-ui-style') || 'industrial'` and sets the attribute on `document.documentElement` before layout calculation begins.

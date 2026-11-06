## Context

The AgentLA web operating system has two primary views:
1. The **Lab Workbench** (`/lab` or `/` in workbench mode): Provides low-level signal capture, mixed-signal oscilloscope (MSO), bus protocol sniffers, hardware crossbar matrix, and SWD/RISC-V debugger.
2. The **App Launcher** (`/apps`): Serves as the tactile home dashboard where users launch core instruments, browse installed LittleFS web apps, and manage applications from the community store.

While both pages use pure DaisyUI semantic tokens, their navigation headers, iconography, and section structures diverged:
- The top application bar in the launcher omitted the Hardware Profile switcher.
- The Core Instruments grid used thin feather-style line strokes instead of the rich 2-tone Solar Icons used throughout the Lab.
- Storage partition telemetry occupied significant vertical real estate above the app grid rather than anchoring the screen as an OS bottom dock.
- User-installed apps lacked a clear resolution pipeline for rendering custom brand icons.

## Goals / Non-Goals

**Goals:**
- Unify the top bar layout and DOM structure between `/lab` and `/apps` so users experience consistent hardware telemetry, emergency safe state controls, profile switching, and settings dialog.
- Implement the Hardware Profile selector in `apps.html`, wired to `/api/profiles` for switching stored pin configurations.
- Update the Core section header to "Core Apps" and replace all placeholder SVGs with authentic 2-tone Solar Icons.
- Design and implement a robust icon resolution hierarchy for user-installed LittleFS applications (`app.iconSvg`, `app.icon`, root `icon.svg`/`icon.png`, with fallback Solar app box).
- Relocate flash partition telemetry into a fixed DaisyUI bottom `.dock` (`.dock-item`, multi-segment progress bar, live metrics) with safe-area spacing.

**Non-Goals:**
- Modifying backend LittleFS C driver partitioning geometry (already solid at ~2.42MB for Pico 2 / 14.4MB for BakuProbe).
- Altering the REST API schemas for `/api/v1/apps` or `/api/profiles`.

## Decisions

### 1. Unified Top Bar with Display-Only Brand Logo
- **Decision**: Render the full header in `apps.html` matching `index.html` including `#profile-dropdown-wrapper`. Keep the AgentLA logo (`#brand-logo`) display-only on `/apps` (cursor default, no navigation action), because the user is already on the launcher and can enter the Lab via the flagship tile.
- **Alternatives Considered**:
  - Making the logo navigate to `/lab`: Rejected based on user preference to enter via the core apps tiles.
  - Adding a separate "Open Lab" button: Rejected to prevent clutter in the compact header.

### 2. 2-Tone Solar Icon Integration for Core Apps
- **Decision**: Integrate authentic 2-tone Solar SVGs (`width="32"` or `"28"`, `viewBox="0 0 24 24"`) with `opacity="0.5"` background layer and solid foreground paths.
  - *AgentLA Lab (Flagship)*: Solar Workstation / Lab bench.
  - *MSO Scope*: Solar Waveform oscilloscope.
  - *Pinout Matrix*: Solar MCU Chip & Matrix.
  - *USB Adapters*: Solar Dual-Interface Link.
  - *Console*: Solar Terminal REPL Prompt.
- **Rationale**: Matches the desktop left navigation rail in the Lab, creating instant visual recognition across views.

### 3. User Installed App Icon Resolution Pipeline
- **Decision**: In `renderInstalledApps()`, evaluate icons in sequence:
  1. `app.iconSvg`: If present, sanitize and inject inline SVG (reactive to theme text colors).
  2. `app.icon` or root file: If specified as a file path (e.g. `icon.svg`, `icon.png`) or present in the app directory, render an `<img>` tag pointing to `/apps/${app.id}/${app.icon || 'icon.svg'}` with an `onerror` handler.
  3. *Fallback*: Authentic 2-tone Solar App Box icon if no custom graphic exists.
- **Rationale**: Gives third-party developers the flexibility to ship lightweight inline SVGs or standard image assets in their TAR packages.

### 4. Fixed OS Storage Bottom Dock (`.dock`)
- **Decision**: Replace the in-page storage card with a fixed bottom dock using DaisyUI `.dock` (`position: fixed; bottom: 0; left: 0; right: 0; height: 3.5rem; background: var(--color-base-100); border-top: 1px solid var(--color-base-300)`).
- **Structure**:
  - Left dock item: Flash storage icon + "Flash Storage".
  - Center: Segmented capacity bar with color-coded fills (System ROM: `info`, Installed Apps: `primary`, Free: `base-300`).
  - Right: Live formatted text badge (`1.4 MB used · 5.5 MB free of 8.0 MB`).
- **Padding**: Add `padding-bottom: 5rem` to `.launcher-workspace` and `.launcher-container` so the bottom row of app tiles is never obscured by the dock.

## Risks / Trade-offs

- *[Risk] Bottom dock overlapping with scrollable app grid on mobile/small viewports.*
  → *Mitigation*: Add `padding-bottom: calc(4.5rem + env(safe-area-inset-bottom, 0px))` to `.launcher-workspace`.
- *[Risk] Missing icon files causing broken image icons on user apps.*
  → *Mitigation*: Include `onerror="this.style.display='none'; this.nextElementSibling.style.display='block';"` with the Solar fallback icon directly sibling-rendered.
- *[Risk] Profile dropdown state sync between pages.*
  → *Mitigation*: Re-use `/api/profiles` fetch logic and `localStorage` active profile key so the selected profile reflects across page transitions.

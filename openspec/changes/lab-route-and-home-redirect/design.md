## Context

The AgentLA platform is evolving into an integrated embedded operating environment for the RP2350 microcontroller. The system includes:
- Core workbench (logic analyzer, oscilloscope, pinout, protocol decoders, signal generator)
- Multi-app launcher (`/apps`) with LittleFS app storage and app installer
- User-installed third-party web apps running under `/apps/{app-name}/`
- REST API `/api/v1/` for device telemetry, LittleFS filesystem operations, and settings

Previously, `apps.html` had custom styling that diverged from `index.html` (the workbench), lacked the System Settings modal, had redundant top-bar pills, and trapped users inside installed apps. This design unifies both pages into a single OS experience.

## Goals / Non-Goals

**Goals:**
- Move AgentLA Lab workbench to route `/lab`, `/lab/`, and `/lab/index.html`.
- Redirect device root (`/`) to the configured `default_home_route` using HTTP 302 (Found) to avoid aggressive browser caching. Default out-of-the-box landing page: `/lab`.
- Unify `apps.html` styling completely with `style.css` and `themes.css` (DaisyUI tokens, noise, depth, and palettes).
- Clean OS top bar on both `/lab` and `/apps`: remove redundant "Workbench" and "App Launcher" navigation pills. In `/lab`, clicking `[❖ AgentLA]` navigates to `/apps`.
- Redesign the App Launcher into an authentic OS app grid: tactile squircle app tiles (tap-to-launch), spotlight search, segmented storage bar, and "AgentLA Lab" flagship tile.
- Embed and share `#modal-settings` in both `/lab` and `/apps`, supporting Theme Mode, Default Light/Dark themes, Default Landing Page, Reboot RP2350, Enter BOOTSEL, and Hardware Telemetry.
- In custom installed apps (`/apps/*`), inject a circular glassmorphic Floating Action Button (FAB) (44×44px) fixed at bottom-right (`bottom: 16px; right: 16px; z-index: 99999`) containing the `win-rar-bold-duotone` Solar icon for instant 1-tap return to `/apps`.

**Non-Goals:**
- Backward compatibility for root `/` serving raw workbench directly without redirection (user explicitly confirmed zero backward compatibility needed).
- Complex multi-level navigation menus inside the floating FAB (only the `win-rar-bold-duotone` icon navigating to `/apps`, no text or lab link).
- Modifying files stored inside LittleFS on upload/install.

## Decisions

### 1. HTTP 302 Redirection vs 301 for Root `/`
- **Choice**: Use HTTP 302 Found with `Cache-Control: no-cache, no-store, must-revalidate`.
- **Rationale**: Browsers cache HTTP 301 redirects permanently in their internal disk cache. If a user changed their default landing page in Settings from `/lab` to `/apps`, a 301 would prevent the browser from seeing the change until caches are manually cleared. HTTP 302 ensures immediate reactivity.

### 2. In-Memory Route Cache for Instant 302 Dispatch
- **Choice**: Maintain `char cached_default_home_route[64]` in RAM (`app_routing.c`).
- **Rationale**: When clients visit `http://10.10.10.10/`, reading from LittleFS flash storage on every request would cause flash read overhead and latency. By loading `default_home_route` into RAM on boot and updating it whenever `PUT /api/v1/settings` is called, the 302 response is constructed and returned in microseconds with zero flash I/O.

### 3. Removal of Redundant Top-Bar Navigation Pills
- **Choice**: Remove "Workbench" and "App Launcher" text pills from the top bar.
- **Rationale**: In `/lab`, clicking the top-left `[❖ AgentLA]` brand takes the user to `/apps`. In `/apps`, the prominent "AgentLA Lab" squircle tile opens `/lab`. Having text pills in the menu bar cluttered the interface and contradicted real OS design patterns.

### 4. OS Squircle Grid & Spotlight Search Metaphor
- **Choice**: App launcher renders 56×56px squircle tiles with smooth radius, `--card-rim-highlight`, spring hover animations, and direct tap-to-launch. Centered spotlight search filters tiles instantaneously.
- **Rationale**: Transitions the launcher from a text-heavy directory listing into a tactile, responsive operating system interface.

### 5. Stream Injection for Installed Apps (`/agentla-bridge.js`)
- **Choice**: Firmware transparently appends `<script src="/agentla-bridge.js" defer></script>` to HTML responses under `/apps/*` during `fs_read_custom`.
- **Rationale**:
  - LittleFS files remain pristine on flash.
  - Zero RAM buffering on RP2350: firmware adjusts `Content-Length` in the HTTP response header and streams the 52-byte script tag at the end of the LittleFS file stream.

### 6. Closed Shadow DOM for Injected FAB
- **Choice**: `agentla-bridge.js` mounts its markup inside `container.attachShadow({ mode: 'closed' })`.
- **Rationale**:
  - CSS Isolation: Any global CSS in installed apps will not leak into the FAB, nor will the FAB styles leak into the app.
  - Inlines the `win-rar-bold-duotone` Solar Icon with opacity layers (.3, .7, .5, 1.0) and glassmorphic circular button styling.

### 7. Shared System Settings Modal (`#modal-settings`)
- **Choice**: Both `/lab` and `/apps` embed the exact same modal markup and share the settings controller to interact with `/api/v1/settings` and `/api/v1/system`.
- **Rationale**: Ensures complete feature parity and unified administration regardless of which view the user is on.

## Risks / Trade-offs

- [Risk] *Installed app has interactive elements at bottom-right (e.g. scroll-to-top, chat widget)* → Mitigation: Keep the FAB compact (44×44px circle) with subtle opacity, provide `?no_launcher=1` query param suppression and `window.AgentLANoLauncher = true` API check.
- [Risk] *Asset loading across `/lab` and `/`* → Mitigation: Use absolute paths (`/style.css`, `/themes.css`, `/app.js`) in `index.html` and `apps.html`.
- [Risk] *Corrupted `/settings.json` on power cut* → Mitigation: Fallback to `/lab` if setting is absent or unparseable.

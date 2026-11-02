## Why

The desktop WebUI currently places all 5 workbench mode tabs (`Active Bench`, `Adapters`, `Debugger`, `Pin Matrix`, and `Console`) directly inside the top application header. This causes severe horizontal crowding on standard laptops and 1080p monitors, competing with the emergency `SAFE STATE` button, flash hardware profile dropdown, connection status watchdog, and theme switcher. Web-demo-specific buttons (`Connect USB` and `← Portal`) introduce additional visual noise.

Meanwhile, in the Console tab:
1. Long command responses, serialized JSON payloads, and hex dumps do not wrap because `#cmd-output-content` lacks unbreakable word breaking (`overflow-wrap: anywhere; word-break: break-word`). Due to CSS Grid's default `minmax(auto, 2fr)` track sizing, this causes a catastrophic column blowout where the terminal window extends far beyond the viewport width.
2. Console input commands and output responses lack visual distinction and semantic theming.
3. Users and AI agent developers frequently need to copy individual command outputs or registers from the terminal, but must manually highlight text.
4. Developers integrating autonomous agents (Google Antigravity, Anthropic Claude, OpenCode / Cline) lack an in-app setup guide for AgentLA's Model Context Protocol (MCP) server.
5. The terminal "Clear Output" button is marooned in the upper view header rather than living inside the terminal window.

Relocating the desktop navigation tabs into a dedicated **Vertical Left Sidebar Navigation** with **-90° rotated words**, decluttering the top bar, adding the Open Source Initiative (OSI) link, implementing comprehensive console text wrapping, semantic token theming, click-to-copy line mechanics, and an integrated Agent Guide modal solves all these pain points while creating a professional instrument-grade UX.

## What Changes

- **Desktop Vertical Left Sidebar Navigation**:
  - Relocate primary workbench mode selection from the top header to a dedicated vertical activity spine on the left edge of the desktop workspace ($\ge 900\text{px}$).
  - Each sidebar tab presents its icon at the top, with the **word label itself rotated by -90 degrees** (`transform: rotate(-90deg); display: inline-block; white-space: nowrap;`).
  - Active tab highlighted using theme token `var(--color-primary)` with active accent indicator and `var(--color-base-100)` card elevation.
  - On mobile/touch portrait viewports ($< 900\text{px}$), automatically transitions to the thumb-friendly bottom navigation bar.
- **Top Application Bar Decluttering**:
  - Remove all 5 mode buttons from the top bar on desktop, leaving the header dedicated exclusively to branding, telemetry, and critical utilities.
  - Remove deprecated `#btn-webserial-connect` and `← Portal` demo buttons.
  - Add an Open Source Initiative (OSI) keyhole icon button anchor (`#btn-open-source`) immediately to the left of the theme toggle, linking to `https://rambros3d.github.io/AgentLA` (`target="_blank" rel="noopener noreferrer"`).
- **Console Terminal Window - Text Wrapping & Viewport Containment**:
  - Apply `white-space: pre-wrap; overflow-wrap: anywhere; word-break: break-word; max-width: 100%;` to `#cmd-output-content` and `.output-content`.
  - Enforce `overflow-x: hidden; min-width: 0; max-width: 100%; box-sizing: border-box;` on `.panel.terminal-display`.
  - Fix grid track blowout in `.console-main-grid` by switching from `2fr 1fr` to `minmax(0, 2fr) minmax(0, 1fr)`, preventing any unwrapped string from widening the viewport.
  - Set `min-width: 0; max-width: 100%;` on `.console-terminal-card` and `.console-cheatsheet-card`.
- **Console Semantic Color Theming (Zero Hardcoded Values)**:
  - Input commands (`> ...`): Styled using `var(--color-primary)` (or `var(--color-info)`).
  - Standard output text: Styled using `var(--color-base-content)`.
  - Success responses (`OK`, `PASS`): Styled using `var(--color-success)`.
  - Error responses (`ERROR`, `FAIL`): Styled using `var(--color-error)`.
  - Line hover state: Highlighted using `var(--color-base-300)`.
- **Click-to-Copy Output Lines**:
  - Clicking any console output line copies its text directly to the system clipboard via `navigator.clipboard.writeText(...)`.
  - Renders a temporary floating inline "Copied!" badge/tooltip (`var(--color-primary)` background, `var(--color-primary-content)` text) next to the line.
- **Console Header - Agent Guide Modal**:
  - Replace the old header clear button with an **"Agent Guide"** button (`#btn-agent-guide`).
  - Opens `#modal-agent-guide` displaying unified MCP integration instructions for **Google Antigravity**, **Claude Desktop / Code**, and **OpenCode / Cline / Cursor**.
  - Features a universal JSON configuration snippet with 1-click clipboard copy, transport endpoints, core tool definitions, and sample prompts.
- **Console Terminal Window - Relocated Clear Button**:
  - Relocate the Clear button into the bottom-right corner inside `.terminal-display` as a compact badge button with trash icon and "Clear" text.

## Capabilities

### Modified Capabilities
- `webui-control-surface`: Top header is decluttered; mode buttons moved to sidebar; demo buttons removed; OSI link added; console header features Agent Guide modal; console terminal incorporates integrated clear badge.
- `adaptive-workspace-hub`: Desktop viewports ($\ge 900\text{px}$) adopt a left vertical activity spine with -90° rotated word labels; mobile viewports ($< 900\text{px}$) retain the bottom navigation dock.
- `firmware-repl-console`: Terminal output enforces unbreakable text wrapping and CSS Grid containment, applies semantic color tokens to input/output, supports interactive click-to-copy line functionality, and features bottom-right clear controls.

## Impact

- **Affected Files**:
  - `firmware/html/index.html` (left vertical sidebar navigation markup with -90° label wrappers, top bar decluttering, OSI link, console header Agent Guide button, terminal clear badge, Agent Guide modal markup)
  - `firmware/html/style.css` (desktop vertical sidebar styles with `transform: rotate(-90deg)`, responsive media queries, console text wrapping rules, grid track containment, semantic console line tokens, inline copied badge styles)
  - `firmware/html/app.js` (mode switching sync for sidebar/mobile nav, click-to-copy line handlers with feedback, Agent Guide modal controller, MCP JSON copy handler)
  - `web/demo/mock-bridge.js` (removal of dead demo button listeners)
- **Zero Flash/Firmware C Regressions**: Strictly frontend HTML/CSS/JS assets; zero changes to C microcontroller drivers.

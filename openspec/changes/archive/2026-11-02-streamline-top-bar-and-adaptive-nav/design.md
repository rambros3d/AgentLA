## Context

AgentLA's WebUI is a unified workbench operating both as an embedded dashboard (USB RNDIS `http://192.168.7.1`) and as an interactive hosted simulator (`web/demo`). 

As the application grew to five distinct instruments (`Active Bench`, `Adapters`, `Debugger`, `Pin Matrix`, and `Console`), packing all five mode switches into the top application bar alongside safety-critical controls created severe horizontal tension on standard desktop and laptop screens (1024px–1440px). 

Simultaneously, the Console tab suffered from two major usability bugs:
1. Long command outputs (hex registers, JSON responses, binary dumps) do not wrap, causing CSS Grid track blowout that pushes the right card and layout off-screen.
2. Console text lacks visual contrast between commands and responses, and users must awkwardly drag-select text to copy values.

Relocating desktop workbench modes to a dedicated **Vertical Left Activity Spine** with **-90° rotated word labels** frees up the entire top bar. Addressing the console text wrapping, semantic token theming, click-to-copy line interactions, and in-app Agent Guide elevates the workbench to commercial instrument quality.

## Goals / Non-Goals

**Goals:**
- Free up the top application header by moving workbench mode tabs to a dedicated **Vertical Left Sidebar** on desktop ($\ge 900\text{px}$).
- Orient tab text on the vertical sidebar with **the word itself rotated by -90 degrees** (`transform: rotate(-90deg)`), reading elegantly from bottom to top below each 20px icon.
- Eliminate top-bar clutter by removing `Connect USB` and `← Portal` demo buttons.
- Add an Open Source Initiative (OSI) keyhole icon button immediately left of the theme toggle linking to `https://rambros3d.github.io/AgentLA`.
- Prevent console window viewport blowout by enforcing `overflow-wrap: anywhere; word-break: break-word;` and CSS Grid `minmax(0, ...)` containment.
- Apply semantic theme color tokens (OKLCH, zero hardcoded colors) to distinguish console input commands from output responses.
- Enable 1-click copying of any console output line with a temporary inline "Copied!" indicator badge.
- Introduce an "Agent Guide" button in the console header opening a unified MCP setup modal for AI coding assistants.
- Relocate the Clear button inside the bottom-right corner of the terminal window as a compact badge button.

**Non-Goals:**
- Removing any workbench modes (all 5 remain easily accessible).
- Altering any C backend firmware code or USB endpoints.
- Introducing heavy 3rd-party frontend dependencies (everything remains vanilla HTML/CSS/JS).

## Decisions

### 1. Desktop Left Vertical Sidebar with -90° Rotated Word Labels
- **Rationale**: Moving mode switches out of the top header eliminates horizontal competition with the emergency Safe State button, Profile Selector, and Watchdog. The vertical sidebar takes only ~48px–54px of horizontal space while providing ample vertical room for all 5 modes.
- **Word Rotation Mechanics**: The user explicitly requested **the words themselves rotated by -90°** rather than stacked individual letters.
- **Implementation**:
  ```css
  .desktop-vertical-nav .sub-btn {
      display: flex;
      flex-direction: column;
      align-items: center;
      justify-content: center;
      gap: 12px;
      padding: 14px 4px;
      width: 44px;
      border: none;
      background: transparent;
      color: var(--text-secondary);
      cursor: pointer;
  }
  .desktop-vertical-nav .sub-btn .solar-icon {
      width: 20px;
      height: 20px;
      flex-shrink: 0;
  }
  .desktop-vertical-nav .sub-btn .btn-label-vertical {
      display: inline-block;
      transform: rotate(-90deg);
      white-space: nowrap;
      font-size: 11px;
      font-weight: 700;
      letter-spacing: 0.6px;
      text-transform: uppercase;
      transform-origin: center center;
  }
  .desktop-vertical-nav .sub-btn.active {
      color: var(--color-primary);
      background: var(--color-base-100);
      border-left: 3px solid var(--color-primary);
  }
  ```
- **Responsive Transition**: On mobile viewports ($< 900\text{px}$), the vertical sidebar hides and the fixed bottom navigation bar activates, displaying icon-only for inactive tabs and icon + label for the active tab.

### 2. Top Bar Decluttering & OSI Project Link
- **Rationale**: With mode tabs moved to the sidebar and redundant demo buttons removed, the top bar is clean, calm, and exclusively focused on system health and safety.
- **OSI Link**: An official Open Source Initiative keyhole SVG icon in a 32px button frame (`#btn-open-source`) sits right next to the theme toggle, linking to `https://rambros3d.github.io/AgentLA` in a new tab.

### 3. Console Window Viewport Containment & Text Wrapping
- **Rationale**: CSS Grid tracks defined as `2fr` compute to `minmax(auto, 2fr)`. Because `#cmd-output-content` had `white-space: pre-wrap` without unbreakable word splitting, long hex values and JSON forced the minimum content width outward, blowing out the page.
- **Implementation**:
  - `#cmd-output-content`, `.output-content`: `white-space: pre-wrap; overflow-wrap: anywhere; word-break: break-word; max-width: 100%;`
  - `.panel.terminal-display`: `overflow-x: hidden; min-width: 0; max-width: 100%; box-sizing: border-box;`
  - `.console-main-grid`: `grid-template-columns: minmax(0, 2fr) minmax(0, 1fr); min-width: 0;` (and `minmax(0, 1fr)` at $< 900\text{px}$).
  - `.console-terminal-card`, `.console-cheatsheet-card`: `min-width: 0; max-width: 100%;`

### 4. Console Input/Output Theming (Semantic Tokens Only)
- **Rationale**: Clean visual distinction between user/agent stimulus commands and hardware responses without any hardcoded HEX or RGB strings.
- **Tokens**:
  - Commands (`> ...`): `color: var(--color-primary); font-weight: 600;`
  - Normal output: `color: var(--color-base-content);`
  - Success responses (`OK`, `PASS`): `color: var(--color-success); font-weight: 600;`
  - Error responses (`ERROR`, `FAIL`): `color: var(--color-error); font-weight: 600;`
  - Hover state: `background: var(--color-base-300); border-radius: 3px; cursor: pointer;`

### 5. Click-to-Copy Line Interaction
- **Rationale**: Developers inspecting registers or pin mappings frequently need to copy a specific line.
- **Behavior**: Clicking any rendered line copies its string (stripping `> ` if it's an input command). A non-blocking inline badge displays "Copied!" for 1.2s before fading out.

### 6. Agent Guide Modal & Floating Clear Button
- **Agent Guide Button**: `<button class="action-btn outline btn-sm" id="btn-agent-guide">` in `.matrix-header-actions` triggers `#modal-agent-guide`.
- **Unified MCP Guide**: Explains how to connect Google Antigravity, Anthropic Claude, and OpenCode/Cline to AgentLA's MCP server over `http://192.168.7.1/mcp` and USB serial `/dev/ttyACM0`, with a 1-click copyable `mcpServers.agentla` JSON block.
- **Floating Clear Button**: Compact badge button (`.terminal-clear-badge-btn`) with trash icon and "Clear" text positioned at `bottom: 10px; right: 12px; z-index: 10;` inside `.panel.terminal-display`.

## Risks / Trade-offs

- **[Risk] Long tab labels in vertical sidebar**: Very long labels could exceed the vertical segment allocated for a tab.
  - **Mitigation**: The 5 workbench tabs have concise labels (`ACTIVE BENCH`, `ADAPTERS`, `DEBUGGER`, `PIN MATRIX`, `CONSOLE`). With 11px font size, the longest label (`ACTIVE BENCH`) spans ~75px of height, fitting comfortably within the 100px+ allocated per tab on a standard vertical layout.
- **[Risk] Unintended copy on text selection in console**: User selecting text with the mouse might inadvertently trigger line copy.
  - **Mitigation**: The click handler checks `window.getSelection().toString().length === 0` so that deliberate text drag-selection is not overridden by line copy.

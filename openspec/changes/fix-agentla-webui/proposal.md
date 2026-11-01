## Why

The AgentLA probe-side WebUI currently fails to boot or operate due to a fatal JavaScript scoping bug in `firmware/html/index.html` (an unclosed `initFlasher` brace trapping the entire script and preventing `DOMContentLoaded` registration), a CSS class mismatch hiding the start menu, and a protocol mismatch where frontend callers expect raw text stdout while the backend returns JSON-RPC 2.0 objects. Fixing these issues makes the browser interface fully functional for flashing, debugging, logic analysis, and hardware adapter interactions over USB-RNDIS.

## What Changes

- **Fix WebUI Script Scoping**: Close `initFlasher()` properly and remove the redundant script-terminating brace so `DOMContentLoaded`, tool initializers, and window management event listeners register on page load.
- **Fix Start Menu Visibility**: Update start button click handling and styling to toggle the `.show` class defined in `style.css` rather than the unmatched `.hidden` class.
- **Standardize JSON-RPC Response Handling**: Update `callMcp()` to parse JSON responses and return structured result objects (`stdout`, `status`, `error`), replacing brittle regex matching on raw response text.
- **Refine Long-Running Job Classification**: Update `webui_command_is_long()` in `firmware/src/webui_command_bridge.c` so high-frequency synchronous chunk commands (`FLASH WRITE`, `FLASH VERIFY`, `FLASH READ`) execute directly, while bulk long operations (`FLASH ERASE`, `MASSERASE`) remain queued.
- **Fix Tool Window Command Mappings**:
  - Debugger "Read memory" maps to `DEBUG READ <addr>` instead of `FLASH READ`.
  - I2C Master "Write then Read" sends separate sequential requests using the active claim handle rather than an invalid composite `"then"` command string.
  - GPIO pin grid dispatches targeted pin inspect queries.
  - Periodic heartbeat calls `updateStatus()` to accurately reflect connection state.

## Capabilities

### New Capabilities
None.

### Modified Capabilities
- `webui-control-surface`: Restore page load lifecycle, Start Menu visibility, tool event dispatching, and live connection status indicators.
- `webui-mcp-api`: Clarify JSON-RPC response parsing on the client side and distinguish synchronous chunk operations from background job queues in the bridge.
- `webui-workflow-bridge`: Align tool workflows (flasher chunk loop, debugger reads, I2C sequential operations) with the firmware's command grammar and execution model.

## Impact

- Frontend assets: `firmware/html/index.html` and `firmware/html/style.css`.
- Firmware command bridge: `firmware/src/webui_command_bridge.c`.
- Build artifacts: `AgentLA.bin` / `AgentLA.uf2` generated via `pico_set_lwip_httpd_content`.
- No host CLI or python SDK breaking changes.

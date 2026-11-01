## Context

AgentLA integrates an on-probe desktop WebUI served directly from the RP2350 via lwIP HTTPD over USB-RNDIS (`192.168.7.1` / `agentla.lab`). The interface uses WinBox.js for floating window management and communicates with the firmware via a JSON-RPC `/mcp` endpoint and `/sse` streams.

During auditing, several critical issues were discovered:
1. **Script Scoping Failure**: In `firmware/html/index.html`, `initFlasher()` lacked a closing brace `}` before `initCommand()`. A closing brace was placed at the script's end, accidentally nesting all tool initializers and the `DOMContentLoaded` listener inside `initFlasher()`. Consequently, `DOMContentLoaded` never executes on page load, leaving the `#loading-screen` permanently visible and blocking all UI interactions.
2. **Start Menu Class Discrepancy**: `style.css` styles `#start-menu` as hidden by default (`opacity: 0; visibility: hidden;`) and makes it visible via `#start-menu.show`. `index.html` toggled `.hidden`, preventing the start menu from ever displaying.
3. **JSON-RPC Response Handling**: The firmware command bridge (`firmware/src/webui_command_bridge.c`) wraps command stdout into JSON-RPC 2.0 objects (`{"jsonrpc":"2.0","result":{"stdout":"..."},"id":0}`). However, the JavaScript `callMcp()` function returned raw response text, and calling functions checked this text against start-of-line regexes (e.g. `/^OK FLASH ERASE/`). Since responses begin with `{"jsonrpc"`, all checks fail.
4. **Command Classification**: In `webui_command_bridge.c`, `webui_command_is_long()` marks any command starting with `FLASH` as a queued job. This causes `FLASH WRITE` and `FLASH VERIFY` chunk operations to return `queued: true` rather than executing synchronously, stalling chunk-based flashing loops.

## Goals / Non-Goals

**Goals:**
- Fix JavaScript scoping so the WebUI boots cleanly, fades out the loader, and binds window and launcher controls.
- Synchronize Start Menu CSS classes between `style.css` and `index.html`.
- Standardize `callMcp()` to return a structured response containing `stdout`, execution status, and error details.
- Differentiate bulk erase operations from synchronous chunk write/verify operations in `webui_command_bridge.c`.
- Correct tool commands (Debugger memory read to `DEBUG READ`, I2C Write-then-Read to sequential calls, and connection indicator polling).

**Non-Goals:**
- Redesigning the desktop CSS theme or replacing WinBox.
- Full streaming logic analyzer canvas renderer (retaining baseline command dispatch and stable canvas UI).
- Modifying USB RNDIS descriptor or host CDC drivers.

## Decisions

### 1. Script Scoping and Initialization Repair
- **Decision**: Add the missing closing brace `}` to `initFlasher(win)` in `firmware/html/index.html` before `initCommand(win)` and remove the redundant `}` before `</script>`.
- **Alternatives Considered**: Refactoring into separate modular `.js` files. Rejected because embedding static files into RP2350 flash via `pico_set_lwip_httpd_content` benefits from a compact single-page layout.

### 2. Start Menu Visibility Contract
- **Decision**: Update `index.html` start button and document click listeners to toggle and remove the `.show` class on `#start-menu`, matching `style.css`:
  ```javascript
  startBtn.addEventListener('click', function() { startMenu.classList.toggle('show'); });
  ```
- **Rationale**: Keeps `style.css` transitions (`opacity 0.2s ease, transform 0.2s cubic-bezier(...)`) fully functional without breaking existing styles.

### 3. Unified JSON-RPC Response Parsing
- **Decision**: Update `callMcp()` in `index.html` to parse response JSON:
  ```javascript
  async function callMcp(method, name, args) {
      ...
      const text = await r.text();
      try {
          const data = JSON.parse(text);
          if (data.error) return { ok: false, error: data.error.message, stdout: '', raw: data };
          const stdout = (data.result && data.result.stdout) || '';
          return { ok: true, stdout: stdout, result: data.result, raw: data };
      } catch (e) {
          return { ok: false, error: text, stdout: text, raw: text };
      }
  }
  ```
  Update callers (`initFlasher`, `initDebugger`, `initI2c`, etc.) to inspect `res.stdout` or `res.ok`.

### 4. Fine-Grained Job Classification in Firmware Bridge
- **Decision**: In `firmware/src/webui_command_bridge.c`, update `webui_command_is_long()` to inspect both the primary command and subcommand:
  - Long/Queued: `FLASH ERASE`, `MASSERASE`, and standalone `CAPTURE`.
  - Synchronous: `FLASH WRITE`, `FLASH VERIFY`, `FLASH READ`, `DEBUG *`, `I2C_*`, `SAFEDISCOVER`, `STATUS`.
- **Rationale**: 256-byte flash writes complete in ~2ms on target MCUs, which is well within standard HTTP request latency budgets and avoids asynchronous polling overhead for hundreds of chunks.

### 5. Debugger and Adapter Command Alignment
- **Decision**:
  - Change Debugger `#dbg-readmem` to call `DEBUG READ <addr>` instead of `FLASH READ`.
  - Update I2C `#i2c-wr` to await `I2C_WRITE ...` followed by `I2C_READ ...` rather than emitting an unsupported composite string.
  - Schedule `updateStatus()` on boot and a periodic interval (e.g. 5 seconds) to reflect live probe connection status.

## Risks / Trade-offs

- **[Risk] Long erase times blocking HTTP request:**
  `FLASH ERASE` can take 500ms–2s depending on flash size.
  *Mitigation:* Keep `FLASH ERASE` classified as a long-running job with progress/SSE notifications, or ensure erase is dispatched before chunk loops.
- **[Risk] Flash footprint increase:**
  Modifying `index.html` and `webui_command_bridge.c`.
  *Mitigation:* Net character change in `index.html` is minimal (balanced braces and streamlined parser), having negligible impact on RP2350 flash image size.

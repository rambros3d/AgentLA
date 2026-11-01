## 1. WebUI Script Scoping and Lifecycle

- [x] 1.1 Close `initFlasher` with a missing closing brace before `initCommand` and remove the trailing redundant brace before `</script>` in `firmware/html/index.html`.
- [x] 1.2 Verify that the `DOMContentLoaded` event listener registers at the top level and fades out `#loading-screen`.
- [x] 1.3 Update start button and document click listeners to toggle and remove the `.show` class on `#start-menu`, matching `style.css`.

## 2. JSON-RPC Protocol and Command Bridge Alignment

- [x] 2.1 Refine `webui_command_is_long()` in `firmware/src/webui_command_bridge.c` to keep `FLASH WRITE`, `FLASH VERIFY`, and `FLASH READ` synchronous.
- [x] 2.2 Update `callMcp()` in `firmware/html/index.html` to parse JSON-RPC payloads and return structured `{ ok, stdout, result, error, raw }` objects.
- [x] 2.3 Update Flasher erase, write, and verify loops to evaluate `res.ok` and `res.stdout` rather than testing raw HTTP response strings.

## 3. Tool Control Surface Polish and Command Corrections

- [x] 3.1 Update Debugger memory reading in `#tpl-debugger` to dispatch `DEBUG READ <addr>` instead of `FLASH READ`.
- [x] 3.2 Update I2C "Write then Read" to dispatch sequential `I2C_WRITE` and `I2C_READ` requests referencing the active claim handle.
- [x] 3.3 Wire `updateStatus()` into `DOMContentLoaded` and a periodic interval to reflect probe connection state.
- [x] 3.4 Update GPIO pin block click handlers to query pin state.

## 4. Verification and Build Sanity

- [x] 4.1 Validate JavaScript syntax and DOM listener attachment using Node.js script.
- [x] 4.2 Capture headless browser screenshot confirming clean boot, loader fade-out, and interactive desktop.
- [x] 4.3 Run `cmake --build build` or verify CMake packaging with `pico_set_lwip_httpd_content` to confirm binary compilation.

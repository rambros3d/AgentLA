## 1. Network Foundation (PicoLab scaffold)
- [x] 1.1 Add dhserver.c, dnserver.c and RNDIS descriptor files
- [x] 1.2 Configure CMake with pico_lwip/pico_lwip_http/pico_httpd_content
- [x] 1.3 Implement USB CDC vs RNDIS composite descriptors
- [x] 1.4 Integrate init_lwip(), DHCP, DNS server
- [x] 1.5 Network keepalive/network flush

## 2. HTTP + Static Assets
- [x] 2.1 Embed html/index.html etc via pico_httpd_content
- [x] 2.2 Create basic HTTP handler + 404 fallback
- [x] 2.3 CSRF token? (Optional)
- [x] 2.4 Verify byte stream HTTP GET /

## 3. /mcp JSON-RPC Endpoint
- [x] 3.1 Parse JSON-RPC request id/method/params
- [x] 3.2 Tool registry with flash_target, read_regs, halt, capture, run
- [x] 3.3 JSON responses + error codes
- [x] 3.4 Stateful session: connect/disconnect/halt/step
- [x] 3.5 MCP discovery via GET /mcp tool list

## 4. SSE Streaming
- [x] 4.1 Accept SSE subscription (/sse/capture)
- [x] 4.2 RLE compression util
- [x] 4.3 Push capture buffer when available
- [x] 4.4 Flash progress SSE
- [x] 4.5 Debug regs SSE

## 5. WinBox Web UI
- [x] 5.1 Port PicoLab index.html shell + css + Winbox.js
- [x] 5.2 Flasher dialog (hex upload + progress bar)
- [x] 5.3 Analyzer canvas + decoder dropdowns
- [x] 5.4 Debugger panel (halt/step/run/memory read)
- [x] 5.5 GPIO pin grid
- [x] 5.6 Bus master I2C/SPI panels
- [x] 5.7 Wire buttons to /mcp calls and SSE

## 6. Integration & Verification
- [x] 6.1 End-to-end connect, flash, capture, debug flow
- [x] 6.2 Performance sanity checks
- [x] 6.3 Document change

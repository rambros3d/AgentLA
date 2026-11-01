# Core Split Policy

- Core 0 owns network (lwIP/TinyUSB RNDIS), HTTP, MCP command routing, WebUI.
- Core 1 owns timing-sensitive capture/decode and SWD/flash operations.
- Cross-core communication via shared command queue and job table.
- Large/long operations (FLASH, CAPTURE, MASSERASE) are queued on Core 0 and executed on Core 1 via job table.
- Core 0 retains read-only diagnostics during capture.

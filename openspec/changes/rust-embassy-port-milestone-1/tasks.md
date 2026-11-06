## 1. Worktree & Cargo Project Setup

- [x] 1.1 Create git worktree `../AgentLA-rust` on branch `rust` branched from `dev`
- [x] 1.2 Initialize Cargo package `firmware-rs` with `.cargo/config.toml` targeting `thumbv8m.main-none-eabihf` (RP2350 Cortex-M33)
- [x] 1.3 Configure `Cargo.toml` dependencies (`embassy-rp`, `embassy-executor`, `embassy-time`, `embassy-sync`, `embassy-usb`, `embassy-net`, `picoserve`, `littlefs2`, `defmt`, `defmt-rtt`)
- [x] 1.4 Configure `memory.x` linker script with RP2350 flash and 520 KB SRAM partition boundaries

## 2. Embassy Runtime & Supervisor

- [x] 2.1 Implement `main.rs` initializing RP2350 clocks (150 MHz) and the Embassy executor
- [x] 2.2 Implement hardware watchdog supervisor background task with periodic petting
- [x] 2.3 Verify bare-metal release build compilation with `cargo build --release`

## 3. Composite USB Stack

- [x] 3.1 Define USB device descriptor with AgentLA vendor/product identity
- [x] 3.2 Implement Dual CDC-ACM interfaces (Port 0: CLI / REPL, Port 1: target UART bridge)
- [x] 3.3 Implement CMSIS-DAP v1 (HID) and CMSIS-DAP v2 (Bulk) descriptors with basic DAP_Info responder
- [x] 3.4 Implement CDC-NCM network class interface over `embassy-usb`
- [x] 3.5 Assemble composite builder and verify USB multi-interface enumeration on host

## 4. Virtual Networking & DNS

- [x] 4.1 Initialize `embassy-net` runner over CDC-NCM with static IP `10.10.10.10/24`
- [x] 4.2 Implement async DHCP server task serving leases on `10.10.10.11-13` with gateway `10.10.10.10`
- [x] 4.3 Implement captive DNS responder resolving `agentla.local` queries to `10.10.10.10`
- [x] 4.4 Verify host Ethernet adapter initialization and ping response to `10.10.10.10`

## 5. Storage & Asynchronous Web Server

- [x] 5.1 Implement `littlefs2` block device driver binding `embassy-rp::flash` to on-flash asset partition
- [x] 5.2 Configure `picoserve` HTTP server router to stream static files (`index.html`, `app.js`, `style.css`) from LittleFS
- [x] 5.3 Implement JSON REST endpoints (`/api/v1/system`, `/api/v1/pins`) in `picoserve`
- [x] 5.4 Implement Server-Sent Events (SSE) telemetry stream (`/api/v1/events`)
- [x] 5.5 Verify end-to-end browser loading at `http://agentla.local` and `http://10.10.10.10`

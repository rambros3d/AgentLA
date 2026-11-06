# Credits & Attribution

AgentLA is built upon and inspired by the incredible work of the open-source embedded, Rust, and hardware engineering communities. This document acknowledges the direct dependencies used in our firmware, as well as the foundational projects that served as architectural references during the port to Embassy async Rust.

---

## 1. Attribution: Direct Dependencies

The following libraries and crates are directly compiled into AgentLA firmware (`crates/agentla-bsp`, `boards/*`, and `firmware-rs`):

| Crate / Library | Project / Authors | License | Role in AgentLA |
| :--- | :--- | :--- | :--- |
| **[`embassy-rp`](https://github.com/embassy-rs/embassy)** | Embassy Project Contributors | Apache-2.0 / MIT | RP2040 and RP2350 async Hardware Abstraction Layer (PIO, DMA, Clocks, SAR ADC, PWM, UART, SIO GPIO). |
| **[`embassy-executor`](https://github.com/embassy-rs/embassy)** | Embassy Project Contributors | Apache-2.0 / MIT | Bare-metal async task executor for Cortex-M microcontrollers. |
| **[`embassy-time`](https://github.com/embassy-rs/embassy)** | Embassy Project Contributors | Apache-2.0 / MIT | Microsecond-accurate monotonic hardware timer driver. |
| **[`embassy-sync`](https://github.com/embassy-rs/embassy)** | Embassy Project Contributors | Apache-2.0 / MIT | Zero-allocation asynchronous synchronization primitives (Channels, Mutexes, Signals). |
| **[`embassy-usb`](https://github.com/embassy-rs/embassy)** | Embassy Project Contributors | Apache-2.0 / MIT | Composite USB device stack, descriptors, CDC-ACM serial, CDC-NCM networking, and HID interfaces. |
| **[`embassy-net`](https://github.com/embassy-rs/embassy)** | Embassy Project Contributors | Apache-2.0 / MIT | Async embedded TCP/IP network stack (IPv4, TCP, UDP, ICMP, DHCP server, DNS server). |
| **[`embassy-net-driver-channel`](https://github.com/embassy-rs/embassy)** | Embassy Project Contributors | Apache-2.0 / MIT | Zero-copy channel-based network driver adapter bridging USB CDC-NCM to `embassy-net`. |
| **[`picoserve`](https://github.com/svenstaro/picoserve)** | Sven-Hendrik Haase | MIT / Apache-2.0 | High-performance, low-memory `#![no_std]` HTTP web server handling REST routing, Server-Sent Events (SSE), and static asset streaming. |
| **[`littlefs2`](https://github.com/32blit/littlefs2)** | 32blit Contributors | MIT / Apache-2.0 | Memory-safe idiomatic Rust bindings to ARM LittleFS for power-fail-resilient on-probe flash storage. |
| **[`rp-pac`](https://github.com/rp-rs/rp-pac)** | rp-rs Team | Apache-2.0 / MIT | Low-level Peripheral Access Crate generated from RP2350 SVDs for direct silicon register manipulation. |
| **[`cortex-m`](https://github.com/rust-embedded/cortex-m)** | Rust on Cortex-M Team | MIT / Apache-2.0 | Low-level ARM Cortex-M hardware access, assembly intrinsics, and core peripheral controls. |
| **[`cortex-m-rt`](https://github.com/rust-embedded/cortex-m)** | Rust on Cortex-M Team | MIT / Apache-2.0 | Minimal runtime, exception vectors, and reset startup code for Cortex-M microcontrollers. |
| **[`defmt`](https://github.com/knurling-rs/defmt)** | Ferrous Systems | Apache-2.0 / MIT | Ultra-efficient deferred string formatting logging framework for resource-constrained targets. |
| **[`defmt-rtt`](https://github.com/knurling-rs/defmt)** | Ferrous Systems | Apache-2.0 / MIT | Transport layer transmitting deferred format logs over Segger Real-Time Transfer (RTT). |
| **[`panic-probe`](https://github.com/knurling-rs/panic-probe)** | Ferrous Systems | Apache-2.0 / MIT | Panic handler printing human-readable defmt backtraces and register dumps across debug probes. |
| **[`static_cell`](https://github.com/embassy-rs/static-cell)** | Dario Nieuwenhuis (Dirbaio) | Apache-2.0 / MIT | Safe, zero-cost static storage abstraction for global embedded asynchronous tasks. |
| **[`heapless`](https://github.com/japaric/heapless)** | Jorge Aparicio & Heapless Team | Apache-2.0 / MIT | Static memory, allocation-free data structures (`Vec`, `String`, `Deque`) with compile-time bounded capacity. |
| **[`portable-atomic`](https://github.com/taiki-e/portable-atomic)** | Taiki Endo | Apache-2.0 / MIT | Hardware and software-fallback atomic operations for ARM Cortex-M33 architecture. |
| **[`embedded-io-async`](https://github.com/rust-embedded/embedded-hal)** | Rust Embedded WG | MIT / Apache-2.0 | Asynchronous I/O trait definitions for embedded stream readers, writers, and network sockets. |
| **[`generic-array`](https://github.com/RustCrypto/generic-array)** | RustCrypto Team | MIT | Type-level generic array sizing primitives. |

---

## 2. Credits: Porting References & Architectural Inspiration

The following upstream projects, drivers, and reference implementations were studied and referenced during the architectural design and port of AgentLA from C/Pico-SDK to Embassy async Rust:

| Project / Reference | Author / Organization | Reference Role & Inspiration |
| :--- | :--- | :--- |
| **[Raspberry Pi Debugprobe](https://github.com/raspberrypi/debugprobe)** | Raspberry Pi (Trading) Ltd. & ARM Limited | Reference implementation for high-speed PIO-based ARM SWD transfers (`swd_probe.pio`) and CMSIS-DAP v1/v2 packet dispatching. |
| **[Pico SDK](https://github.com/raspberrypi/pico-sdk)** | Raspberry Pi (Trading) Ltd. | Register specifications, clock tree PLL formulas, DMA channel pacing, and RP2350 Hazard3 / Cortex-M33 errata documentation. |
| **[TinyUSB](https://github.com/hathach/tinyusb)** | Ha Thach & TinyUSB Contributors | USB device descriptors and composite layout design (CDC-ACM, CDC-NCM, HID, WinUSB bulk interfaces). |
| **[lwIP (Lightweight IP)](https://savannah.nongnu.org/projects/lwip/)** | Adam Dunkels & Swedish Institute of Computer Science | Architecture reference for low-memory embedded packet buffer management and web server routing. |
| **[Linux Kernel SocketCAN `gs_usb`](https://github.com/torvalds/linux/blob/master/drivers/net/can/usb/gs_usb.c)** | Bernd Ries & Geschwister Schneider Technologie | Framing formats, bit timing calculations, and control request codes for native host SocketCAN integration. |
| **[Linux Kernel `i2c-tiny-usb`](https://github.com/torvalds/linux/blob/master/drivers/i2c/busses/i2c-tiny-usb.c)** | Till Harbaum | Control requests and transaction protocol (`CMD_I2C_IO`) for native Linux `/dev/i2c-X` driver compatibility. |
| **[ARM CMSIS-DAP / Core](https://github.com/ARM-software/CMSIS_5)** | ARM Limited | Formal specifications for Core Debug registers, DP/AP access protocols, and CMSIS-DAP v1 (HID) & v2 (Bulk) transport standards. |
| **[RVSWD_pico](https://github.com/Nicolai-Electronics/RVSWD_pico)** | Nicolai-Electronics | 2-wire RISC-V debug module protocol (`rvswd.pio`) for WCH QingKe V4 cores (CH32V20x/V30x and CH59x). |
| **[PicoRVD](https://github.com/aappleby/picorvd)** | Andrew Appleby | 1-wire SWIO PIO timing, open-drain turnaround, and PROGBUF memory access routines for WCH QingKe V2 cores (CH32V003). |
| **[gusmanb/logicanalyzer](https://github.com/gusmanb/logicanalyzer)** | Bernardo Guasconi | High-speed PIO digital capture pipelines, DMA circular buffering, and trigger qualifier architecture. |
| **[lrndis](https://github.com/fetisov/lrndis)** | Sergey Fetisov | Packet structures for embedded DHCP and captive DNS packet formatting. |
| **[DaisyUI & Tailwind CSS](https://github.com/saadeghi/daisyui)** | Pouya Saadeghi & Tailwind Labs | Component library and styling powering the embedded Web Desktop OS. |

---

## 3. Acknowledgements

Special thanks to the **Embassy Project** developers for creating a transformative async runtime for embedded microcontrollers, and the **Raspberry Pi silicon team** for the RP2350 architecture.

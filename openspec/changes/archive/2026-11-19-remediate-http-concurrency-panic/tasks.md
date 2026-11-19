## 1. Concurrency and Memory Optimization

- [x] 1.1 In `firmware-rs/src/web.rs`, reduce the worker sockets from 8 down to 2 concurrent workers (`server_0`, `server_1`).
- [x] 1.2 Remove buffers `_2` through `_7` (`http_buf_2..7`, `tcp_rx_buf_2..7`, `tcp_tx_buf_2..7`) and replace the nested `join(join4, join4)` with `embassy_futures::join::join(server_0, server_1)`.
- [x] 1.3 Update the defmt log message to reflect 2 concurrent worker sockets.
- [x] 1.4 Verify with `nm -S --size-sort` that `web_server_task::POOL` reduces from 324.8 KiB down to < 85 KiB.

## 2. Request Handling Hardening

- [x] 2.1 Audit `HeadersIter::next` usage and header iteration in `firmware-rs/src/fs_http.rs` to ensure slice indexing never exceeds slice lengths on incoming requests.
- [x] 2.2 Rebuild the `firmware-rs` release binary with zero warnings and verify memory layout.

## 3. Hardware Flash and Verification

- [x] 3.1 Flash the updated binary to the RP2350 target using `probe-rs download` or `probe-rs run`.
- [x] 3.2 Verify live ping and check that `usb0` interface remains stable with zero reset cycles.
- [x] 3.3 Execute HTTP GET requests against `http://10.10.10.10/` and `http://10.10.10.10/api/v1/system` via `curl` and verify HTTP 200/302 responses.
- [x] 3.4 Execute HTTP GET request against `http://10.10.10.10/barefoc` and verify BareFOC Studio HTML delivery.
- [x] 3.5 Run the test suite (`test_barefoc_route.py`, `test_barefoc_parity.py`) to confirm zero regressions.

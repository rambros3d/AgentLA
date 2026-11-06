use core::fmt::Write as _;
use embassy_net::tcp::TcpSocket;
use embassy_net::Stack;
use embedded_io_async::Write as _;

use crate::programmer::swd::SwdTransport;
use crate::programmer::target::cortex_m::{
    CortexError, CortexM, DHCSR, DHCSR_C_DEBUGEN, DHCSR_C_STEP, DHCSR_DBGKEY,
    DHCSR_S_HALT,
};

// CoreSight FPB registers
pub const FP_CTRL: u32 = 0xE0002000;
pub const FP_COMP0: u32 = 0xE0002008;
pub const FP_COMP_COUNT: usize = 6;

pub fn calculate_checksum(data: &[u8]) -> u8 {
    data.iter().fold(0u8, |acc, &b| acc.wrapping_add(b))
}

pub fn hex_to_u32(s: &str) -> Option<u32> {
    u32::from_str_radix(s, 16).ok()
}

pub fn hex_to_bytes(hex: &str, out: &mut [u8]) -> usize {
    let bytes = hex.as_bytes();
    let mut count = 0;
    for i in (0..bytes.len()).step_by(2) {
        if i + 1 >= bytes.len() || count >= out.len() {
            break;
        }
        let hi = match bytes[i] {
            b'0'..=b'9' => bytes[i] - b'0',
            b'a'..=b'f' => bytes[i] - b'a' + 10,
            b'A'..=b'F' => bytes[i] - b'A' + 10,
            _ => return count,
        };
        let lo = match bytes[i + 1] {
            b'0'..=b'9' => bytes[i + 1] - b'0',
            b'a'..=b'f' => bytes[i + 1] - b'a' + 10,
            b'A'..=b'F' => bytes[i + 1] - b'A' + 10,
            _ => return count,
        };
        out[count] = (hi << 4) | lo;
        count += 1;
    }
    count
}

pub fn bytes_to_hex(bytes: &[u8], out: &mut heapless::String<512>) {
    const HEX_CHARS: &[u8; 16] = b"0123456789abcdef";
    for &b in bytes {
        if out.len() + 2 <= out.capacity() {
            let _ = out.push(HEX_CHARS[(b >> 4) as usize] as char);
            let _ = out.push(HEX_CHARS[(b & 0x0F) as usize] as char);
        }
    }
}

pub fn format_rsp(payload: &str, out: &mut heapless::String<512>) {
    let csum = calculate_checksum(payload.as_bytes());
    out.clear();
    let _ = write!(out, "${}#{:02x}", payload, csum);
}

// Hardware Breakpoints via Cortex-M FPB (Flash Patch and Breakpoint Unit)
pub fn enable_fpb(cortex: &mut CortexM) -> Result<(), CortexError> {
    // Write KEY=1 (bit 1) and ENABLE=1 (bit 0) to FP_CTRL
    cortex.mem_write32(FP_CTRL, 0x03)?;
    Ok(())
}

pub fn insert_hw_breakpoint(cortex: &mut CortexM, addr: u32) -> Result<(), CortexError> {
    enable_fpb(cortex)?;
    for i in 0..FP_COMP_COUNT {
        let comp_addr = FP_COMP0 + (i as u32 * 4);
        let val = cortex.mem_read32(comp_addr)?;
        if (val & 1) == 0 {
            // Unused slot found.
            // Bits [28:2] = Address[28:2]
            // Bit 0 = ENABLE
            // Bits [31:30] = Replace: 1 = lower halfword (bits 1:0 = 00), 2 = upper halfword (bits 1:0 = 10)
            let replace = if (addr & 2) != 0 { 2 << 30 } else { 1 << 30 };
            let comp_val = (addr & 0x1FFFFFFC) | replace | 1;
            cortex.mem_write32(comp_addr, comp_val)?;
            return Ok(());
        }
    }
    Err(CortexError::Timeout) // No free breakpoint registers
}

pub fn remove_hw_breakpoint(cortex: &mut CortexM, addr: u32) -> Result<(), CortexError> {
    for i in 0..FP_COMP_COUNT {
        let comp_addr = FP_COMP0 + (i as u32 * 4);
        let val = cortex.mem_read32(comp_addr)?;
        if (val & 1) != 0 && (val & 0x1FFFFFFC) == (addr & 0x1FFFFFFC) {
            cortex.mem_write32(comp_addr, 0)?;
            return Ok(());
        }
    }
    Ok(())
}

pub fn handle_gdb_command(cmd: &str, cortex: &mut CortexM, resp: &mut heapless::String<512>) {
    if cmd.is_empty() {
        format_rsp("", resp);
        return;
    }

    match cmd.as_bytes()[0] {
        b'?' => {
            // Query halt reason
            let halted = cortex.is_halted().unwrap_or(false);
            if halted {
                format_rsp("S05", resp); // SIGTRAP
            } else {
                format_rsp("OK", resp);
            }
        }

        b'g' => {
            // Read general registers R0-R15 + xPSR (17 registers x 4 bytes = 68 bytes)
            let mut reg_hex = heapless::String::<512>::new();
            for r in 0..=16 {
                let val = cortex.read_core_reg(r as u8).unwrap_or(0);
                let le = val.to_le_bytes();
                bytes_to_hex(&le, &mut reg_hex);
            }
            format_rsp(reg_hex.as_str(), resp);
        }

        b'G' => {
            // Write general registers
            let hex_data = &cmd[1..];
            let mut raw = [0u8; 68];
            let n = hex_to_bytes(hex_data, &mut raw);
            for r in 0..(n / 4).min(17) {
                let val = u32::from_le_bytes([
                    raw[r * 4],
                    raw[r * 4 + 1],
                    raw[r * 4 + 2],
                    raw[r * 4 + 3],
                ]);
                let _ = cortex.write_core_reg(r as u8, val);
            }
            format_rsp("OK", resp);
        }

        b'm' => {
            // Read memory: m<addr>,<length>
            let rest = &cmd[1..];
            if let Some((addr_str, len_str)) = rest.split_once(',') {
                if let (Some(addr), Some(len)) = (hex_to_u32(addr_str), hex_to_u32(len_str)) {
                    let len = len as usize;
                    let mut mem_buf = [0u8; 128];
                    let to_read = len.min(mem_buf.len());
                    if cortex.mem_read_block(addr, &mut mem_buf[..to_read]).is_ok() {
                        let mut hex_resp = heapless::String::<512>::new();
                        bytes_to_hex(&mem_buf[..to_read], &mut hex_resp);
                        format_rsp(hex_resp.as_str(), resp);
                        return;
                    }
                }
            }
            format_rsp("E01", resp);
        }

        b'M' => {
            // Write memory: M<addr>,<length>:<hex_data>
            let rest = &cmd[1..];
            if let Some((addr_len, hex_payload)) = rest.split_once(':') {
                if let Some((addr_str, len_str)) = addr_len.split_once(',') {
                    if let (Some(addr), Some(len)) = (hex_to_u32(addr_str), hex_to_u32(len_str)) {
                        let len = len as usize;
                        let mut mem_buf = [0u8; 128];
                        let decoded = hex_to_bytes(hex_payload, &mut mem_buf[..len]);
                        if cortex.mem_write_block(addr, &mem_buf[..decoded]).is_ok() {
                            format_rsp("OK", resp);
                            return;
                        }
                    }
                }
            }
            format_rsp("E01", resp);
        }

        b's' => {
            // Single step
            let _ = cortex.mem_write32(
                DHCSR,
                DHCSR_DBGKEY | DHCSR_C_DEBUGEN | DHCSR_C_STEP,
            );
            // Poll for halt
            for _ in 0..1000 {
                if let Ok(dhcsr) = cortex.mem_read32(DHCSR) {
                    if (dhcsr & DHCSR_S_HALT) != 0 {
                        break;
                    }
                }
                cortex_m::asm::delay(1500);
            }
            format_rsp("S05", resp);
        }

        b'c' => {
            // Continue execution
            let _ = cortex.resume();
            format_rsp("S05", resp);
        }

        b'Z' => {
            // Insert breakpoint: Z0,<addr>,<kind>
            if cmd.starts_with("Z0,") {
                let rest = &cmd[3..];
                let addr_str = rest.split(',').next().unwrap_or("");
                if let Some(addr) = hex_to_u32(addr_str) {
                    if insert_hw_breakpoint(cortex, addr).is_ok() {
                        format_rsp("OK", resp);
                        return;
                    }
                }
                format_rsp("E01", resp);
            } else {
                format_rsp("", resp); // Unsupported breakpoint type
            }
        }

        b'z' => {
            // Remove breakpoint: z0,<addr>,<kind>
            if cmd.starts_with("z0,") {
                let rest = &cmd[3..];
                let addr_str = rest.split(',').next().unwrap_or("");
                if let Some(addr) = hex_to_u32(addr_str) {
                    let _ = remove_hw_breakpoint(cortex, addr);
                    format_rsp("OK", resp);
                    return;
                }
                format_rsp("E01", resp);
            } else {
                format_rsp("", resp);
            }
        }

        b'q' => {
            if cmd.starts_with("qSupported") {
                format_rsp("PacketSize=512;qXfer:memory-map:read+;hwbreak+", resp);
            } else if cmd.starts_with("qAttached") {
                format_rsp("1", resp);
            } else {
                format_rsp("", resp);
            }
        }

        b'v' => {
            if cmd == "vCont?" {
                format_rsp("vCont;c;s", resp);
            } else if cmd.starts_with("vCont;c") {
                let _ = cortex.resume();
                format_rsp("S05", resp);
            } else if cmd.starts_with("vCont;s") {
                let _ = cortex.mem_write32(
                    DHCSR,
                    DHCSR_DBGKEY | DHCSR_C_DEBUGEN | DHCSR_C_STEP,
                );
                format_rsp("S05", resp);
            } else {
                format_rsp("", resp);
            }
        }

        b'k' | b'D' => {
            // Kill / Detach: resume core
            let _ = cortex.resume();
            format_rsp("OK", resp);
        }

        _ => {
            // Unrecognized command
            format_rsp("", resp);
        }
    }
}

#[embassy_executor::task]
pub async fn gdb_server_task(stack: Stack<'static>) {
    let mut rx_buf = [0u8; 1024];
    let mut tx_buf = [0u8; 1024];

    loop {
        let mut socket = TcpSocket::new(stack, &mut rx_buf, &mut tx_buf);
        socket.set_timeout(Some(embassy_time::Duration::from_secs(120)));

        if let Err(e) = socket.accept(3333).await {
            defmt::error!("GDB TCP accept error: {:?}", e);
            embassy_time::Timer::after(embassy_time::Duration::from_millis(200)).await;
            continue;
        }

        defmt::info!("GDB client connected on 10.10.10.10:3333");

        let mut swd = SwdTransport::new();
        swd.init(4000);
        swd.jtag_to_swd();
        let mut dpidr = 0u32;
        let _ = swd.read_dpidr_retry(&mut dpidr);
        let _ = swd.power_up_debug();

        let mut cortex = CortexM::new(&mut swd);
        let mut in_buf = [0u8; 512];
        let mut resp_str = heapless::String::<512>::new();

        loop {
            match socket.read(&mut in_buf).await {
                Ok(0) => break, // Disconnected
                Ok(n) => {
                    let req_slice = &in_buf[..n];
                    let mut i = 0;
                    while i < req_slice.len() {
                        let b = req_slice[i];
                        if b == 0x03 {
                            // Interrupt (Ctrl-C)
                            let _ = cortex.halt();
                            let _ = socket.write_all(b"$S05#b8").await;
                            i += 1;
                        } else if b == b'+' || b == b'-' {
                            // ACK / NACK from host
                            i += 1;
                        } else if b == b'$' {
                            // Start of RSP packet: find '#'
                            if let Some(hash_pos) = req_slice[i..].iter().position(|&x| x == b'#') {
                                let payload_start = i + 1;
                                let payload_end = i + hash_pos;
                                let csum_end = payload_end + 3;
                                if csum_end <= req_slice.len() {
                                    if let Ok(payload_str) =
                                        core::str::from_utf8(&req_slice[payload_start..payload_end])
                                    {
                                        // Send ACK '+'
                                        let _ = socket.write_all(b"+").await;
                                        handle_gdb_command(payload_str, &mut cortex, &mut resp_str);
                                        let _ = socket.write_all(resp_str.as_bytes()).await;
                                    }
                                    i = csum_end;
                                } else {
                                    break;
                                }
                            } else {
                                break;
                            }
                        } else {
                            i += 1;
                        }
                    }
                }
                Err(_) => break,
            }
        }

        // On client disconnect, safely resume target core
        let _ = cortex.resume();
        defmt::info!("GDB client disconnected from :3333");
    }
}

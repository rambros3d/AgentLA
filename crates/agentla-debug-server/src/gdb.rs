//! GDB Remote Serial Protocol (RSP) packet framing, checksums, and stub dispatcher.

use core::fmt::Write as _;
use embedded_io_async::{Read, Write};
use heapless::String;

pub use crate::target::{BreakpointError, GdbTarget, MemoryAccess};

// CoreSight FPB registers
pub const FP_CTRL: u32 = 0xE0002000;
pub const FP_COMP0: u32 = 0xE0002008;
pub const FP_COMP_COUNT: usize = 6;

/// Calculate 8-bit wrapping checksum of byte slice for GDB RSP packet.
pub fn calculate_checksum(data: &[u8]) -> u8 {
    data.iter().fold(0u8, |acc, &b| acc.wrapping_add(b))
}

/// Convert hex string to u32 value.
pub fn hex_to_u32(s: &str) -> Option<u32> {
    u32::from_str_radix(s, 16).ok()
}

/// Decode hex-encoded ASCII string into raw bytes.
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

/// Encode raw bytes into lower-case hex characters.
pub fn bytes_to_hex(bytes: &[u8], out: &mut String<512>) {
    const HEX_CHARS: &[u8; 16] = b"0123456789abcdef";
    for &b in bytes {
        if out.len() + 2 <= out.capacity() {
            let _ = out.push(HEX_CHARS[(b >> 4) as usize] as char);
            let _ = out.push(HEX_CHARS[(b & 0x0F) as usize] as char);
        }
    }
}

/// Format GDB RSP packet: `$payload#checksum`.
pub fn format_rsp(payload: &str, out: &mut String<512>) {
    let csum = calculate_checksum(payload.as_bytes());
    out.clear();
    let _ = write!(out, "${}#{:02x}", payload, csum);
}

/// Enable ARM CoreSight Flash Patch and Breakpoint (FPB) unit.
pub fn enable_fpb<M: MemoryAccess + ?Sized>(target: &mut M) -> Result<(), BreakpointError> {
    target.mem_write32(FP_CTRL, 0x03).map_err(|_| BreakpointError::TargetError)
}

/// Insert hardware breakpoint into CoreSight FPB.
pub fn insert_hw_breakpoint<M: MemoryAccess + ?Sized>(target: &mut M, addr: u32) -> Result<(), BreakpointError> {
    enable_fpb(target)?;
    for i in 0..FP_COMP_COUNT {
        let comp_addr = FP_COMP0 + (i as u32 * 4);
        let val = target.mem_read32(comp_addr).map_err(|_| BreakpointError::TargetError)?;
        if (val & 1) == 0 {
            // Unused slot found.
            // Bits [28:2] = Address[28:2]
            // Bit 0 = ENABLE
            // Bits [31:30] = Replace: 1 = lower halfword (bits 1:0 = 00), 2 = upper halfword (bits 1:0 = 10)
            let replace = if (addr & 2) != 0 { 2 << 30 } else { 1 << 30 };
            let comp_val = (addr & 0x1FFFFFFC) | replace | 1;
            target.mem_write32(comp_addr, comp_val).map_err(|_| BreakpointError::TargetError)?;
            return Ok(());
        }
    }
    Err(BreakpointError::NoFreeSlot)
}

/// Remove hardware breakpoint from CoreSight FPB.
pub fn remove_hw_breakpoint<M: MemoryAccess + ?Sized>(target: &mut M, addr: u32) -> Result<(), BreakpointError> {
    for i in 0..FP_COMP_COUNT {
        let comp_addr = FP_COMP0 + (i as u32 * 4);
        let val = target.mem_read32(comp_addr).map_err(|_| BreakpointError::TargetError)?;
        if (val & 1) != 0 && (val & 0x1FFFFFFC) == (addr & 0x1FFFFFFC) {
            target.mem_write32(comp_addr, 0).map_err(|_| BreakpointError::TargetError)?;
            return Ok(());
        }
    }
    Ok(())
}

/// Dispatch and execute a single GDB RSP command, generating the response packet.
pub fn handle_gdb_command<T: GdbTarget + ?Sized>(
    cmd: &str,
    target: &mut T,
    resp: &mut String<512>,
) {
    if cmd.is_empty() {
        format_rsp("", resp);
        return;
    }

    match cmd.as_bytes()[0] {
        b'?' => {
            // Query halt reason
            let halted = target.is_halted().unwrap_or(false);
            if halted {
                format_rsp("S05", resp); // SIGTRAP
            } else {
                format_rsp("OK", resp);
            }
        }

        b'g' => {
            // Read general registers R0-R15 + xPSR (17 registers x 4 bytes = 68 bytes)
            let mut reg_hex = String::<512>::new();
            for r in 0..=16 {
                let val = target.read_core_reg(r as u8).unwrap_or(0);
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
                let _ = target.write_core_reg(r as u8, val);
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
                    if target.mem_read_block(addr, &mut mem_buf[..to_read]).is_ok() {
                        let mut hex_resp = String::<512>::new();
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
                        if target.mem_write_block(addr, &mem_buf[..decoded]).is_ok() {
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
            let _ = target.step();
            format_rsp("S05", resp);
        }

        b'c' => {
            // Continue execution
            let _ = target.resume();
            format_rsp("S05", resp);
        }

        b'Z' => {
            // Insert breakpoint: Z0,<addr>,<kind>
            if cmd.starts_with("Z0,") {
                let rest = &cmd[3..];
                let addr_str = rest.split(',').next().unwrap_or("");
                if let Some(addr) = hex_to_u32(addr_str) {
                    if target.insert_breakpoint(addr).is_ok() {
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
                    let _ = target.remove_breakpoint(addr);
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
                let _ = target.resume();
                format_rsp("S05", resp);
            } else if cmd.starts_with("vCont;s") {
                let _ = target.step();
                format_rsp("S05", resp);
            } else {
                format_rsp("", resp);
            }
        }

        b'k' | b'D' => {
            // Kill / Detach: resume core
            let _ = target.resume();
            format_rsp("OK", resp);
        }

        _ => {
            // Unrecognized command
            format_rsp("", resp);
        }
    }
}

/// Run an asynchronous GDB RSP protocol session over any `embedded_io_async::{Read, Write}` stream.
pub async fn run_gdb_session<I, T>(io: &mut I, target: &mut T) -> Result<(), ()>
where
    I: Read + Write + ?Sized,
    T: GdbTarget + ?Sized,
{
    let mut buf = [0u8; 512];
    let mut valid_len = 0usize;
    let mut resp_str = String::<512>::new();

    loop {
        let n = match io.read(&mut buf[valid_len..]).await {
            Ok(0) => break, // Disconnected / EOF
            Ok(n) => n,
            Err(_) => break,
        };
        valid_len += n;

        let mut i = 0;
        while i < valid_len {
            let b = buf[i];
            if b == 0x03 {
                // Interrupt (Ctrl-C)
                let _ = target.halt();
                let _ = io.write_all(b"$S05#b8").await;
                i += 1;
            } else if b == b'+' || b == b'-' {
                // ACK / NACK from host
                i += 1;
            } else if b == b'$' {
                // Start of RSP packet: search for '#'
                if let Some(hash_rel) = buf[i..valid_len].iter().position(|&x| x == b'#') {
                    let hash_pos = i + hash_rel;
                    let csum_end = hash_pos + 3;
                    if csum_end <= valid_len {
                        let payload = &buf[i + 1..hash_pos];
                        let expected_csum_bytes = &buf[hash_pos + 1..csum_end];
                        let calc_csum = calculate_checksum(payload);

                        let csum_valid = if let Ok(csum_str) = core::str::from_utf8(expected_csum_bytes) {
                            u8::from_str_radix(csum_str, 16).map(|v| v == calc_csum).unwrap_or(false)
                        } else {
                            false
                        };

                        if csum_valid {
                            let _ = io.write_all(b"+").await;
                            if let Ok(payload_str) = core::str::from_utf8(payload) {
                                handle_gdb_command(payload_str, target, &mut resp_str);
                                let _ = io.write_all(resp_str.as_bytes()).await;
                            }
                        } else {
                            let _ = io.write_all(b"-").await;
                        }
                        i = csum_end;
                    } else {
                        // Incomplete checksum, wait for more data
                        break;
                    }
                } else {
                    // Incomplete packet payload, wait for more data
                    break;
                }
            } else {
                // Ignore unexpected / whitespace byte
                i += 1;
            }
        }

        if i < valid_len {
            buf.copy_within(i..valid_len, 0);
            valid_len -= i;
        } else {
            valid_len = 0;
        }

        if valid_len == buf.len() {
            // Buffer full with unparseable data, clear to recover
            valid_len = 0;
        }
    }

    Ok(())
}

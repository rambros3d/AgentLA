use agentla_debug_server::gdb::{
    bytes_to_hex, calculate_checksum, format_rsp, handle_gdb_command, hex_to_bytes, hex_to_u32,
    run_gdb_session, BreakpointError, GdbTarget, MemoryAccess,
};
use embedded_io_async::{ErrorType, Read, Write};
use heapless::String;
use std::collections::HashMap;
use std::future::Future;
use std::pin::pin;
use std::task::{Context, Poll, Waker};

fn block_on<F: Future>(f: F) -> F::Output {
    let mut f = pin!(f);
    let waker = Waker::noop();
    let mut cx = Context::from_waker(&waker);
    loop {
        match f.as_mut().poll(&mut cx) {
            Poll::Ready(val) => return val,
            Poll::Pending => panic!("Future remained pending"),
        }
    }
}

struct MockTarget {
    regs: [u32; 17],
    halted: bool,
    memory: HashMap<u32, u8>,
    breakpoints: [Option<u32>; 6],
}

impl MockTarget {
    fn new() -> Self {
        let mut regs = [0u32; 17];
        regs[13] = 0x20002000; // SP
        regs[14] = 0x08000101; // LR
        regs[15] = 0x08000250; // PC
        regs[16] = 0x01000000; // xPSR

        Self {
            regs,
            halted: true,
            memory: HashMap::new(),
            breakpoints: [None; 6],
        }
    }
}

impl MemoryAccess for MockTarget {
    type Error = core::convert::Infallible;

    fn mem_read32(&mut self, addr: u32) -> Result<u32, Self::Error> {
        let mut b = [0u8; 4];
        let _ = self.mem_read_block(addr, &mut b);
        Ok(u32::from_le_bytes(b))
    }

    fn mem_write32(&mut self, addr: u32, val: u32) -> Result<(), Self::Error> {
        self.mem_write_block(addr, &val.to_le_bytes())
    }

    fn mem_read_block(&mut self, addr: u32, buf: &mut [u8]) -> Result<(), Self::Error> {
        for (i, b) in buf.iter_mut().enumerate() {
            *b = *self.memory.get(&(addr + i as u32)).unwrap_or(&0);
        }
        Ok(())
    }

    fn mem_write_block(&mut self, addr: u32, data: &[u8]) -> Result<(), Self::Error> {
        for (i, &b) in data.iter().enumerate() {
            self.memory.insert(addr + i as u32, b);
        }
        Ok(())
    }
}

impl GdbTarget for MockTarget {
    fn is_halted(&mut self) -> Result<bool, Self::Error> {
        Ok(self.halted)
    }

    fn halt(&mut self) -> Result<(), Self::Error> {
        self.halted = true;
        Ok(())
    }

    fn resume(&mut self) -> Result<(), Self::Error> {
        self.halted = false;
        Ok(())
    }

    fn step(&mut self) -> Result<(), Self::Error> {
        self.halted = true;
        self.regs[15] += 2; // advance PC by 2
        Ok(())
    }

    fn read_core_reg(&mut self, reg: u8) -> Result<u32, Self::Error> {
        if (reg as usize) < self.regs.len() {
            Ok(self.regs[reg as usize])
        } else {
            Ok(0)
        }
    }

    fn write_core_reg(&mut self, reg: u8, val: u32) -> Result<(), Self::Error> {
        if (reg as usize) < self.regs.len() {
            self.regs[reg as usize] = val;
        }
        Ok(())
    }

    fn insert_breakpoint(&mut self, addr: u32) -> Result<(), BreakpointError> {
        for slot in &mut self.breakpoints {
            if slot.is_none() {
                *slot = Some(addr);
                return Ok(());
            }
        }
        Err(BreakpointError::NoFreeSlot)
    }

    fn remove_breakpoint(&mut self, addr: u32) -> Result<(), BreakpointError> {
        for slot in &mut self.breakpoints {
            if *slot == Some(addr) {
                *slot = None;
                return Ok(());
            }
        }
        Ok(())
    }
}

struct MockStream {
    rx_data: Vec<u8>,
    rx_pos: usize,
    chunk_size: Option<usize>,
    tx_data: Vec<u8>,
}

impl MockStream {
    fn new(input: &[u8]) -> Self {
        Self {
            rx_data: input.to_vec(),
            rx_pos: 0,
            chunk_size: None,
            tx_data: Vec::new(),
        }
    }

    fn with_chunk_size(input: &[u8], chunk_size: usize) -> Self {
        Self {
            rx_data: input.to_vec(),
            rx_pos: 0,
            chunk_size: Some(chunk_size),
            tx_data: Vec::new(),
        }
    }
}

impl ErrorType for MockStream {
    type Error = core::convert::Infallible;
}

impl Read for MockStream {
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        if self.rx_pos >= self.rx_data.len() {
            return Ok(0); // EOF
        }
        let rem = &self.rx_data[self.rx_pos..];
        let mut max_read = rem.len().min(buf.len());
        if let Some(cs) = self.chunk_size {
            max_read = max_read.min(cs);
        }
        buf[..max_read].copy_from_slice(&rem[..max_read]);
        self.rx_pos += max_read;
        Ok(max_read)
    }
}

impl Write for MockStream {
    async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        self.tx_data.extend_from_slice(buf);
        Ok(buf.len())
    }

    async fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[test]
fn test_checksum_calculation() {
    assert_eq!(calculate_checksum(b""), 0);
    assert_eq!(calculate_checksum(b"qSupported"), 0x37);
    assert_eq!(calculate_checksum(b"S05"), 0xb8);
    assert_eq!(calculate_checksum(b"OK"), 0x9a);
    assert_eq!(calculate_checksum(b"?"), 0x3f);
}

#[test]
fn test_hex_conversions() {
    assert_eq!(hex_to_u32("1234"), Some(0x1234));
    assert_eq!(hex_to_u32("20000000"), Some(0x20000000));
    assert_eq!(hex_to_u32("deadbeef"), Some(0xdeadbeef));
    assert_eq!(hex_to_u32("invalid"), None);

    let mut buf = [0u8; 4];
    let n = hex_to_bytes("01020304", &mut buf);
    assert_eq!(n, 4);
    assert_eq!(buf, [1, 2, 3, 4]);

    let mut hex_str = String::<512>::new();
    bytes_to_hex(&[0x12, 0x34, 0xab, 0xcd], &mut hex_str);
    assert_eq!(hex_str.as_str(), "1234abcd");
}

#[test]
fn test_format_rsp() {
    let mut resp = String::<512>::new();
    format_rsp("OK", &mut resp);
    assert_eq!(resp.as_str(), "$OK#9a");

    format_rsp("S05", &mut resp);
    assert_eq!(resp.as_str(), "$S05#b8");

    format_rsp("", &mut resp);
    assert_eq!(resp.as_str(), "$#00");
}

#[test]
fn test_command_handling() {
    let mut target = MockTarget::new();
    let mut resp = String::<512>::new();

    // Query halt reason
    handle_gdb_command("?", &mut target, &mut resp);
    assert_eq!(resp.as_str(), "$S05#b8");

    // qSupported
    handle_gdb_command("qSupported", &mut target, &mut resp);
    assert!(resp.starts_with("$PacketSize="));
    assert!(resp.contains("hwbreak+"));

    // qAttached
    handle_gdb_command("qAttached", &mut target, &mut resp);
    assert_eq!(resp.as_str(), "$1#31");

    // vCont?
    handle_gdb_command("vCont?", &mut target, &mut resp);
    assert_eq!(resp.as_str(), "$vCont;c;s#56");

    // Read registers 'g'
    handle_gdb_command("g", &mut target, &mut resp);
    // 17 regs * 8 hex chars = 136 chars + 4 prefix/csum = 140
    assert!(resp.starts_with("$"));
    assert_eq!(resp.len(), 1 + 136 + 3);

    // Write registers 'G'
    let mut new_regs_hex = String::<512>::new();
    for i in 0..17u32 {
        bytes_to_hex(&i.to_le_bytes(), &mut new_regs_hex);
    }
    let g_cmd = std::format!("G{}", new_regs_hex);
    handle_gdb_command(&g_cmd, &mut target, &mut resp);
    assert_eq!(resp.as_str(), "$OK#9a");
    assert_eq!(target.regs[0], 0);
    assert_eq!(target.regs[5], 5);
    assert_eq!(target.regs[16], 16);

    // Write memory 'M'
    handle_gdb_command("M20000000,4:01020304", &mut target, &mut resp);
    assert_eq!(resp.as_str(), "$OK#9a");

    // Read memory 'm'
    handle_gdb_command("m20000000,4", &mut target, &mut resp);
    assert_eq!(resp.as_str(), "$01020304#8a");

    // Breakpoints
    handle_gdb_command("Z0,08000100,2", &mut target, &mut resp);
    assert_eq!(resp.as_str(), "$OK#9a");
    assert_eq!(target.breakpoints[0], Some(0x08000100));

    handle_gdb_command("z0,08000100,2", &mut target, &mut resp);
    assert_eq!(resp.as_str(), "$OK#9a");
    assert_eq!(target.breakpoints[0], None);

    // Single step
    handle_gdb_command("s", &mut target, &mut resp);
    assert_eq!(resp.as_str(), "$S05#b8");

    // Continue
    handle_gdb_command("c", &mut target, &mut resp);
    assert_eq!(resp.as_str(), "$S05#b8");

    // Kill / Detach
    handle_gdb_command("k", &mut target, &mut resp);
    assert_eq!(resp.as_str(), "$OK#9a");
    assert_eq!(target.halted, false);
}

#[test]
fn test_async_gdb_session() {
    let mut target = MockTarget::new();
    // Simulate host sending $?#3f
    let input = b"$?#3f";
    let mut stream = MockStream::new(input);

    block_on(async {
        run_gdb_session(&mut stream, &mut target).await.unwrap();
    });

    // Output should contain '+' followed by $S05#b8
    assert_eq!(stream.tx_data, b"+$S05#b8");
}

#[test]
fn test_async_gdb_session_multiple_commands() {
    let mut target = MockTarget::new();
    let mut input = Vec::new();
    input.extend_from_slice(b"+$qSupported#37");
    input.extend_from_slice(b"+$?#3f");
    input.push(0x03); // Ctrl-C
    let mut stream = MockStream::new(&input);

    block_on(async {
        run_gdb_session(&mut stream, &mut target).await.unwrap();
    });

    let tx_str = std::str::from_utf8(&stream.tx_data).unwrap();
    assert!(tx_str.contains("+$PacketSize=512;"));
    assert!(tx_str.contains("+$S05#b8"));
    assert!(tx_str.contains("$S05#b8")); // from 0x03 interrupt
}

#[test]
fn test_async_gdb_session_fragmented_stream() {
    let mut target = MockTarget::new();
    let mut pkt = String::<512>::new();
    format_rsp("m20000000,4", &mut pkt);
    let input = pkt.as_bytes();
    // Feed in tiny 2-byte chunks to stress buffer reconstruction
    let mut stream = MockStream::with_chunk_size(input, 2);

    block_on(async {
        run_gdb_session(&mut stream, &mut target).await.unwrap();
    });

    assert!(stream.tx_data.starts_with(b"+$"));
}

#[test]
fn test_async_gdb_checksum_mismatch() {
    let mut target = MockTarget::new();
    // Bad checksum: expected 3f, give 00
    let input = b"$?#00";
    let mut stream = MockStream::new(input);

    block_on(async {
        run_gdb_session(&mut stream, &mut target).await.unwrap();
    });

    // Should return '-' NACK
    assert_eq!(stream.tx_data, b"-");
}

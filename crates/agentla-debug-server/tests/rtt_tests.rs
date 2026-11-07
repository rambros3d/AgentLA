use agentla_debug_server::rtt::{
    drain_rtt_up, format_rtt_status_json, scan_rtt, RttBufferDesc, RttControlBlock,
    RttEngineState, RttError,
};
use agentla_debug_server::target::MemoryAccess;
use heapless::String;
use std::collections::HashMap;

struct MockRam {
    memory: HashMap<u32, u8>,
}

impl MockRam {
    fn new() -> Self {
        Self {
            memory: HashMap::new(),
        }
    }
}

impl MemoryAccess for MockRam {
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

fn create_rtt_header(
    buffer_ptr: u32,
    size: u32,
    write_offset: u32,
    read_offset: u32,
) -> [u8; 48] {
    let mut hdr = [0u8; 48];
    // "SEGGER RTT\0\0\0\0\0\0"
    hdr[..16].copy_from_slice(b"SEGGER RTT\0\0\0\0\0\0");
    // max_num_up_buffers = 1
    hdr[16..20].copy_from_slice(&1u32.to_le_bytes());
    // max_num_down_buffers = 1
    hdr[20..24].copy_from_slice(&1u32.to_le_bytes());
    // name_ptr = 0
    hdr[24..28].copy_from_slice(&0u32.to_le_bytes());
    // buffer_ptr
    hdr[28..32].copy_from_slice(&buffer_ptr.to_le_bytes());
    // size
    hdr[32..36].copy_from_slice(&size.to_le_bytes());
    // write_offset
    hdr[36..40].copy_from_slice(&write_offset.to_le_bytes());
    // read_offset
    hdr[40..44].copy_from_slice(&read_offset.to_le_bytes());
    // flags = 0
    hdr[44..48].copy_from_slice(&0u32.to_le_bytes());
    hdr
}

#[test]
fn test_parse_rtt_control_block() {
    let hdr = create_rtt_header(0x20001000, 1024, 12, 4);
    let cb = RttControlBlock::parse_from_slice(0x20000400, &hdr).expect("Failed to parse valid CB");

    assert_eq!(cb.cb_addr, 0x20000400);
    assert_eq!(cb.max_num_up_buffers, 1);
    assert_eq!(cb.max_num_down_buffers, 1);
    assert_eq!(cb.up_buffer.buffer_ptr, 0x20001000);
    assert_eq!(cb.up_buffer.size, 1024);
    assert_eq!(cb.up_buffer.write_offset, 12);
    assert_eq!(cb.up_buffer.read_offset, 4);

    // Invalid signature
    let mut bad_hdr = hdr;
    bad_hdr[0..6].copy_from_slice(b"NOTRTT");
    assert!(RttControlBlock::parse_from_slice(0x20000400, &bad_hdr).is_none());

    // Zero size
    let zero_size_hdr = create_rtt_header(0x20001000, 0, 0, 0);
    assert!(RttControlBlock::parse_from_slice(0x20000400, &zero_size_hdr).is_none());
}

#[test]
fn test_scan_rtt_in_memory() {
    let mut ram = MockRam::new();
    let sram_base = 0x20000000;
    let cb_addr = sram_base + 0x0180; // offset in RAM
    let buf_addr = sram_base + 0x0400;

    let hdr = create_rtt_header(buf_addr, 512, 10, 0);
    ram.mem_write_block(cb_addr, &hdr).unwrap();

    let cb = scan_rtt(&mut ram, sram_base, 0x1000).expect("Should find RTT block");
    assert_eq!(cb.cb_addr, cb_addr);
    assert_eq!(cb.up_buffer.size, 512);
    assert_eq!(cb.up_buffer.buffer_ptr, buf_addr);
}

#[test]
fn test_scan_rtt_not_found() {
    let mut ram = MockRam::new();
    let res = scan_rtt(&mut ram, 0x20000000, 0x1000);
    assert_eq!(res, Err(RttError::NotFound));
}

#[test]
fn test_drain_rtt_contiguous() {
    let mut ram = MockRam::new();
    let cb_addr = 0x20000200;
    let buf_addr = 0x20000400;
    let size = 64u32;

    let hdr = create_rtt_header(buf_addr, size, 11, 0);
    ram.mem_write_block(cb_addr, &hdr).unwrap();
    ram.mem_write_block(buf_addr, b"Hello World").unwrap();

    let mut cb = RttControlBlock::parse_from_slice(cb_addr, &hdr).unwrap();
    let mut out = [0u8; 128];

    let n = drain_rtt_up(&mut ram, &mut cb, &mut out).expect("Drain should succeed");
    assert_eq!(n, 11);
    assert_eq!(&out[..11], b"Hello World");
    assert_eq!(cb.up_buffer.read_offset, 11);

    // Verify target RAM read offset was updated
    let rd_in_ram = ram.mem_read32(cb_addr + 24 + 16).unwrap();
    assert_eq!(rd_in_ram, 11);

    // Second drain should return 0 since buffer is caught up
    let n2 = drain_rtt_up(&mut ram, &mut cb, &mut out).expect("Drain should succeed");
    assert_eq!(n2, 0);
}

#[test]
fn test_drain_rtt_wraparound() {
    let mut ram = MockRam::new();
    let cb_addr = 0x20000200;
    let buf_addr = 0x20000400;
    let size = 16u32;

    // Buffer has 16 bytes. Read pointer at 12, write pointer wrapped around to 4.
    // Bytes [12..16] = "WXYZ", Bytes [0..4] = "ABCD"
    // Total available = 4 + 4 = 8 bytes ("WXYZABCD")
    let hdr = create_rtt_header(buf_addr, size, 4, 12);
    ram.mem_write_block(cb_addr, &hdr).unwrap();
    ram.mem_write_block(buf_addr + 12, b"WXYZ").unwrap();
    ram.mem_write_block(buf_addr, b"ABCD").unwrap();

    let mut cb = RttControlBlock::parse_from_slice(cb_addr, &hdr).unwrap();
    let mut out = [0u8; 32];

    let n = drain_rtt_up(&mut ram, &mut cb, &mut out).expect("Drain should succeed");
    assert_eq!(n, 8);
    assert_eq!(&out[..8], b"WXYZABCD");
    assert_eq!(cb.up_buffer.read_offset, 4);

    let rd_in_ram = ram.mem_read32(cb_addr + 24 + 16).unwrap();
    assert_eq!(rd_in_ram, 4);
}

#[test]
fn test_format_rtt_status_json() {
    let mut state = RttEngineState::new();
    let mut buf = String::<256>::new();

    format_rtt_status_json(&state, &mut buf);
    assert!(buf.contains(r#""found":false"#));

    state.found = true;
    state.cb.cb_addr = 0x20001000;
    state.cb.up_buffer = RttBufferDesc {
        name_ptr: 0,
        buffer_ptr: 0x20002000,
        size: 1024,
        write_offset: 20,
        read_offset: 20,
        flags: 0,
    };
    state.total_bytes_read = 500;

    buf.clear();
    format_rtt_status_json(&state, &mut buf);
    assert!(buf.contains(r#""found":true"#));
    assert!(buf.contains(r#""cb_addr":"0x20001000""#));
    assert!(buf.contains(r#""buffer_size":1024"#));
    assert!(buf.contains(r#""total_bytes_read":500"#));
}

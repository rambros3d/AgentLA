use agentla_programmer::algo::{AlgoBlobHeader, ALGO_MAGIC, ALGO_VERSION};
use agentla_programmer::dap::*;
use agentla_programmer::swd::*;
use agentla_programmer::target::cortex_m::{
    CortexM, DHCSR_C_DEBUGEN, DHCSR_DBGKEY, DHCSR_S_HALT, DHCSR_S_REGRDY,
};
use agentla_programmer::target::targets::*;
use agentla_programmer::traits::SwdIo;
use std::collections::HashMap;

#[derive(Default)]
struct MockSwd {
    dp_regs: HashMap<u8, u32>,
    ap_regs: HashMap<u32, u32>, // (apsel << 8) | reg
    current_ap_select: u32,
    reset_count: usize,
    jtag_to_swd_count: usize,
}

impl MockSwd {
    fn new() -> Self {
        let mut s = Self::default();
        // Default DPIDR for Cortex-M3
        s.dp_regs.insert(DP_DPIDR, 0x1BA01477);
        s.dp_regs.insert(DP_CTRLSTAT, 0xF0000000);
        s
    }
}

impl SwdIo for MockSwd {
    type Error = &'static str;

    fn transfer(&mut self, req: u8, data: Option<u32>) -> Result<(u8, u32), Self::Error> {
        let is_ap = (req & (1 << 1)) != 0;
        let is_read = (req & (1 << 2)) != 0;
        let a23 = (req >> 3) & 3;
        let reg = a23 << 2;

        if !is_ap {
            // DP access
            if is_read {
                let val = self.dp_regs.get(&reg).copied().unwrap_or(0);
                Ok((SWD_ACK_OK, val))
            } else {
                let val = data.unwrap_or(0);
                if reg == DP_SELECT {
                    self.current_ap_select = val;
                }
                self.dp_regs.insert(reg, val);
                Ok((SWD_ACK_OK, 0))
            }
        } else {
            // AP access
            let apsel = (self.current_ap_select >> 24) as u8;
            let apbank = ((self.current_ap_select >> 4) & 0x0F) as u8;
            let full_reg = ((apsel as u32) << 8) | ((apbank as u32) << 4) | (reg as u32);

            if is_read {
                let val = self.ap_regs.get(&full_reg).copied().unwrap_or(0);
                // In SWD, AP read data is buffered in DP_RDBUFF
                self.dp_regs.insert(DP_RDBUFF, val);
                Ok((SWD_ACK_OK, val))
            } else {
                let val = data.unwrap_or(0);
                self.ap_regs.insert(full_reg, val);
                Ok((SWD_ACK_OK, 0))
            }
        }
    }

    fn reset_sequence(&mut self) -> Result<(), Self::Error> {
        self.reset_count += 1;
        Ok(())
    }

    fn jtag_to_swd(&mut self) -> Result<(), Self::Error> {
        self.jtag_to_swd_count += 1;
        Ok(())
    }
}

#[test]
fn test_cmsis_dap_info() {
    let mock = MockSwd::new();
    let mut handler = CmsisDapHandler::new(mock);
    let mut resp = [0u8; 64];

    // 1. DAP_Info: Vendor
    let len = handler.handle_packet(&[DAP_INFO_ID, 0x01], &mut resp);
    assert!(len > 2);
    assert_eq!(resp[0], DAP_INFO_ID);
    let vendor_len = resp[1] as usize;
    assert_eq!(&resp[2..2 + vendor_len], b"AgentLA");

    // 2. DAP_Info: Capabilities (SWD supported)
    let len = handler.handle_packet(&[DAP_INFO_ID, 0xF0], &mut resp);
    assert_eq!(len, 3);
    assert_eq!(resp[2] & 0x01, 1); // SWD bit set
}

#[test]
fn test_cmsis_dap_connect_disconnect() {
    let mock = MockSwd::new();
    let mut handler = CmsisDapHandler::new(mock);
    let mut resp = [0u8; 64];

    // Connect SWD
    let len = handler.handle_packet(&[DAP_CONNECT, DAP_PORT_SWD], &mut resp);
    assert_eq!(len, 2);
    assert_eq!(resp[0], DAP_CONNECT);
    assert_eq!(resp[1], DAP_PORT_SWD);
    assert!(handler.connected);

    // Disconnect
    let len = handler.handle_packet(&[DAP_DISCONNECT], &mut resp);
    assert_eq!(len, 2);
    assert_eq!(resp[0], DAP_DISCONNECT);
    assert_eq!(resp[1], 0);
    assert!(!handler.connected);
}

#[test]
fn test_adiv5_engine_dp_ap_access() {
    let mock = MockSwd::new();
    let mut adiv5 = Adiv5Engine::new(mock);

    // Read DPIDR
    let mut dpidr = 0u32;
    assert!(adiv5.read_dp(DP_DPIDR, &mut dpidr).is_ok());
    assert_eq!(dpidr, 0x1BA01477);

    // Write & Read DP register (e.g. CTRLSTAT)
    assert!(adiv5.write_dp(DP_CTRLSTAT, 0x50000000).is_ok());
    let mut ctrlstat = 0u32;
    assert!(adiv5.read_dp(DP_CTRLSTAT, &mut ctrlstat).is_ok());
    assert_eq!(ctrlstat, 0x50000000);

    // Write AP register
    assert!(adiv5.write_ap(AP_TAR, 0x20000000).is_ok());
    let mut tar = 0u32;
    assert!(adiv5.read_ap(AP_TAR, &mut tar).is_ok());
    assert_eq!(tar, 0x20000000);
}

#[test]
fn test_cortex_m_memory_and_core_regs() {
    let mut mock = MockSwd::new();
    // Pre-populate DHCSR with HALTED and REGRDY flags so halt/reg access succeeds
    mock.ap_regs.insert(
        AP_DRW as u32,
        DHCSR_DBGKEY | DHCSR_C_DEBUGEN | DHCSR_S_HALT | DHCSR_S_REGRDY,
    );
    let mut adiv5 = Adiv5Engine::new(mock);
    let mut cortex = CortexM::new(&mut adiv5);

    // Memory write and readback
    assert!(cortex.mem_write32(0x20000100, 0xCAFEBABE).is_ok());
    let val = cortex.mem_read32(0x20000100).unwrap();
    assert_eq!(val, 0xCAFEBABE);

    // Core register write and readback
    assert!(cortex.write_core_reg(0, 0x12345678).is_ok());
}

#[test]
fn test_target_identification() {
    // STM32F103 Medium Density
    let info = identify_cortex_target(0x1BA01477, 0x00000410, 0);
    assert_eq!(info.family, TargetFamily::Stm32F1);
    assert_eq!(info.flash_size, 128 * 1024);

    // RP2040
    let info_rp2040 = identify_cortex_target(0x0BC12477, 0, 0x00002040);
    assert_eq!(info_rp2040.family, TargetFamily::Rp2040);
    assert_eq!(info_rp2040.flash_base, 0x10000000);

    // RP2350
    let info_rp2350 = identify_cortex_target(0x0BA04477, 0, 0x00002350);
    assert_eq!(info_rp2350.family, TargetFamily::Rp2350);
    assert_eq!(info_rp2350.flash_base, 0x10000000);

    // CH32V003
    let ch32_info = ch32v003_info(0x00030500);
    assert_eq!(ch32_info.family, TargetFamily::Ch32V003);
}

#[test]
fn test_algo_blob_header_parsing() {
    let mut valid_bytes = [0u8; 44];
    valid_bytes[0..4].copy_from_slice(&ALGO_MAGIC.to_le_bytes());
    valid_bytes[4..8].copy_from_slice(&ALGO_VERSION.to_le_bytes());
    valid_bytes[8..12].copy_from_slice(&1024u32.to_le_bytes()); // code_size
    valid_bytes[12..16].copy_from_slice(&4u32.to_le_bytes());   // init
    valid_bytes[16..20].copy_from_slice(&8u32.to_le_bytes());   // erase
    valid_bytes[20..24].copy_from_slice(&12u32.to_le_bytes());  // program
    valid_bytes[32..36].copy_from_slice(&0x20000000u32.to_le_bytes()); // ram_base

    let hdr = AlgoBlobHeader::from_bytes(&valid_bytes).expect("Valid header");
    assert_eq!(hdr.magic, ALGO_MAGIC);
    assert_eq!(hdr.code_size, 1024);
    assert_eq!(hdr.ram_base, 0x20000000);

    // Corrupted magic
    valid_bytes[0..4].copy_from_slice(&0xDEADBEEFu32.to_le_bytes());
    assert!(AlgoBlobHeader::from_bytes(&valid_bytes).is_none());
}

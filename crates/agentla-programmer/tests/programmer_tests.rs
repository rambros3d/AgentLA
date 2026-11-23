use agentla_programmer::algo::{AlgoBlobHeader, ALGO_MAGIC, ALGO_VERSION};
use agentla_programmer::dap::*;
use agentla_programmer::rvswd::*;
use agentla_programmer::stlink::*;
use agentla_programmer::swd::*;
use agentla_programmer::target::cortex_m::{
    CortexM, DHCSR_C_DEBUGEN, DHCSR_DBGKEY, DHCSR_S_HALT, DHCSR_S_REGRDY,
};
use agentla_programmer::target::targets::*;
use agentla_programmer::traits::{RvswdIo, SwdIo};
use agentla_programmer::wch::*;
use std::collections::HashMap;

#[derive(Default)]
struct MockSwd {
    dp_regs: HashMap<u8, u32>,
    ap_regs: HashMap<u32, u32>, // (apsel << 8) | reg
    current_ap_select: u32,
    reset_count: usize,
    jtag_to_swd_count: usize,
    clock_khz: u32,
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
                if reg != DP_ABORT {
                    self.dp_regs.insert(reg, val);
                }
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

    fn set_clock(&mut self, freq_khz: u32) -> Result<(), Self::Error> {
        self.clock_khz = freq_khz;
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
fn test_cortex_m_dual_ap_selection_and_memory_access() {
    let mock = MockSwd::new();
    let mut adiv5 = Adiv5Engine::new(mock);
    let mut cortex = CortexM::new(&mut adiv5);

    // Initial state: AP 0, default CSW
    assert_eq!(cortex.apsel, 0);
    assert_eq!(cortex.csw, ADIV5_AP_CSW_DEFAULT);

    // 1. Configure for ARMv8-M / Cortex-M33 (STM32H5 DPIDR 0x6BA02477)
    assert!(cortex.auto_configure_for_dpidr(0x6BA02477).is_ok());
    assert_eq!(cortex.apsel, 1);
    assert_eq!(cortex.csw, 0x0B000012);

    // 2. Perform memory write targeting AP 1
    assert!(cortex.mem_write32(0x08000000, 0x12345678).is_ok());

    // Verify AP 1 registers were written with correct values and AP 0 was untouched
    let ap1_tar_key = ((1u32) << 8) | (AP_TAR as u32);
    let ap1_drw_key = ((1u32) << 8) | (AP_DRW as u32);
    let ap1_csw_key = ((1u32) << 8) | (AP_CSW as u32);
    let ap0_tar_key = ((0u32) << 8) | (AP_TAR as u32);

    assert_eq!(cortex.swd.transport.ap_regs.get(&ap1_tar_key).copied(), Some(0x08000000));
    assert_eq!(cortex.swd.transport.ap_regs.get(&ap1_drw_key).copied(), Some(0x12345678));
    assert_eq!(cortex.swd.transport.ap_regs.get(&ap1_csw_key).copied(), Some(0x0B000012));
    assert_eq!(cortex.swd.transport.ap_regs.get(&ap0_tar_key), None);

    // 3. Perform memory read from AP 1
    let val = cortex.mem_read32(0x08000000).unwrap();
    assert_eq!(val, 0x12345678);

    // 4. Configure for legacy ARMv7-M (STM32F1 DPIDR 0x1BA01477)
    assert!(cortex.auto_configure_for_dpidr(0x1BA01477).is_ok());
    assert_eq!(cortex.apsel, 0);
    assert_eq!(cortex.csw, ADIV5_AP_CSW_DEFAULT);

    // 5. Memory write now targets AP 0
    assert!(cortex.mem_write32(0x20000000, 0xAABBCCDD).is_ok());
    assert_eq!(cortex.swd.transport.ap_regs.get(&ap0_tar_key).copied(), Some(0x20000000));
}

#[test]
fn test_cortex_m33_connect_under_reset() {
    let mut mock = MockSwd::new();
    mock.dp_regs.insert(DP_DPIDR, 0x6BA02477);
    let ap0_drw_key = ((0u32) << 8) | (AP_DRW as u32);
    let ap1_drw_key = ((1u32) << 8) | (AP_DRW as u32);
    mock.ap_regs.insert(ap0_drw_key, DHCSR_DBGKEY | DHCSR_C_DEBUGEN | DHCSR_S_HALT | DHCSR_S_REGRDY);
    mock.ap_regs.insert(ap1_drw_key, DHCSR_DBGKEY | DHCSR_C_DEBUGEN | DHCSR_S_HALT | DHCSR_S_REGRDY);

    let mut adiv5 = Adiv5Engine::new(mock);
    let mut cortex = CortexM::new(&mut adiv5);

    let res = cortex.connect_under_reset();
    assert!(res.is_ok());
    assert_eq!(cortex.apsel, 1);
    assert_eq!(cortex.csw, 0x0B000012);
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

#[derive(Default)]
struct MockRvswd {
    regs: HashMap<u8, u32>,
    reset_count: usize,
    clock_khz: u32,
}

impl MockRvswd {
    fn new() -> Self {
        let mut s = Self::default();
        s.regs.insert(0x7F, 0x30700518); // CH32V307 chip ID
        s.regs.insert(0x11, 0x00030C82); // dmstatus: running
        s
    }
}

impl RvswdIo for MockRvswd {
    type Error = &'static str;

    fn write(&mut self, reg: u8, value: u32) -> Result<(), Self::Error> {
        self.regs.insert(reg, value);
        Ok(())
    }

    fn read(&mut self, reg: u8) -> Result<u32, Self::Error> {
        Ok(self.regs.get(&reg).copied().unwrap_or(0))
    }

    fn reset(&mut self) -> Result<(), Self::Error> {
        self.reset_count += 1;
        Ok(())
    }

    fn set_clock(&mut self, freq_khz: u32) -> Result<(), Self::Error> {
        self.clock_khz = freq_khz;
        Ok(())
    }
}

#[test]
fn test_rvswd_parity_calculations() {
    // 0 has 0 ones (even) -> parity bit is 0, double is 0b00
    assert_eq!(even_parity_bit(0), 0);
    assert_eq!(double_even_parity(0), 0b00);

    // 1 has 1 one (odd) -> parity bit is 1, double is 0b11
    assert_eq!(even_parity_bit(1), 1);
    assert_eq!(double_even_parity(1), 0b11);

    // 0x03 has 2 ones (even) -> parity bit is 0, double is 0b00
    assert_eq!(even_parity_bit(0x03), 0);
    assert_eq!(double_even_parity(0x03), 0b00);

    // 0x07 has 3 ones (odd) -> parity bit is 1, double is 0b11
    assert_eq!(even_parity_bit(0x07), 1);
    assert_eq!(double_even_parity(0x07), 0b11);
}

#[test]
fn test_rvswd_packet_framing_and_status() {
    let h_read = build_header_byte(0x11, false);
    assert_eq!(h_read, (0x11 << 1) | 0);
    let h_write = build_header_byte(0x10, true);
    assert_eq!(h_write, (0x10 << 1) | 1);

    // Status verification: 0x07 (0b0111) is STATUS_OK
    assert!(verify_status(0x07).is_ok());
    assert!(verify_status(0x0F).is_err());
    assert!(verify_status(0x00).is_err());

    // Tail verification: data = 1 has odd parity (1), so bit 5 must be 1.
    // tail: bit 5 = 1, bit 4 = 1, status = 0x07 -> (1 << 5) | (1 << 4) | 0x07 = 0x37
    let valid_tail = 0x37;
    assert!(verify_read_tail(1, valid_tail).is_ok());

    // Bad parity
    let bad_par_tail = 0x17; // bit 5 = 0
    assert_eq!(verify_read_tail(1, bad_par_tail), Err(RvswdError::ParityError));

    // Bad status
    let bad_stat_tail = 0x30;
    assert!(verify_read_tail(1, bad_stat_tail).is_err());
}

#[test]
fn test_stlink_version_handshake() {
    let mock = MockSwd::new();
    let mut handler = StLinkHandler::new(mock);
    let mut resp = [0u8; 64];

    // Default version: V2J48M35 = 0x2C23, VID 0x0483, PID 0x3752
    assert_eq!(handler.reported_version(), 0x2C23);
    let len = handler.execute(&[STLINK_GET_VERSION], &mut resp);
    assert_eq!(len, 6);
    assert_eq!(resp[0], 0x2C);
    assert_eq!(resp[1], 0x23);
    assert_eq!(resp[2], 0x83);
    assert_eq!(resp[3], 0x04);
    assert_eq!(resp[4], 0x52);
    assert_eq!(resp[5], 0x37);

    // Dynamic version update to V2J49M36 = 0x2C64
    handler.set_reported_version(0x2C64);
    assert_eq!(handler.reported_version(), 0x2C64);
    let len = handler.execute(&[STLINK_GET_VERSION], &mut resp);
    assert_eq!(len, 6);
    assert_eq!(resp[0], 0x2C);
    assert_eq!(resp[1], 0x64);

    // Dynamic version update to V2J46M27 = 0x2B9B
    handler.set_reported_version(0x2B9B);
    assert_eq!(handler.reported_version(), 0x2B9B);
    let len = handler.execute(&[STLINK_GET_VERSION], &mut resp);
    assert_eq!(len, 6);
    assert_eq!(resp[0], 0x2B);
    assert_eq!(resp[1], 0x9B);
}

#[test]
fn test_stlink_enter_swd_replies_without_target() {
    // hybridlink's contract: ENTER_SWD reports success even when the target
    // cannot be reached, so the host's command never times out.
    let mock = MockSwd::new();
    let mut handler = StLinkHandler::new(mock);
    let mut resp = [0u8; 64];

    let len = handler.execute(&[STLINK_DEBUG_COMMAND, 0x30, 0xA3], &mut resp);
    assert_eq!(len, 2, "ENTER_SWD must always reply 2 bytes");
    assert_eq!(resp[0], 0x80, "STLINK_OK");
    assert_eq!(resp[1], 0);
    // The mock transport returns 0 for every read, and the handler normalises
    // an all-ones DPIDR to "no target". Either way the reply is unaffected.
    assert_ne!(handler.idcode, 0xFFFF_FFFF, "all-ones DPIDR means no target");
}

#[test]
fn test_stlink_enter_swd_rejects_non_swd() {
    let mock = MockSwd::new();
    let mut handler = StLinkHandler::new(mock);
    let mut resp = [0u8; 64];

    let len = handler.execute(&[STLINK_DEBUG_COMMAND, 0x30, 0x00], &mut resp);
    assert_eq!(len, 2);
    assert_eq!(resp[0], STLINK_NO_DEVICE);
}

#[test]
fn test_stlink_core_debug_resets_ap_selector() {
    // hybridlink's prologue: core debug commands run on AP 0 regardless of
    // what a previous memory command selected.
    let mock = MockSwd::new();
    let mut handler = StLinkHandler::new(mock);
    let mut resp = [0u8; 64];

    handler.current_ap = 1;
    handler.execute(&[STLINK_DEBUG_COMMAND, 0x09], &mut resp); // DBG_RUNCORE
    assert_eq!(handler.current_ap, 0, "RUNCORE must select AP 0");
}

#[test]
fn test_stlink_dap_commands_keep_ap_selector() {
    // DAP-level commands manage their own AP state and must not be reset.
    let mock = MockSwd::new();
    let mut handler = StLinkHandler::new(mock);
    let mut resp = [0u8; 64];

    handler.current_ap = 1;
    handler.execute(&[STLINK_DEBUG_COMMAND, 0x4B, 0x01], &mut resp); // INIT_AP
    assert_eq!(handler.current_ap, 1, "INIT_AP must not reset the AP");
}

#[test]
fn test_stlink_version_parsing_and_formatting() {
    use agentla_programmer::stlink::{format_version, parse_version};

    let mut buf = [0u8; 16];

    // Standard preset: V2J48M35 -> 0x2C23
    assert_eq!(parse_version("V2J48M35"), Some(0x2C23));
    assert_eq!(format_version(0x2C23, &mut buf), "V2J48M35");

    // Case insensitivity and optional leading 'V'
    assert_eq!(parse_version("v2j48m35"), Some(0x2C23));
    assert_eq!(parse_version("2J48M35"), Some(0x2C23));
    assert_eq!(parse_version("  V2J48M35  "), Some(0x2C23));

    // Preset V2J46M27 -> 0x2B9B (v=2, j=46, m=27)
    assert_eq!(parse_version("V2J46M27"), Some(0x2B9B));
    assert_eq!(format_version(0x2B9B, &mut buf), "V2J46M27");

    // Preset V2J49M36 -> 0x2C64 (v=2, j=49, m=36)
    assert_eq!(parse_version("V2J49M36"), Some(0x2C64));
    assert_eq!(format_version(0x2C64, &mut buf), "V2J49M36");

    // Boundary checks: M=0, J=1, V=0
    let min_ver = parse_version("V0J1M0").expect("valid min version");
    assert_eq!(min_ver, (0 << 12) | (1 << 6) | 0);
    assert_eq!(format_version(min_ver, &mut buf), "V0J1M0");

    // Boundary checks: M=63, J=63, V=15
    let max_ver = parse_version("V15J63M63").expect("valid max version");
    assert_eq!(max_ver, (15 << 12) | (63 << 6) | 63);
    assert_eq!(format_version(max_ver, &mut buf), "V15J63M63");

    // Malformed inputs
    assert_eq!(parse_version(""), None);
    assert_eq!(parse_version("   "), None);
    assert_eq!(parse_version("invalid"), None);
    assert_eq!(parse_version("V2J48"), None);
    assert_eq!(parse_version("V2M35"), None);
    assert_eq!(parse_version("J48M35"), None);
    assert_eq!(parse_version("V2J48M35extra"), None);

    // Out-of-bounds component values
    assert_eq!(parse_version("V2J0M35"), None);    // J cannot be 0
    assert_eq!(parse_version("V2J64M35"), None);   // J > 63 (exceeds 6-bit field)
    assert_eq!(parse_version("V2J48M64"), None);   // M > 63 (exceeds 6-bit field)
    assert_eq!(parse_version("V16J48M35"), None);  // V > 15 (exceeds 4-bit field)
}

#[test]
fn test_stlink_mode_and_debug_entry() {
    let mock = MockSwd::new();
    let mut handler = StLinkHandler::new(mock);
    let mut resp = [0u8; 64];

    // Initial mode is MASS (0x01)
    let len = handler.execute(&[STLINK_GET_CURRENT_MODE], &mut resp);
    assert_eq!(len, 2);
    assert_eq!(resp[0], STLINK_MODE_MASS);

    // Enter SWD
    let len = handler.execute(&[STLINK_DEBUG_COMMAND, DBG_APIV2_ENTER, DBG_ENTER_SWD], &mut resp);
    assert_eq!(len, 2);
    assert_eq!(resp[0], STLINK_OK);

    // Mode is now DEBUG (0x02)
    let len = handler.execute(&[STLINK_GET_CURRENT_MODE], &mut resp);
    assert_eq!(len, 2);
    assert_eq!(resp[0], STLINK_MODE_DEBUG);

    // IDCODEs are resolved after ENTER_SWD, matching a genuine ST-Link.
    let len = handler.execute(&[STLINK_DEBUG_COMMAND, DBG_APIV2_READ_IDCODES], &mut resp);
    assert_eq!(len, 12);
    assert_eq!(resp[0], STLINK_OK);

    // Read Core ID
    let len = handler.execute(&[STLINK_DEBUG_COMMAND, DBG_READCOREID], &mut resp);
    assert_eq!(len, 4);
    let id = u32::from_le_bytes([resp[0], resp[1], resp[2], resp[3]]);
    assert_eq!(id, 0x1BA01477);

    // Exit debug mode
    let _ = handler.execute(&[STLINK_DEBUG_COMMAND, DBG_EXIT], &mut resp);
    let _ = handler.execute(&[STLINK_GET_CURRENT_MODE], &mut resp);
    assert_eq!(resp[0], STLINK_MODE_MASS);
}

#[test]
fn test_stlink_get_target_voltage() {
    let mock = MockSwd::new();
    let mut handler = StLinkHandler::new(mock);
    let mut resp = [0u8; 64];

    // 1. Fallback default (voltage_provider is None): 2400 ref factor, 3300 mV target
    let len = handler.execute(&[STLINK_GET_TARGET_VOLTAGE], &mut resp);
    assert_eq!(len, 8);
    assert_eq!(&resp[0..4], &2400u32.to_le_bytes());
    assert_eq!(&resp[4..8], &3300u32.to_le_bytes());

    // 2. Active 3.3V target: 3300 mV
    fn mock_voltage_3300() -> u32 {
        3300
    }
    handler.set_voltage_provider(mock_voltage_3300);
    let len = handler.execute(&[STLINK_GET_TARGET_VOLTAGE], &mut resp);
    assert_eq!(len, 8);
    assert_eq!(&resp[0..4], &2400u32.to_le_bytes());
    assert_eq!(&resp[4..8], &3300u32.to_le_bytes());

    // 3. Active 1.8V target: 1800 mV
    fn mock_voltage_1800() -> u32 {
        1800
    }
    handler.set_voltage_provider(mock_voltage_1800);
    let len = handler.execute(&[STLINK_GET_TARGET_VOLTAGE], &mut resp);
    assert_eq!(len, 8);
    assert_eq!(&resp[0..4], &2400u32.to_le_bytes());
    assert_eq!(&resp[4..8], &1800u32.to_le_bytes());

    // 4. Unpowered floating (<800 mV, e.g. 350 mV) clamped to 0 mV
    fn mock_voltage_floating() -> u32 {
        350
    }
    handler.set_voltage_provider(mock_voltage_floating);
    let len = handler.execute(&[STLINK_GET_TARGET_VOLTAGE], &mut resp);
    assert_eq!(len, 8);
    assert_eq!(&resp[0..4], &2400u32.to_le_bytes());
    assert_eq!(&resp[4..8], &0u32.to_le_bytes());

    // 5. Unpowered disconnected (0 mV)
    fn mock_voltage_zero() -> u32 {
        0
    }
    handler.set_voltage_provider(mock_voltage_zero);
    let len = handler.execute(&[STLINK_GET_TARGET_VOLTAGE], &mut resp);
    assert_eq!(len, 8);
    assert_eq!(&resp[0..4], &2400u32.to_le_bytes());
    assert_eq!(&resp[4..8], &0u32.to_le_bytes());

    // 6. Threshold boundary: 799 mV clamped to 0 mV, 800 mV reports 800 mV
    fn mock_voltage_799() -> u32 {
        799
    }
    handler.set_voltage_provider(mock_voltage_799);
    let len = handler.execute(&[STLINK_GET_TARGET_VOLTAGE], &mut resp);
    assert_eq!(len, 8);
    assert_eq!(&resp[0..4], &2400u32.to_le_bytes());
    assert_eq!(&resp[4..8], &0u32.to_le_bytes());

    fn mock_voltage_800() -> u32 {
        800
    }
    handler.set_voltage_provider(mock_voltage_800);
    let len = handler.execute(&[STLINK_GET_TARGET_VOLTAGE], &mut resp);
    assert_eq!(len, 8);
    assert_eq!(&resp[0..4], &2400u32.to_le_bytes());
    assert_eq!(&resp[4..8], &800u32.to_le_bytes());
}

#[test]
fn test_wchlink_version_and_attach() {
    let mock = MockRvswd::new();
    let mut handler = WchLinkHandler::new(mock);
    let mut resp = [0u8; 64];

    // Version query (0x0D 0x01)
    let len = handler.execute(&[0x81, CMD_CONTROL, 0x01, 0x01], &mut resp);
    assert_eq!(len, 7);
    assert_eq!(resp[0], 0x82);
    assert_eq!(resp[1], CMD_CONTROL);
    assert_eq!(resp[2], 0x04);
    assert_eq!(&resp[3..7], &[FW_MAJOR, FW_MINOR, PROBE_TYPE_CH549, 0]);

    // Attach (0x0D 0x02)
    let len = handler.execute(&[0x81, CMD_CONTROL, 0x01, 0x02], &mut resp);
    assert_eq!(len, 8);
    assert_eq!(resp[0], 0x82);
    assert_eq!(resp[1], CMD_CONTROL);
    assert_eq!(resp[2], 0x05);
    assert_eq!(resp[3], 0x06); // Family CH32V30x
    let chip_id = u32::from_be_bytes([resp[4], resp[5], resp[6], resp[7]]);
    assert_eq!(chip_id, 0x30700518);
}

#[test]
fn test_wchlink_dmi_read_write() {
    let mock = MockRvswd::new();
    let mut handler = WchLinkHandler::new(mock);
    let mut resp = [0u8; 64];

    // Write DMI register 0x10 with 0x00000001
    let write_req = [0x81, CMD_DMI, 0x06, 0x10, 0x00, 0x00, 0x00, 0x01, 0x02];
    let len = handler.execute(&write_req, &mut resp);
    assert_eq!(len, 9);
    assert_eq!(resp[0], 0x82);
    assert_eq!(resp[1], CMD_DMI);
    assert_eq!(resp[8], DMI_OK);

    // Read DMI register 0x10
    let read_req = [0x81, CMD_DMI, 0x06, 0x10, 0x00, 0x00, 0x00, 0x00, 0x01];
    let len = handler.execute(&read_req, &mut resp);
    assert_eq!(len, 9);
    assert_eq!(resp[0], 0x82);
    assert_eq!(resp[1], CMD_DMI);
    let val = u32::from_be_bytes([resp[4], resp[5], resp[6], resp[7]]);
    assert_eq!(val, 0x00000001);
}

#[test]
fn test_swo_ring_buffer_wrapping() {
    use agentla_programmer::swo::*;

    let swo = SwoEngine::new();
    assert!(!swo.is_active());
    assert!(swo.start(2_000_000, 0));
    assert!(swo.is_active());
    assert_eq!(swo.baud_rate(), 2_000_000);

    // Push and read 10,000 bytes in chunks of 250 bytes (testing multiple wraps of 4096-byte buffer)
    let mut sent_val = 0u8;
    let mut recv_val = 0u8;
    let mut read_buf = [0u8; 250];

    for _ in 0..40 {
        let mut write_buf = [0u8; 250];
        for b in &mut write_buf {
            *b = sent_val;
            sent_val = sent_val.wrapping_add(1);
        }
        swo.push_bytes(&write_buf);
        assert_eq!(swo.available(), 250);

        let n = swo.read(&mut read_buf);
        assert_eq!(n, 250);
        for &b in &read_buf {
            assert_eq!(b, recv_val);
            recv_val = recv_val.wrapping_add(1);
        }
        assert_eq!(swo.available(), 0);
    }

    let telem = swo.telemetry();
    assert_eq!(telem.bytes_received, 10000);
    assert_eq!(telem.bytes_delivered, 10000);
    assert_eq!(telem.bytes_dropped, 0);
}

#[test]
fn test_swo_overrun_half_buffer_drop() {
    use agentla_programmer::swo::*;

    let swo = SwoEngine::new();
    assert!(swo.start(2_000_000, 0));

    // Push 3841 bytes without reading (overrun threshold is 3840 = 4096 - 256)
    let mut write_buf = [0u8; 3841];
    for (i, b) in write_buf.iter_mut().enumerate() {
        *b = (i & 0xFF) as u8;
    }
    swo.push_bytes(&write_buf);

    // Overrun should have dropped 3841 - 2048 = 1793 bytes, leaving exactly 2048 bytes
    assert_eq!(swo.available(), 2048);
    let telem = swo.telemetry();
    assert_eq!(telem.bytes_received, 3841);
    assert_eq!(telem.bytes_dropped, 1793);
    assert_eq!(telem.bytes_delivered, 0);

    // Verify the 2048 retained bytes correspond to the newest half of the stream (indices 1793..3841)
    let mut read_buf = [0u8; 2048];
    let n = swo.read(&mut read_buf);
    assert_eq!(n, 2048);
    for (i, &b) in read_buf.iter().enumerate() {
        let expected = ((1793 + i) & 0xFF) as u8;
        assert_eq!(b, expected, "Mismatch at index {}", i);
    }
}

#[test]
fn test_swo_stlink_commands_and_exact_promises() {
    use agentla_programmer::stlink::*;

    let mock = MockSwd::new();
    let mut handler = StLinkHandler::new(mock);
    let mut resp = [0u8; 64];

    // DBG_APIV2_START_TRACE_RX: max_buf = 2048, baud = 2_000_000
    // [0xF2, 0x40, 0x00, 0x08, 0x80, 0x84, 0x1E, 0x00] -> trace_max = 2048, baud = 0x001E8480 = 2_000_000
    let start_cmd = [
        STLINK_DEBUG_COMMAND,
        DBG_APIV2_START_TRACE_RX,
        0x00, 0x08, // trace_max = 2048
        0x80, 0x84, 0x1E, 0x00, // baud = 2000000
    ];
    let len = handler.execute(&start_cmd, &mut resp);
    assert_eq!(len, 2);
    assert_eq!(resp[0], STLINK_OK);
    assert!(handler.swo.is_active());
    assert_eq!(handler.swo.baud_rate(), 2_000_000);

    // Push 150 bytes into the SWO buffer
    let mut trace_data = [0u8; 150];
    for (i, b) in trace_data.iter_mut().enumerate() {
        *b = (0xA0u8).wrapping_add(i as u8);
    }
    handler.swo.push_bytes(&trace_data);

    // DBG_APIV2_GET_TRACE_NB: Should return 150 (0x0096)
    let get_nb_cmd = [STLINK_DEBUG_COMMAND, DBG_APIV2_GET_TRACE_NB];
    let len = handler.execute(&get_nb_cmd, &mut resp);
    assert_eq!(len, 2);
    let promised = (resp[0] as u16) | ((resp[1] as u16) << 8);
    assert_eq!(promised, 150);
    assert_eq!(handler.swo.trace_owed(), 150);

    // Pop packets via pop_trace_packet into 64-byte chunks
    let mut ep_buf = [0u8; 64];

    // Packet 1: 64 bytes
    let n1 = handler.swo.pop_trace_packet(&mut ep_buf);
    assert_eq!(n1, 64);
    assert_eq!(&ep_buf[..64], &trace_data[..64]);
    assert_eq!(handler.swo.trace_owed(), 86);

    // Packet 2: 64 bytes
    let n2 = handler.swo.pop_trace_packet(&mut ep_buf);
    assert_eq!(n2, 64);
    assert_eq!(&ep_buf[..64], &trace_data[64..128]);
    assert_eq!(handler.swo.trace_owed(), 22);

    // Packet 3: 22 bytes (short packet terminating transfer)
    let n3 = handler.swo.pop_trace_packet(&mut ep_buf);
    assert_eq!(n3, 22);
    assert_eq!(&ep_buf[..22], &trace_data[128..150]);
    assert_eq!(handler.swo.trace_owed(), 0);

    // Next pop should return 0 since owed is 0
    let n4 = handler.swo.pop_trace_packet(&mut ep_buf);
    assert_eq!(n4, 0);

    // DBG_APIV2_STOP_TRACE_RX: Should stop trace and reset state
    let stop_cmd = [STLINK_DEBUG_COMMAND, DBG_APIV2_STOP_TRACE_RX];
    let len = handler.execute(&stop_cmd, &mut resp);
    assert_eq!(len, 2);
    assert_eq!(resp[0], STLINK_OK);
    assert!(!handler.swo.is_active());
}

#[test]
fn test_probe_wire_topology_reexport_and_pin_mapping() {
    use agentla_programmer::traits::ProbeWireTopology;

    let combined = ProbeWireTopology::Combined { clk: 4, dio: 5 };
    assert_eq!(combined.clk(), 4);
    assert_eq!(combined.dio_out(), 5);
    assert_eq!(combined.dio_in(), 5);
    assert!(!combined.is_split());

    let split = ProbeWireTopology::Split {
        clk: 14,
        dio_drv: 15,
        dio_sns: 16,
    };
    assert_eq!(split.clk(), 14);
    assert_eq!(split.dio_out(), 15);
    assert_eq!(split.dio_in(), 16);
    assert!(split.is_split());

    let helper_comb = ProbeWireTopology::combined(2, 3);
    assert_eq!(helper_comb.clk(), 2);
    assert_eq!(helper_comb.dio_out(), 3);
    assert_eq!(helper_comb.dio_in(), 3);
    assert!(!helper_comb.is_split());

    let helper_split = ProbeWireTopology::split(10, 11, 12);
    assert_eq!(helper_split.clk(), 10);
    assert_eq!(helper_split.dio_out(), 11);
    assert_eq!(helper_split.dio_in(), 12);
    assert!(helper_split.is_split());
}

#[test]
fn test_stlink_swd_set_freq() {
    let mock = MockSwd::new();
    let mut handler = StLinkHandler::new(mock);
    let mut resp = [0u8; 64];

    // Divisor 0 -> 4000 kHz
    let cmd_div0 = [STLINK_DEBUG_COMMAND, DBG_APIV2_SWD_SET_FREQ, 0x00, 0x00];
    let len = handler.execute(&cmd_div0, &mut resp);
    assert_eq!(len, 2);
    assert_eq!(resp[0], STLINK_OK);
    assert_eq!(handler.engine.transport.clock_khz, 4000);

    // Divisor 1 -> 1800 kHz
    let cmd_div1 = [STLINK_DEBUG_COMMAND, DBG_APIV2_SWD_SET_FREQ, 0x01, 0x00];
    let len = handler.execute(&cmd_div1, &mut resp);
    assert_eq!(len, 2);
    assert_eq!(resp[0], STLINK_OK);
    assert_eq!(handler.engine.transport.clock_khz, 1800);

    // Divisor 2 -> 1200 kHz
    let cmd_div2 = [STLINK_DEBUG_COMMAND, DBG_APIV2_SWD_SET_FREQ, 0x02, 0x00];
    let len = handler.execute(&cmd_div2, &mut resp);
    assert_eq!(len, 2);
    assert_eq!(resp[0], STLINK_OK);
    assert_eq!(handler.engine.transport.clock_khz, 1200);
}

#[test]
fn test_wchlink_speed_tier_command() {
    let mock = MockRvswd::new();
    let mut handler = WchLinkHandler::new(mock);
    let mut resp = [0u8; 64];

    // Speed tier 1 (High speed: 6000 kHz)
    let req_tier1 = [0x81, CMD_SPEED, 0x02, 0x06, 0x01];
    let len = handler.execute(&req_tier1, &mut resp);
    assert_eq!(len, 4);
    assert_eq!(resp[0], 0x82);
    assert_eq!(resp[1], CMD_SPEED);
    assert_eq!(resp[2], 0x01);
    assert_eq!(resp[3], 0x01);
    assert_eq!(handler.rvdm.io.clock_khz, 6000);

    // Speed tier 2 (Medium speed: 4000 kHz)
    let req_tier2 = [0x81, CMD_SPEED, 0x02, 0x06, 0x02];
    let len = handler.execute(&req_tier2, &mut resp);
    assert_eq!(len, 4);
    assert_eq!(resp[0], 0x82);
    assert_eq!(resp[1], CMD_SPEED);
    assert_eq!(resp[2], 0x01);
    assert_eq!(resp[3], 0x01);
    assert_eq!(handler.rvdm.io.clock_khz, 4000);

    // Speed tier 3 (Low speed: 400 kHz)
    let req_tier3 = [0x81, CMD_SPEED, 0x02, 0x06, 0x03];
    let len = handler.execute(&req_tier3, &mut resp);
    assert_eq!(len, 4);
    assert_eq!(resp[0], 0x82);
    assert_eq!(resp[1], CMD_SPEED);
    assert_eq!(resp[2], 0x01);
    assert_eq!(resp[3], 0x01);
    assert_eq!(handler.rvdm.io.clock_khz, 400);

    // Malformed request (payload too short)
    let req_short = [0x81, CMD_SPEED, 0x01, 0x06];
    let len = handler.execute(&req_short, &mut resp);
    assert_eq!(len, 4);
    assert_eq!(resp[0], 0x82);
    assert_eq!(resp[1], 0x81); // unknown error reply
    assert_eq!(resp[3], 0xFF);
}

#[test]
fn test_cmsis_dap_match_value_and_mask() {
    let mut mock = MockSwd::new();
    // DP 0x04 (CTRLSTAT) = 0xF0000000
    mock.dp_regs.insert(DP_CTRLSTAT, 0xF0000000);

    let mut handler = CmsisDapHandler::new(mock);
    let mut resp = [0u8; 64];

    // Connect
    handler.handle_packet(&[DAP_CONNECT, 0x01], &mut resp);

    // 1. Configure Transfer with match retry = 10
    let config_cmd = [DAP_TRANSFER_CONFIGURE, 0, 100, 0, 10, 0];
    let len = handler.handle_packet(&config_cmd, &mut resp);
    assert_eq!(len, 2);
    assert_eq!(resp[0], DAP_TRANSFER_CONFIGURE);
    assert_eq!(resp[1], 0);

    // 2. Transfer with MATCH_MASK and MATCH_VALUE matching DP_CTRLSTAT
    // Request 2 operations:
    // Op 0: Match Mask = 0xF0000000 (req byte 0x20)
    // Op 1: Match Value for DP reg 0x04 = 0xF0000000 (req byte 0x12: is_read=1, a23=1 -> reg 4, match_value=1)
    let transfer_cmd = [
        DAP_TRANSFER,
        0x00, // dap index
        0x02, // 2 transfers
        0x20, // Match Mask
        0x00, 0x00, 0x00, 0xF0, // mask = 0xF0000000
        0x16, // Read DP reg 0x04 (RnW=1, A23=1 -> 0x04 | 0x02 = 0x06 | 0x10 = 0x16)
        0x00, 0x00, 0x00, 0xF0, // expected = 0xF0000000
    ];
    let _len = handler.handle_packet(&transfer_cmd, &mut resp);
    assert_eq!(resp[0], DAP_TRANSFER);
    assert_eq!(resp[1], 2); // 2 transfers completed
    assert_eq!(resp[2], DAP_TRANSFER_OK); // ACK OK

    // 3. Transfer with mismatch
    let transfer_mismatch = [
        DAP_TRANSFER,
        0x00,
        0x02,
        0x20, // Match Mask
        0x00, 0x00, 0x00, 0xF0,
        0x16, // Read DP reg 0x04
        0x00, 0x00, 0x00, 0x10, // expected = 0x10000000 (will mismatch 0xF0000000)
    ];
    let _len = handler.handle_packet(&transfer_mismatch, &mut resp);
    assert_eq!(resp[0], DAP_TRANSFER);
    assert_eq!(resp[1], 1); // 1 transfer completed before mismatch
    assert_eq!(resp[2], DAP_TRANSFER_MISMATCH | DAP_TRANSFER_OK); // 0x11
}

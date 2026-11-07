use agentla_targets::catalog::{lookup_by_dev_id, lookup_by_name};
use agentla_targets::cascade::{flash_cascade_execute, ExecutionTier};
use agentla_targets::engines::FlashEngineRunner;
use agentla_targets::traits::TargetMemAccess;
use agentla_targets::types::{DirectEngineKind, TargetError};

struct MockMem {
    pub mem: [u8; 0x20000], // 128 KB simulated memory
    pub reg_cr: u32,
    pub reg_sr: u32,
    pub reset_asserted: bool,
}

impl MockMem {
    pub fn new() -> Self {
        Self {
            mem: [0xFF; 0x20000],
            reg_cr: 0,
            reg_sr: 0,
            reset_asserted: false,
        }
    }
}

impl TargetMemAccess for MockMem {
    type Error = TargetError;

    fn read32(&mut self, addr: u32) -> Result<u32, Self::Error> {
        if addr >= 0x40000000 {
            // Peripheral address
            if addr == 0x4001E400 {
                return Ok(0x01); // READY=1 for NVMC READY (0x4001E400)
            }
            if (addr & 0x0F) == 0x0C || (addr & 0x0F) == 0x10 {
                return Ok(self.reg_sr); // BSY=0
            }
            if (addr & 0x1F) == 0x14 {
                return Ok(0x01); // READY=1 for NVMCTRL
            }
            return Ok(self.reg_cr);
        }
        let offset = (addr as usize) % self.mem.len();
        if offset + 4 <= self.mem.len() {
            Ok(u32::from_le_bytes(self.mem[offset..offset + 4].try_into().unwrap()))
        } else {
            Ok(0xFFFFFFFF)
        }
    }

    fn write32(&mut self, addr: u32, val: u32) -> Result<(), Self::Error> {
        if addr >= 0x40000000 {
            self.reg_cr = val;
            return Ok(());
        }
        let offset = (addr as usize) % self.mem.len();
        if offset + 4 <= self.mem.len() {
            self.mem[offset..offset + 4].copy_from_slice(&val.to_le_bytes());
        }
        Ok(())
    }

    fn read_block(&mut self, addr: u32, buf: &mut [u8]) -> Result<(), Self::Error> {
        let offset = (addr as usize) % self.mem.len();
        let len = buf.len().min(self.mem.len() - offset);
        buf[..len].copy_from_slice(&self.mem[offset..offset + len]);
        Ok(())
    }

    fn write_block(&mut self, addr: u32, data: &[u8]) -> Result<(), Self::Error> {
        let offset = (addr as usize) % self.mem.len();
        let len = data.len().min(self.mem.len() - offset);
        self.mem[offset..offset + len].copy_from_slice(&data[..len]);
        Ok(())
    }

    fn assert_reset(&mut self, assert: bool) -> Result<(), Self::Error> {
        self.reset_asserted = assert;
        Ok(())
    }
}

#[test]
fn test_catalog_lookup_by_dev_id() {
    let f103 = lookup_by_dev_id(0x410).expect("STM32F103 should match 0x410");
    assert_eq!(f103.engine, DirectEngineKind::Stm32Classic16);
    assert_eq!(f103.flash_base, 0x08000000);

    let f411 = lookup_by_dev_id(0x431).expect("STM32F411 should match 0x431");
    assert_eq!(f411.engine, DirectEngineKind::Stm32Sector);

    let g031 = lookup_by_dev_id(0x466).expect("STM32G031 should match 0x466");
    assert_eq!(g031.engine, DirectEngineKind::Stm32Modern64);

    let nrf52 = lookup_by_dev_id(0x052).expect("nRF52832 should match 0x052");
    assert_eq!(nrf52.engine, DirectEngineKind::NordicNvmc);

    let ch32v003 = lookup_by_dev_id(0x003).expect("CH32V003 should match 0x003");
    assert_eq!(ch32v003.engine, DirectEngineKind::QingKeV2);

    let ch32v203 = lookup_by_dev_id(0x203).expect("CH32V203 should match 0x203");
    assert_eq!(ch32v203.engine, DirectEngineKind::QingKeV4);
}

#[test]
fn test_radix_part_number_matcher() {
    let t1 = lookup_by_name("STM32F103C8T6").expect("Should match STM32F103 prefix");
    assert_eq!(t1.engine, DirectEngineKind::Stm32Classic16);

    let t2 = lookup_by_name("STM32F411CEU6").expect("Should match BlackPill F411");
    assert_eq!(t2.engine, DirectEngineKind::Stm32Sector);

    let t3 = lookup_by_name("nRF52840-QIAA").expect("Should match Nordic nRF52840");
    assert_eq!(t3.engine, DirectEngineKind::NordicNvmc);

    let t4 = lookup_by_name("ATSAMD21G18A-AU").expect("Should match Microchip SAMD21");
    assert_eq!(t4.engine, DirectEngineKind::MicrochipSamd);

    let t5 = lookup_by_name("CH32V003F4P6").expect("Should match CH32V003");
    assert_eq!(t5.engine, DirectEngineKind::QingKeV2);

    let t6 = lookup_by_name("ESP32-C3-WROOM-02").expect("Should match ESP32-C3");
    assert_eq!(t6.engine, DirectEngineKind::EspressifRiscvSpi);
}

#[test]
fn test_mock_stm32_classic_program_and_verify() {
    let mut mem = MockMem::new();
    let desc = lookup_by_name("STM32F103").unwrap();
    let payload = [0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88];

    FlashEngineRunner::program_page(&mut mem, desc, 0x08000000, &payload).unwrap();
    FlashEngineRunner::verify(&mut mem, desc, 0x08000000, &payload).unwrap();
}

#[test]
fn test_mock_nordic_program_and_verify() {
    let mut mem = MockMem::new();
    let desc = lookup_by_name("nRF52840").unwrap();
    let payload = [0xAA, 0xBB, 0xCC, 0xDD, 0x12, 0x34, 0x56, 0x78];

    FlashEngineRunner::program_page(&mut mem, desc, 0x00000000, &payload).unwrap();
    FlashEngineRunner::verify(&mut mem, desc, 0x00000000, &payload).unwrap();
}

#[test]
fn test_3tier_cascade_direct_engine() {
    let mut mem = MockMem::new();
    let payload = [0x01, 0x02, 0x03, 0x04];
    let res = flash_cascade_execute(
        &mut mem,
        "STM32F411",
        0x08000000,
        &payload,
        None,
        None,
    ).unwrap();

    assert_eq!(res.tier, ExecutionTier::Tier1DirectEngine);
    assert_eq!(res.bytes_written, 4);
}

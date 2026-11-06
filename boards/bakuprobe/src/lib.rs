//! BakuProbe Board Support Package (RP2350B, 48 GPIOs, PGA113 front-end, INA180 current sense).

#![no_std]

use agentla_bsp::{
    AdapterPins, AnalogFrontendConfig, BoardFeatures, BoardProfile, CanPins, FlexiblePool,
    I2cPins, McuPackage, ProgrammerPins, Rs485Pins, SpiPins, UartPins,
};

pub const MANIFEST_JSON: &str = include_str!("../board.json");
pub const LOGO_SVG: &[u8] = include_bytes!("../logo.svg");
pub const FAVICON_SVG: &[u8] = include_bytes!("../favicon.svg");
pub const PINOUT_SVG: &[u8] = include_bytes!("../pinout.svg");

/// BakuProbe instrument board profile.
pub struct BakuProbeBoard;

impl BakuProbeBoard {
    pub const fn manifest_json() -> &'static str {
        MANIFEST_JSON
    }
}

impl BoardProfile for BakuProbeBoard {
    const BOARD_ID: &'static str = "bakuprobe";
    const NAME: &'static str = "BakuProbe";
    const VENDOR: &'static str = "AgentLA";
    const VERSION: &'static str = "1.0.0";
    const MCU_PACKAGE: McuPackage = McuPackage::Qfn80;
    const GPIO_COUNT: u32 = 48;

    const PROGRAMMER: ProgrammerPins = ProgrammerPins {
        swclk: 4,
        swdio: 5,
        nrst: 6,
        boot0: 2,
        sdi: 7,
        uart_tx: 8,
        uart_rx: 9,
        trst: Some(0),
        tdo: Some(1),
        tdi: Some(3),
    };

    const ADAPTERS: AdapterPins = AdapterPins {
        can: Some(CanPins {
            tx: 10,
            rx: 11,
            term: Some(12),
        }),
        rs485: Some(Rs485Pins {
            tx: 13,
            rx: 14,
            de_re: 15,
        }),
        default_i2c: I2cPins { sda: 16, scl: 17 },
        default_spi: SpiPins {
            sck: 18,
            mosi: 19,
            miso: 20,
            cs: 21,
        },
        default_uart: UartPins { tx: 22, rx: 23 },
        default_ws2812: None,
    };

    const FLEXIBLE: FlexiblePool = FlexiblePool::new(24, 39);

    const FEATURES: BoardFeatures = BoardFeatures {
        onboard_led: None,
        vref_volts: 3.0,
        has_hardware_can: true,
        has_hardware_rs485: true,
        target_current_sense: true,
        target_power_control: true,
        qwiic_stemma_qt: true,
    };

    fn analog_frontend() -> AnalogFrontendConfig {
        AnalogFrontendConfig::Pga113 {
            adc_pin: 40,
            cs_pin: 21,
            sck_pin: 18,
            mosi_pin: 19,
            gains: &[1, 2, 5, 10, 20, 50, 100, 200],
            has_vcal: true,
        }
    }

    fn manifest_json() -> &'static str {
        MANIFEST_JSON
    }

    fn logo_svg() -> &'static [u8] {
        LOGO_SVG
    }

    fn favicon_svg() -> &'static [u8] {
        FAVICON_SVG
    }

    fn pinout_svg() -> &'static [u8] {
        PINOUT_SVG
    }
}

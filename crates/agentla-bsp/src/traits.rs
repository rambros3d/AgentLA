//! Core Board Support Package (BSP) trait definitions and hardware abstraction types.
//!
//! Models physical pin zones, transceivers, multiplexed flexible pools, and analog front-ends.

/// Silicon packaging and physical GPIO count.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McuPackage {
    /// RP2350A 60-pin QFN (GPIO 0..=29, 4 ADC channels).
    Qfn60,
    /// RP2350B 80-pin QFN (GPIO 0..=47, 8 ADC channels).
    Qfn80,
}

impl McuPackage {
    pub const fn as_str(&self) -> &'static str {
        match self {
            McuPackage::Qfn60 => "QFN-60",
            McuPackage::Qfn80 => "QFN-80",
        }
    }
}

/// Pin assignments for the fixed hardware programmer / debugger interface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProgrammerPins {
    pub swclk: u32,
    pub swdio: u32,
    pub nrst: u32,
    pub boot0: u32,
    pub sdi: u32,
    pub uart_tx: u32,
    pub uart_rx: u32,
    pub trst: Option<u32>,
    pub tdo: Option<u32>,
    pub tdi: Option<u32>,
}

impl ProgrammerPins {
    pub const fn new(
        swclk: u32,
        swdio: u32,
        nrst: u32,
        boot0: u32,
        sdi: u32,
        uart_tx: u32,
        uart_rx: u32,
    ) -> Self {
        Self {
            swclk,
            swdio,
            nrst,
            boot0,
            sdi,
            uart_tx,
            uart_rx,
            trst: None,
            tdo: None,
            tdi: None,
        }
    }

    /// Primary bridge pins (SWCLK, SWDIO, nRST, BOOT0, UART TX, UART RX).
    pub const fn bridge_pins(&self) -> [u32; 6] {
        [
            self.swclk,
            self.swdio,
            self.nrst,
            self.boot0,
            self.uart_tx,
            self.uart_rx,
        ]
    }
}

/// Dedicated or default CAN transceiver pin mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanPins {
    pub tx: u32,
    pub rx: u32,
    pub term: Option<u32>,
}

/// Dedicated or default RS-485 transceiver pin mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rs485Pins {
    pub tx: u32,
    pub rx: u32,
    pub de_re: u32,
}

/// Default I2C bus master pins.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct I2cPins {
    pub sda: u32,
    pub scl: u32,
}

/// Default SPI bus master pins.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpiPins {
    pub sck: u32,
    pub mosi: u32,
    pub miso: u32,
    pub cs: u32,
}

/// Default UART bus master pins.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UartPins {
    pub tx: u32,
    pub rx: u32,
}

/// Dedicated transceiver and default protocol bridge pin mappings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdapterPins {
    pub can: Option<CanPins>,
    pub rs485: Option<Rs485Pins>,
    pub default_i2c: I2cPins,
    pub default_spi: SpiPins,
    pub default_uart: UartPins,
    pub default_ws2812: Option<u32>,
}

/// Flexible multiplexed pin pool allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FlexiblePool {
    pub start: u32,
    pub end: u32,
}

impl FlexiblePool {
    pub const fn new(start: u32, end: u32) -> Self {
        Self { start, end }
    }

    pub const fn range(&self) -> core::ops::RangeInclusive<u32> {
        self.start..=self.end
    }

    pub const fn count(&self) -> u32 {
        if self.end >= self.start {
            self.end - self.start + 1
        } else {
            0
        }
    }

    pub const fn contains(&self, pin: u32) -> bool {
        pin >= self.start && pin <= self.end
    }
}

/// Modeling of the analog oscilloscope and measurement front-end.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnalogFrontendConfig {
    /// Baseline: Direct pin routing into RP2350 SAR ADC (e.g. Pico 2).
    Direct {
        adc_pins: &'static [u32],
        adc_channels: &'static [u8],
        attenuation: u32,
        dual_personality: bool,
    },

    /// High-Precision: SPI-controlled Programmable Gain Amplifier (e.g. TI PGA113 on BakuProbe).
    Pga113 {
        adc_pin: u32,
        cs_pin: u32,
        sck_pin: u32,
        mosi_pin: u32,
        gains: &'static [u16],
        has_vcal: bool,
    },
}

/// Hardware features and onboard peripheral capabilities.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoardFeatures {
    pub onboard_led: Option<u32>,
    pub vref_volts: f32,
    pub has_hardware_can: bool,
    pub has_hardware_rs485: bool,
    pub target_current_sense: bool,
    pub target_power_control: bool,
    pub qwiic_stemma_qt: bool,
}

/// Contract defining a concrete board's hardware profile and capabilities.
pub trait BoardProfile: Send + Sync + 'static {
    const BOARD_ID: &'static str;
    const NAME: &'static str;
    const VENDOR: &'static str;
    const VERSION: &'static str;
    const MCU_PACKAGE: McuPackage;
    const GPIO_COUNT: u32;

    const PROGRAMMER: ProgrammerPins;
    const ADAPTERS: AdapterPins;
    const FLEXIBLE: FlexiblePool;
    const FEATURES: BoardFeatures;

    /// Pin definitions for the target programmer interface.
    fn programmer_pins() -> ProgrammerPins {
        Self::PROGRAMMER
    }

    /// Pin definitions for transceivers and default bus bridges.
    fn adapter_pins() -> AdapterPins {
        Self::ADAPTERS
    }

    /// Inclusive range of the general-purpose multiplexed flexible pin pool.
    fn flexible_pool() -> FlexiblePool {
        Self::FLEXIBLE
    }

    /// Analog front-end architecture and channel bindings.
    fn analog_frontend() -> AnalogFrontendConfig;

    /// Onboard features and hardware peripheral flags.
    fn features() -> BoardFeatures {
        Self::FEATURES
    }

    /// Static JSON manifest metadata for Web Desktop / REST discovery.
    fn manifest_json() -> &'static str;

    /// Embedded SVG branding logo asset.
    fn logo_svg() -> &'static [u8] {
        &[]
    }

    /// Embedded SVG favicon asset.
    fn favicon_svg() -> &'static [u8] {
        &[]
    }

    /// Embedded SVG pinout diagram asset.
    fn pinout_svg() -> &'static [u8] {
        &[]
    }
}

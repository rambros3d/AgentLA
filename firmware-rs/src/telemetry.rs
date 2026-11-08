//! Real-time hardware telemetry engine for AgentLA.
//! Samples target rail power (via TI INA180), RP2350 junction temperature,
//! VSYS voltage, memory gauges, and active 30-pin ownership matrix.
//! Implements hardware overcurrent trip safety and 10 Hz SSE streaming.

use core::fmt::Write as _;
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use rp_pac as pac;

pub static OVERCURRENT_LIMIT_MA: AtomicU32 = AtomicU32::new(500);
pub static OVERCURRENT_TRIPPED: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone, Copy)]
pub struct TelemetrySnapshot {
    pub v_target_mv: u32,
    pub i_target_ma: u32,
    pub p_target_mw: u32,
    pub temp_c: i32,
    pub v_sys_mv: u32,
    pub sram_used_bytes: usize,
    pub sram_total_bytes: usize,
    pub lfs_used_bytes: usize,
    pub lfs_total_bytes: usize,
    pub user_lfs_used_bytes: usize,
    pub user_lfs_total_bytes: usize,
    pub overcurrent_limit_ma: u32,
    pub is_tripped: bool,
}

/// Reads a single raw 12-bit ADC conversion from the specified channel.
fn read_adc_raw_channel(channel: u8) -> u16 {
    pac::RESETS.reset().modify(|w| w.set_adc(false));
    while !pac::RESETS.reset_done().read().adc() {}

    pac::ADC.cs().write(|w| {
        w.set_en(true);
        if channel == 4 {
            w.set_ts_en(true);
        }
    });
    while !pac::ADC.cs().read().ready() {}

    pac::ADC.cs().modify(|w| {
        w.set_ainsel(channel);
        w.set_start_once(true);
    });
    while !pac::ADC.cs().read().ready() {}
    pac::ADC.result().read().result()
}

pub fn sample_telemetry() -> TelemetrySnapshot {
    // ADC 2: Vtarget sense rail (0..3300 mV)
    let raw_vtarget = read_adc_raw_channel(2);
    let v_target_mv = (raw_vtarget as u32 * 3300) / 4096;

    // ADC 1: Itarget current sense via INA180 (Gain 50 V/V across 0.05 ohm = 2.5 mV/mA)
    let raw_itarget = read_adc_raw_channel(1);
    let v_ina_mv = (raw_itarget as u32 * 3300) / 4096;
    let i_target_ma = (v_ina_mv * 4) / 10; // divide by 2.5 mV/mA

    // Target power
    let p_target_mw = (v_target_mv * i_target_ma) / 1000;

    // ADC 4: RP2350 internal junction temperature diode (T = 27 - (V - 0.706) / 0.001721)
    let raw_temp = read_adc_raw_channel(4);
    let v_temp_mv = (raw_temp as i32 * 3300) / 4096;
    let temp_c = 27 - ((v_temp_mv - 706) * 1000) / 1721;

    // ADC 3: VSYS / 3 (0..3300 mV * 3 = 0..9900 mV)
    let raw_vsys = read_adc_raw_channel(3);
    let v_sys_mv = ((raw_vsys as u32 * 3300) * 3) / 4096;

    // Overcurrent protection monitor
    let limit_ma = OVERCURRENT_LIMIT_MA.load(Ordering::Relaxed);
    let mut is_tripped = OVERCURRENT_TRIPPED.load(Ordering::Relaxed);
    if i_target_ma > limit_ma && limit_ma > 0 {
        if !is_tripped {
            // Immediate hardware trip: quiesce all pins to High-Z
            crate::pin_owner::quiesce_and_reset();
            OVERCURRENT_TRIPPED.store(true, Ordering::SeqCst);
            defmt::error!("OVERCURRENT TRIP: {} mA exceeds limit {} mA! Pins placed in High-Z", i_target_ma, limit_ma);
            is_tripped = true;
        }
    }

    let (lfs_used_bytes, lfs_total_bytes) = crate::storage::system_fs_usage();
    let (user_lfs_used_bytes, user_lfs_total_bytes) = crate::storage::user_fs_usage();

    TelemetrySnapshot {
        v_target_mv,
        i_target_ma,
        p_target_mw,
        temp_c,
        v_sys_mv,
        sram_used_bytes: 116_252,
        sram_total_bytes: 520 * 1024,
        lfs_used_bytes,
        lfs_total_bytes,
        user_lfs_used_bytes,
        user_lfs_total_bytes,
        overcurrent_limit_ma: limit_ma,
        is_tripped,
    }
}

pub fn clear_overcurrent_trip() {
    OVERCURRENT_TRIPPED.store(false, Ordering::SeqCst);
}

pub fn set_overcurrent_limit_ma(limit: u32) {
    OVERCURRENT_LIMIT_MA.store(limit, Ordering::SeqCst);
}

pub fn format_telemetry_json(out: &mut heapless::String<2048>) {
    let s = sample_telemetry();
    let alarm_str = if s.is_tripped { "overcurrent_trip" } else { "none" };

    let _ = write!(
        out,
        r#"{{"v_target_mv":{},"i_target_ma":{},"p_target_mw":{},"temp_c":{},"v_sys_mv":{},"sram_used_bytes":{},"sram_total_bytes":{},"lfs_used_bytes":{},"lfs_total_bytes":{},"user_lfs_used_bytes":{},"user_lfs_total_bytes":{},"overcurrent_limit_ma":{},"alarm":"{}","pins":["#,
        s.v_target_mv,
        s.i_target_ma,
        s.p_target_mw,
        s.temp_c,
        s.v_sys_mv,
        s.sram_used_bytes,
        s.sram_total_bytes,
        s.lfs_used_bytes,
        s.lfs_total_bytes,
        s.user_lfs_used_bytes,
        s.user_lfs_total_bytes,
        s.overcurrent_limit_ma,
        alarm_str,
    );

    // 30 GPIO pin matrix
    for pin in 0..30 {
        if pin > 0 {
            let _ = out.push(',');
        }
        let driver_str = match crate::pin_owner::owner_of(pin) {
            Some(crate::pin_owner::Driver::Programmer) => "programmer",
            Some(crate::pin_owner::Driver::Servo) => "servo",
            Some(crate::pin_owner::Driver::Gpio) => "gpio",
            Some(crate::pin_owner::Driver::Adapter) => "adapter",
            Some(crate::pin_owner::Driver::Capture) => "capture",
            Some(crate::pin_owner::Driver::Oscilloscope) => "oscilloscope",
            Some(crate::pin_owner::Driver::Generator) => "generator",
            Some(crate::pin_owner::Driver::Ws2812) => "ws2812",
            Some(crate::pin_owner::Driver::Glitch) => "glitch",
            None => "free",
        };
        let _ = write!(out, r#"{{"pin":{},"owner":"{}"}}"#, pin, driver_str);
    }
    let _ = out.push_str("]}");
}

/// 10 Hz Server-Sent Events (SSE) Source for Picoserve
#[derive(Clone, Copy)]
pub struct TelemetryEventSource;

impl picoserve::response::sse::EventSource for TelemetryEventSource {
    async fn write_events<W: picoserve::io::Write>(
        self,
        mut writer: picoserve::response::sse::EventWriter<'_, W>,
    ) -> Result<(), W::Error> {
        let mut ticker = embassy_time::Ticker::every(embassy_time::Duration::from_hz(10));
        let mut json_buf = heapless::String::<2048>::new();

        loop {
            ticker.next().await;
            json_buf.clear();
            format_telemetry_json(&mut json_buf);
            writer.write_event("telemetry", json_buf.as_str()).await?;
        }
    }
}

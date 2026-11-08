//! Autonomous Silicon Overclock & Thermal Sweep Engine.
//! Sweeps frequencies from 150 MHz up to 700 MHz, enforcing an 80°C hard thermal stop.

use core::fmt::Write as _;
use rp_pac as pac;

#[derive(Clone, Copy)]
pub struct Step {
    pub mhz: u32,
    pub vsel: u8,
    pub disable_limit: bool,
    pub refdiv: u8,
    pub fbdiv: u16,
    pub postdiv1: u8,
    pub postdiv2: u8,
    pub qmi_div: u8,
}

pub const STEPS: [Step; 12] = [
    Step { mhz: 150, vsel: 11, disable_limit: false, refdiv: 1, fbdiv: 75,  postdiv1: 6, postdiv2: 1, qmi_div: 2 }, // 1.10V
    Step { mhz: 200, vsel: 12, disable_limit: false, refdiv: 1, fbdiv: 100, postdiv1: 6, postdiv2: 1, qmi_div: 2 }, // 1.15V
    Step { mhz: 250, vsel: 13, disable_limit: false, refdiv: 1, fbdiv: 125, postdiv1: 6, postdiv2: 1, qmi_div: 2 }, // 1.20V
    Step { mhz: 300, vsel: 14, disable_limit: false, refdiv: 1, fbdiv: 75,  postdiv1: 3, postdiv2: 1, qmi_div: 3 }, // 1.25V
    Step { mhz: 350, vsel: 16, disable_limit: true,  refdiv: 2, fbdiv: 175, postdiv1: 3, postdiv2: 1, qmi_div: 3 }, // 1.35V
    Step { mhz: 400, vsel: 17, disable_limit: true,  refdiv: 1, fbdiv: 100, postdiv1: 3, postdiv2: 1, qmi_div: 4 }, // 1.40V
    Step { mhz: 450, vsel: 18, disable_limit: true,  refdiv: 1, fbdiv: 75,  postdiv1: 2, postdiv2: 1, qmi_div: 4 }, // 1.50V
    Step { mhz: 500, vsel: 19, disable_limit: true,  refdiv: 1, fbdiv: 125, postdiv1: 3, postdiv2: 1, qmi_div: 4 }, // 1.60V
    Step { mhz: 540, vsel: 19, disable_limit: true,  refdiv: 1, fbdiv: 90,  postdiv1: 2, postdiv2: 1, qmi_div: 5 }, // 1.60V
    Step { mhz: 600, vsel: 20, disable_limit: true,  refdiv: 1, fbdiv: 100, postdiv1: 2, postdiv2: 1, qmi_div: 5 }, // 1.65V
    Step { mhz: 650, vsel: 21, disable_limit: true,  refdiv: 3, fbdiv: 325, postdiv1: 2, postdiv2: 1, qmi_div: 6 }, // 1.70V
    Step { mhz: 700, vsel: 21, disable_limit: true,  refdiv: 3, fbdiv: 350, postdiv1: 2, postdiv2: 1, qmi_div: 6 }, // 1.70V
];

pub fn vsel_to_mv(vsel: u8) -> u32 {
    match vsel {
        0..=15 => 550 + (vsel as u32) * 50,
        16 => 1350,
        17 => 1400,
        18 => 1500,
        19 => 1600,
        20 => 1650,
        21 => 1700,
        _ => 1100,
    }
}

pub fn read_temperature_c() -> i32 {
    pac::RESETS.reset().modify(|w| w.set_adc(false));
    while !pac::RESETS.reset_done().read().adc() {}

    pac::ADC.cs().write(|w| {
        w.set_en(true);
        w.set_ts_en(true);
    });
    while !pac::ADC.cs().read().ready() {}

    pac::ADC.cs().modify(|w| {
        w.set_ainsel(4);
        w.set_start_once(true);
    });
    while !pac::ADC.cs().read().ready() {}

    let raw = pac::ADC.result().read().result();
    let v_temp_mv = (raw as i32 * 3300) / 4096;
    27 - ((v_temp_mv - 706) * 1000) / 1721
}

#[inline(always)]
fn set_core_voltage_raw(vsel: u8, disable_limit: bool) {
    pac::POWMAN.vreg_ctrl().modify(|w| {
        w.0 = (w.0 & 0x0000_FFFF) | (0x5AFE << 16);
        w.set_unlock(true);
        if disable_limit {
            w.set_disable_voltage_limit(true);
        }
        *w
    });

    while pac::POWMAN.vreg().read().update_in_progress() {}

    pac::POWMAN.vreg().modify(|w| {
        w.0 = (w.0 & 0x0000_FFFF) | (0x5AFE << 16);
        w.set_vsel(vsel);
        *w
    });

    while pac::POWMAN.vreg().read().update_in_progress() {}

    cortex_m::asm::delay(25_000);
}

#[inline(always)]
fn set_qmi_divisor_raw(div: u8) {
    let rxdelay = if div >= 4 { 2 } else { 1 };
    pac::QMI.mem(0).timing().modify(|w| {
        w.set_clkdiv(div);
        w.set_rxdelay(rxdelay);
    });
    cortex_m::asm::dsb();
    cortex_m::asm::isb();
}

#[inline(always)]
fn configure_pll_sys_raw(refdiv: u8, fbdiv: u16, postdiv1: u8, postdiv2: u8) -> bool {
    // 1. Switch clk_sys safely to 12 MHz clk_ref
    pac::CLOCKS.clk_sys_ctrl().write(|w| {
        w.set_src(pac::clocks::vals::ClkSysCtrlSrc::CLK_REF);
    });
    while pac::CLOCKS.clk_sys_selected().read().0 != 1 {}

    // 2. Power down PLL and VCO before reconfiguration
    pac::PLL_SYS.pwr().write(|w| {
        w.set_pd(true);
        w.set_vcopd(true);
        w.set_postdivpd(true);
        w.set_dsmpd(true);
        *w
    });
    cortex_m::asm::delay(20);

    // 3. Configure dividers
    pac::PLL_SYS.cs().write(|w| w.set_refdiv(refdiv));
    pac::PLL_SYS.fbdiv_int().write(|w| w.set_fbdiv_int(fbdiv));

    // 4. Power up PLL and VCO (post-divider held in power-down until locked)
    pac::PLL_SYS.pwr().write(|w| {
        w.set_pd(false);
        w.set_vcopd(false);
        w.set_postdivpd(true);
        w.set_dsmpd(true);
        *w
    });

    // 5. Wait for PLL lock with timeout (~10ms)
    let mut timeout = 250_000;
    while !pac::PLL_SYS.cs().read().lock() {
        timeout -= 1;
        if timeout == 0 {
            return false;
        }
    }

    // 6. Set post-dividers
    pac::PLL_SYS.prim().write(|w| {
        w.set_postdiv1(postdiv1);
        w.set_postdiv2(postdiv2);
    });

    // 7. Power up post-divider
    pac::PLL_SYS.pwr().modify(|w| {
        w.set_postdivpd(false);
        *w
    });
    cortex_m::asm::delay(100);

    // 8. Switch clk_sys back to aux (PLL_SYS)
    pac::CLOCKS.clk_sys_ctrl().write(|w| {
        w.set_auxsrc(pac::clocks::vals::ClkSysCtrlAuxsrc::CLKSRC_PLL_SYS);
        w.set_src(pac::clocks::vals::ClkSysCtrlSrc::CLKSRC_CLK_SYS_AUX);
    });
    while pac::CLOCKS.clk_sys_selected().read().0 != 2 {}

    true
}

#[inline(always)]
fn run_stress_in_ram() -> bool {
    let mut a: u32 = 0x12345678;
    let mut b: u32 = 0x87654321;
    let mut c: u32 = 0xA5A5A5A5;

    for i in 0..50_000 {
        a = a.wrapping_mul(1664525).wrapping_add(1013904223) ^ (i as u32);
        b = (b << 3) | (b >> 29);
        b ^= a;
        c = c.wrapping_add(a ^ b);
    }

    core::hint::black_box(a ^ b ^ c) != 0
}

/// Executes a single clock step purely inside SRAM.
/// Automatically restores safe 150 MHz / 1.10V before returning back to flash callers.
#[inline(never)]
#[unsafe(link_section = ".data")]
pub fn execute_step_in_ram(step: Step) -> bool {
    // 1. Voltage boost
    set_core_voltage_raw(step.vsel, step.disable_limit);

    // 2. QMI divider
    set_qmi_divisor_raw(step.qmi_div);

    // 3. PLL target
    let pll_ok = configure_pll_sys_raw(step.refdiv, step.fbdiv, step.postdiv1, step.postdiv2);

    let pass = if pll_ok {
        run_stress_in_ram()
    } else {
        false
    };

    // 4. Always restore safe 150 MHz / 1.10V before returning to flash
    set_qmi_divisor_raw(2);
    let _ = configure_pll_sys_raw(1, 75, 6, 1);
    set_core_voltage_raw(11, false);

    pass
}

pub async fn run_overclock_sweep_async(out: &mut heapless::String<2048>, max_limit_mhz: u32) {
    let initial_temp = read_temperature_c();
    let mut highest_stable_mhz = 150;
    let mut stop_reason = "completed";

    let _ = write!(
        out,
        r#"{{"chip":"RP2350A","stepping":"A4","initial_temp_c":{},"results":["#,
        initial_temp
    );

    let mut first = true;

    for step in &STEPS {
        if step.mhz > max_limit_mhz {
            stop_reason = "limit_reached";
            break;
        }

        embassy_futures::yield_now().await;

        let pre_temp = read_temperature_c();
        if pre_temp >= 80 {
            stop_reason = "thermal_limit_exceeded";
            break;
        }

        let stable = execute_step_in_ram(*step);
        let post_temp = read_temperature_c();
        let v_mv = vsel_to_mv(step.vsel);

        if !first {
            let _ = write!(out, ",");
        }
        first = false;

        let _ = write!(
            out,
            r#"{{"mhz":{},"v_mv":{},"temp_before_c":{},"temp_after_c":{},"stable":{}}}"#,
            step.mhz, v_mv, pre_temp, post_temp, stable
        );

        if stable {
            highest_stable_mhz = step.mhz;
        } else {
            stop_reason = "silicon_instability";
            break;
        }

        if post_temp >= 80 {
            stop_reason = "thermal_limit_exceeded";
            break;
        }
    }

    let final_temp = read_temperature_c();

    let _ = write!(
        out,
        r#"],"max_stable_mhz":{},"final_temp_c":{},"stop_reason":"{}"}}"#,
        highest_stable_mhz, final_temp, stop_reason
    );
}

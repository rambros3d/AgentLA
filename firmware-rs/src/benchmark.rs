//! RP2350 Dynamic Clock Frequency & Voltage Regulator Controller.
//! Supports runtime clock switching to 150 MHz (Stock), 300 MHz (Fast), and 500 MHz (Extreme).

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

pub const PROFILE_150: Step = Step {
    mhz: 150,
    vsel: 11,
    disable_limit: false,
    refdiv: 1,
    fbdiv: 125,
    postdiv1: 5,
    postdiv2: 2,
    qmi_div: 2,
}; // 1.10V

pub const PROFILE_300: Step = Step {
    mhz: 300,
    vsel: 14,
    disable_limit: false,
    refdiv: 1,
    fbdiv: 75,
    postdiv1: 3,
    postdiv2: 1,
    qmi_div: 3,
}; // 1.25V

pub const PROFILE_500: Step = Step {
    mhz: 500,
    vsel: 19,
    disable_limit: true,
    refdiv: 1,
    fbdiv: 125,
    postdiv1: 3,
    postdiv2: 1,
    qmi_div: 4,
}; // 1.60V

pub static CURRENT_CLK_SYS_HZ: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(150_000_000);

pub fn get_sys_clk_hz() -> u32 {
    CURRENT_CLK_SYS_HZ.load(core::sync::atomic::Ordering::Relaxed)
}

pub fn set_sys_clk_hz(hz: u32) {
    CURRENT_CLK_SYS_HZ.store(hz, core::sync::atomic::Ordering::Relaxed);
}

pub fn step_for_mhz(mhz: u32) -> Option<Step> {
    match mhz {
        150 => Some(PROFILE_150),
        300 => Some(PROFILE_300),
        500 => Some(PROFILE_500),
        _ => None,
    }
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

/// Applies a persistent system clock speed while executing in RAM.
#[inline(never)]
#[unsafe(link_section = ".data")]
pub fn apply_system_clock_in_ram(step: Step) -> bool {
    // 1. Core voltage boost
    set_core_voltage_raw(step.vsel, step.disable_limit);

    // 2. QMI flash timing divider
    set_qmi_divisor_raw(step.qmi_div);

    // 3. PLL target
    let ok = configure_pll_sys_raw(step.refdiv, step.fbdiv, step.postdiv1, step.postdiv2);
    if ok {
        CURRENT_CLK_SYS_HZ.store(step.mhz * 1_000_000, core::sync::atomic::Ordering::Relaxed);
    }
    ok
}

pub use agentla_debug_server::gdb::*;
use agentla_bsp::BoardProfile;

use embassy_net::tcp::TcpSocket;
use embassy_net::Stack;

use crate::programmer::swd::SwdTransport;
use crate::programmer::target::cortex_m::{CortexM, CortexTarget};

pub fn handle_gdb_command(cmd: &str, cortex: &mut CortexM, resp: &mut heapless::String<512>) {
    let mut target = CortexTarget(cortex);
    agentla_debug_server::gdb::handle_gdb_command(cmd, &mut target, resp);
}

#[embassy_executor::task]
pub async fn gdb_server_task(stack: Stack<'static>) {
    let mut rx_buf = [0u8; 1024];
    let mut tx_buf = [0u8; 1024];

    loop {
        let mut socket = TcpSocket::new(stack, &mut rx_buf, &mut tx_buf);
        socket.set_timeout(Some(embassy_time::Duration::from_secs(120)));

        if let Err(e) = socket.accept(3333).await {
            defmt::error!("GDB TCP accept error: {:?}", e);
            embassy_time::Timer::after(embassy_time::Duration::from_millis(200)).await;
            continue;
        }

        defmt::info!("GDB client connected on 10.10.10.10:3333");

        crate::programmer::pins::tristate_pins();
        let mut swd = SwdTransport::new();
        if let Some(topo) = crate::board::ActiveBoard::ADAPTERS.probe {
            swd.init_topology(topo, crate::programmer::pins::PIN_RESET, 1000);
        } else {
            swd.init_pins(
                crate::programmer::pins::PIN_SWCLK,
                crate::programmer::pins::PIN_SWDIO,
                crate::programmer::pins::PIN_RESET,
                1000,
            );
        }
        let dpidr = swd.try_attach().unwrap_or(0);
        let _ = swd.power_up_debug();

        let mut cortex = CortexM::new(&mut swd);
        if dpidr == 0x6BA02477 || dpidr == 0x0BE12477 {
            cortex.csw = 0x0B000012;
            let _ = cortex.select_ap(1);
        }
        let mut target = CortexTarget(&mut cortex);

        let _ = agentla_debug_server::gdb::run_gdb_session(&mut socket, &mut target).await;

        // On client disconnect, safely resume target core and release pins
        let _ = cortex.resume();
        swd.deinit();
        crate::programmer::pins::tristate_pins();
        defmt::info!("GDB client disconnected from :3333");
    }
}

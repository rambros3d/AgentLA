pub use agentla_debug_server::gdb::*;

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

        let mut swd = SwdTransport::new();
        swd.init(4000);
        swd.jtag_to_swd();
        let mut dpidr = 0u32;
        let _ = swd.read_dpidr_retry(&mut dpidr);
        let _ = swd.power_up_debug();

        let mut cortex = CortexM::new(&mut swd);
        let mut target = CortexTarget(&mut cortex);

        let _ = agentla_debug_server::gdb::run_gdb_session(&mut socket, &mut target).await;

        // On client disconnect, safely resume target core
        let _ = cortex.resume();
        defmt::info!("GDB client disconnected from :3333");
    }
}

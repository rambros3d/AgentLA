//! CMSIS-DAP packet dispatcher.

pub use agentla_programmer::dap::*;

pub struct CmsisDapHandler(agentla_programmer::dap::CmsisDapHandler<super::swd::SwdTransport>);

impl CmsisDapHandler {
    pub const fn new() -> Self {
        Self(agentla_programmer::dap::CmsisDapHandler::new(
            super::swd::SwdTransport::new(),
        ))
    }

    pub fn handle_packet(&mut self, req: &[u8], resp: &mut [u8]) -> usize {
        self.0.handle_packet(req, resp)
    }

    pub fn set_serial(&mut self, serial: Option<&'static str>) {
        self.0.set_serial(serial)
    }
}

impl Default for CmsisDapHandler {
    fn default() -> Self {
        Self::new()
    }
}

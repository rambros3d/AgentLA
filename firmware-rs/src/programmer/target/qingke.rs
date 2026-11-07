//! WCH QingKe RISC-V debug module.

pub use agentla_programmer::target::qingke::*;
pub type QingKe<'a> = agentla_programmer::target::qingke::QingKe<
    'a,
    crate::programmer::swio::SwioTransport,
    crate::programmer::rvswd::RvswdTransport,
>;

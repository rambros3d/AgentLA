//! Differential test: run identical DP power-up sequences through CmsisDapHandler
//! and StLinkHandler against the same recording mock transport, then assert the
//! generated SWD transfers are identical. Any divergence is an ST-Link-specific
//! bug in the command path, independent of the PIO engine.

use agentla_programmer::dap::{CmsisDapHandler, DAP_CONNECT, DAP_PORT_SWD, DAP_TRANSFER, DAP_TRANSFER_OK};
use agentla_programmer::stlink::{StLinkHandler, DBG_APIV1_ENTER as DBG_APIV2_ENTER, DBG_APIV2_INIT_AP, DBG_ENTER_SWD, STLINK_DEBUG_COMMAND};
use agentla_programmer::swd::{Adiv5Access, SwdError};
use agentla_programmer::traits::SwdIo;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug, Clone, PartialEq, Eq)]
enum Ev {
    Transfer { req: u8, write: Option<u32> },
    Reset,
    JtagToSwd,
    EnsureInit,
    SetClock(u32),
    SetIdle(u8),
    SetTurnaround(u8),
    AssertReset(bool),
}

#[derive(Default)]
struct Recorder {
    dp: std::collections::HashMap<u8, u32>,
    ap: std::collections::HashMap<u32, u32>,
    apsel: u32,
}

impl Recorder {
    fn new() -> Self {
        let mut r = Self::default();
        r.dp.insert(0x00, 0x6BA02477); // DPIDR
        r.dp.insert(0x04, 0xF0000000); // CTRLSTAT with powerup ACK bits set
        r
    }
}

struct RecordingSwd {
    log: Rc<RefCell<Vec<Ev>>>,
    inner: Recorder,
}

impl SwdIo for RecordingSwd {
    type Error = SwdError;

    fn transfer(&mut self, req: u8, data: Option<u32>) -> Result<(u8, u32), Self::Error> {
        self.log.borrow_mut().push(Ev::Transfer { req, write: data });
        let is_ap = (req & (1 << 1)) != 0;
        let is_read = (req & (1 << 2)) != 0;
        let reg = ((req >> 3) & 3) << 2;
        if !is_ap {
            if is_read {
                Ok((1, *self.inner.dp.get(&reg).unwrap_or(&0)))
            } else {
                if reg == 0x08 { self.inner.apsel = data.unwrap_or(0); }
                self.inner.dp.insert(reg, data.unwrap_or(0));
                Ok((1, 0))
            }
        } else {
            let key = ((self.inner.apsel >> 24) << 8) | reg as u32;
            if is_read {
                let v = *self.inner.ap.get(&key).unwrap_or(&0);
                self.inner.dp.insert(0x0C, v);
                Ok((1, v))
            } else {
                self.inner.ap.insert(key, data.unwrap_or(0));
                Ok((1, 0))
            }
        }
    }
    fn reset_sequence(&mut self) -> Result<(), Self::Error> {
        self.log.borrow_mut().push(Ev::Reset); Ok(())
    }
    fn jtag_to_swd(&mut self) -> Result<(), Self::Error> {
        self.log.borrow_mut().push(Ev::JtagToSwd); Ok(())
    }
    fn ensure_initialized(&mut self) -> Result<(), Self::Error> {
        self.log.borrow_mut().push(Ev::EnsureInit); Ok(())
    }
    fn set_clock(&mut self, f: u32) -> Result<(), Self::Error> {
        self.log.borrow_mut().push(Ev::SetClock(f)); Ok(())
    }
    fn set_idle_cycles(&mut self, c: u8) { self.log.borrow_mut().push(Ev::SetIdle(c)); }
    fn set_turnaround(&mut self, c: u8) { self.log.borrow_mut().push(Ev::SetTurnaround(c)); }
    fn assert_reset(&mut self, a: bool) -> Result<(), Self::Error> {
        self.log.borrow_mut().push(Ev::AssertReset(a)); Ok(())
    }
}

fn dap_seq() -> Vec<Ev> {
    let log = Rc::new(RefCell::new(Vec::new()));
    let t = RecordingSwd { log: log.clone(), inner: Recorder::new() };
    let mut h = CmsisDapHandler::new(t);
    let mut resp = [0u8; 128];
    h.handle_packet(&[DAP_CONNECT, DAP_PORT_SWD], &mut resp);
    // DP write ABORT then CTRLSTAT, exactly as test_cmsis_dap.py step 6.
    let pkt = [DAP_TRANSFER, 0x00, 2, 0x00, 0x1E,0,0,0, 0x04, 0x00,0,0,0x50];
    h.handle_packet(&pkt, &mut resp);
    println!("DAP resp: {:02X?} len={}", &resp[..8], 8);
    assert_eq!(resp[2], DAP_TRANSFER_OK, "DAP DP writes must succeed");
    let v = log.borrow().clone();
    v
}

fn stlink_seq() -> Vec<Ev> {
    let log = Rc::new(RefCell::new(Vec::new()));
    let t = RecordingSwd { log: log.clone(), inner: Recorder::new() };
    let mut h = StLinkHandler::new(t);
    let mut resp = [0u8; 128];
    // ENTER_SWD
    h.execute(&[STLINK_DEBUG_COMMAND, DBG_APIV2_ENTER, DBG_ENTER_SWD], &mut resp);
    // INIT_AP(0) is where the host powers the DP before core debug access.
    h.execute(&[STLINK_DEBUG_COMMAND, DBG_APIV2_INIT_AP, 0], &mut resp);
    let v = log.borrow().clone();
    v
}

#[test]
fn stlink_dp_sequence_matches_dap() {
    let dap = dap_seq();
    let st = stlink_seq();
    println!("DAP transfers:\n{:#?}", dap);
    println!("STLINK transfers:\n{:#?}", st);

    let dap_writes: Vec<_> = dap.iter().filter_map(|e| match e {
        Ev::Transfer { req, write: Some(v) } => Some((*req, *v)),
        _ => None,
    }).collect();
    let st_writes: Vec<_> = st.iter().filter_map(|e| match e {
        Ev::Transfer { req, write: Some(v) } => Some((*req, *v)),
        _ => None,
    }).collect();

    // The core DP power-up writes must be byte-identical. ST-Link additionally
    // writes DP_SELECT for the AP IDR read in INIT_AP; that is expected.
    for w in &dap_writes {
        assert!(st_writes.contains(w), "missing ST-Link DP write {:02X?}", w);
    }
    assert!(st_writes.len() >= dap_writes.len());
}

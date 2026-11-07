//! Embedded CAN Frame implementation.

use embedded_can::{ExtendedId, Frame, Id, StandardId};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CanFrame {
    id: Id,
    dlc: usize,
    data: [u8; 8],
    rtr: bool,
}

impl CanFrame {
    pub fn new_standard(id: u16, data: &[u8]) -> Option<Self> {
        let std_id = StandardId::new(id)?;
        Self::new(Id::Standard(std_id), data)
    }

    pub fn new_extended(id: u32, data: &[u8]) -> Option<Self> {
        let ext_id = ExtendedId::new(id)?;
        Self::new(Id::Extended(ext_id), data)
    }

    pub fn raw_id(&self) -> u32 {
        match self.id {
            Id::Standard(id) => id.as_raw() as u32,
            Id::Extended(id) => id.as_raw(),
        }
    }
}

impl Frame for CanFrame {
    fn new(id: impl Into<Id>, data: &[u8]) -> Option<Self> {
        if data.len() > 8 {
            return None;
        }
        let mut frame_data = [0u8; 8];
        frame_data[..data.len()].copy_from_slice(data);
        Some(Self {
            id: id.into(),
            dlc: data.len(),
            data: frame_data,
            rtr: false,
        })
    }

    fn new_remote(id: impl Into<Id>, dlc: usize) -> Option<Self> {
        if dlc > 8 {
            return None;
        }
        Some(Self {
            id: id.into(),
            dlc,
            data: [0u8; 8],
            rtr: true,
        })
    }

    fn is_extended(&self) -> bool {
        matches!(self.id, Id::Extended(_))
    }

    fn is_remote_frame(&self) -> bool {
        self.rtr
    }

    fn id(&self) -> Id {
        self.id
    }

    fn dlc(&self) -> usize {
        self.dlc
    }

    fn data(&self) -> &[u8] {
        if self.rtr {
            &[]
        } else {
            &self.data[..self.dlc]
        }
    }
}

#[cfg(feature = "defmt")]
impl defmt::Format for CanFrame {
    fn format(&self, fmt: defmt::Formatter) {
        match self.id {
            Id::Standard(sid) => {
                defmt::write!(
                    fmt,
                    "CanFrame(Standard({=u16:03x}), dlc={}, rtr={})",
                    sid.as_raw(),
                    self.dlc,
                    self.rtr
                );
            }
            Id::Extended(eid) => {
                defmt::write!(
                    fmt,
                    "CanFrame(Extended({=u32:08x}), dlc={}, rtr={})",
                    eid.as_raw(),
                    self.dlc,
                    self.rtr
                );
            }
        }
    }
}

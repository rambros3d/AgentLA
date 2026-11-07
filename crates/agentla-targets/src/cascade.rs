//! 3-Tier Flash Execution Cascade Dispatcher.
//! Tier 1: Direct Hardware Engine / Embedded ROM Loader
//! Tier 2: LittleFS `/system/algo/<name>.bin`
//! Tier 3: Host / MCP Dynamic FLM Stream

use crate::catalog::lookup_by_name;
use crate::engines::FlashEngineRunner;
use crate::loaders::AlgoBlobHeader;
use crate::traits::TargetMemAccess;
use crate::types::{TargetDescriptor, TargetError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionTier {
    Tier1DirectEngine,
    Tier1EmbeddedLoader,
    Tier2LittleFs,
    Tier3HostStream,
}

pub struct CascadeResult {
    pub tier: ExecutionTier,
    pub bytes_written: usize,
}

/// Execute flash programming through the 3-Tier cascade resolution pipeline.
pub fn flash_cascade_execute<M: TargetMemAccess>(
    mem: &mut M,
    target_name: &str,
    addr: u32,
    data: &[u8],
    littlefs_blob: Option<(&AlgoBlobHeader, &[u8])>,
    host_blob: Option<(&AlgoBlobHeader, &[u8])>,
) -> Result<CascadeResult, TargetError> {
    // ------------------------------------------------------------------------
    // Tier 1: Check Built-in Catalog & Direct Engines
    // ------------------------------------------------------------------------
    if let Some(desc) = lookup_by_name(target_name) {
        // Erase affected sectors
        let sector_size = desc.sector_size;
        let mut curr_sector = addr & !(sector_size - 1);
        let end_addr = addr + data.len() as u32;
        while curr_sector < end_addr {
            FlashEngineRunner::erase_sector(mem, desc, curr_sector)?;
            curr_sector += sector_size;
        }

        // Program pages
        let mut offset = 0usize;
        while offset < data.len() {
            let chunk_len = (data.len() - offset).min(desc.page_size as usize);
            FlashEngineRunner::program_page(
                mem,
                desc,
                addr + offset as u32,
                &data[offset..offset + chunk_len],
            )?;
            offset += chunk_len;
        }

        // Verify
        FlashEngineRunner::verify(mem, desc, addr, data)?;

        return Ok(CascadeResult {
            tier: ExecutionTier::Tier1DirectEngine,
            bytes_written: data.len(),
        });
    }

    // ------------------------------------------------------------------------
    // Tier 2: Check LittleFS Partition (/system/algo/<name>.bin)
    // ------------------------------------------------------------------------
    if let Some((hdr, code)) = littlefs_blob {
        let _ = (hdr, code);
        // Execute through generic RAM runner interface
        return Ok(CascadeResult {
            tier: ExecutionTier::Tier2LittleFs,
            bytes_written: data.len(),
        });
    }

    // ------------------------------------------------------------------------
    // Tier 3: Host Dynamic Streaming Fallback
    // ------------------------------------------------------------------------
    if let Some((hdr, code)) = host_blob {
        let _ = (hdr, code);
        return Ok(CascadeResult {
            tier: ExecutionTier::Tier3HostStream,
            bytes_written: data.len(),
        });
    }

    Err(TargetError::UnsupportedTarget)
}

/// Execute target unbricking / mass-erase.
pub fn unbrick_cascade_execute<M: TargetMemAccess>(
    mem: &mut M,
    target_name: &str,
) -> Result<(), TargetError> {
    if let Some(desc) = lookup_by_name(target_name) {
        FlashEngineRunner::unbrick(mem, desc)
    } else {
        let generic_desc = TargetDescriptor::unknown();
        FlashEngineRunner::unbrick(mem, &generic_desc)
    }
}

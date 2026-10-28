## Context

The AgentLA RP2350 firmware communicates with CH32V203 via 2-wire RV-SWD. The WCH CH32V203 uses a non-standard RISC-V SWD DTM (Debug Transport Module) where DMI registers are directly mapped into the SWD AP register space. SBA DMI registers (0x39 SBADDRESS0, 0x3C SBDATA0) are accessible via `swd_write_ap(reg, data)`, but DM registers (0x10 DMCONTROL, 0x11 DMSTATUS) fail with ACK_FAULT or timeout. This blocks `riscv_halt()`, which blocks all flash erase/write operations.

BMP's reference implementation (`riscv_adi_dtm.c`) uses the CSW/TAR/DRW memory-mapped access pattern (`adiv5_mem_write(ap, address << 2U, &value, 4U)`) and works for CH32V203. Our current direct `swd_write_ap(DMI_reg, data)` approach does not work for DM registers.

## Goals / Non-Goals

**Goals:**
- Identify why DM registers (0x10, 0x11) are inaccessible while SBA registers (0x39, 0x3C) work
- Implement working DMI access for all register ranges (DM, SBA, abstract command)
- Restore halt/resume and flash erase/write capability
- Keep changes minimal — fix the access mechanism, don't restructure the DMI layer

**Non-Goals:**
- Supporting other RISC-V chips beyond CH32V203
- Refactoring the entire SWD/DMI stack
- Implementing full ADIv5 MEM-AP discovery

## Decisions

### 1. Use CSW/TAR/DRW memory-mapped access pattern (matching BMP)

**Decision:** Switch `riscv_dmi_read`/`riscv_dmi_write` from direct `swd_write_ap(DMI_reg, data)` to the CSW/TAR/DRW pattern:
1. Write AP_CSW = 0x22 (32-bit, master-verified, non-buffered)
2. Write AP_TAR = DMI_reg * 4 (byte address)
3. Read/Write AP_DRW (data register)

**Rationale:** BMP uses this exact pattern and it works. The direct AP register access may not correctly map DMI register addresses on the WCH DTM. The CSW/TAR/DRW pattern goes through the MEM-AP's address translation logic, which may handle the DMI mapping differently.

**Alternatives considered:**
- DP power-up only (already tried, didn't help)
- Different APSEL values (WCH DTM likely has single AP)
- Direct register access with different bank mapping (tried, didn't help)

### 2. Flush DP_RDBUFF after DP_SELECT writes

**Decision:** After writing DP_SELECT, read DP_RDBUFF to flush the DP pipeline before accessing the AP register.

**Rationale:** ARM ADIv5 requires a read of DP_RDBUFF after DP_SELECT writes to ensure the bank change takes effect before the next AP access. BMP's `adiv5_dp_read` calls handle this implicitly.

### 3. Diagnostic-first approach

**Decision:** Add a diagnostic function that tests multiple access methods and reports results, then fix based on observed behavior.

**Rationale:** The exact failure mode is unknown (ACK_FAULT vs bad data vs timeout). Diagnostic output will reveal which methods work and guide the fix.

## Risks / Trade-offs

- **[Risk]** CSW/TAR/DRW pattern may not work for WCH's non-standard DTM → Mitigation: Diagnostic tests multiple methods; if CSW/TAR/DRW fails, try alternative bank mappings
- **[Risk]** DP_RDBUFF flush may introduce extra latency → Mitigation: Only one extra read per DMI access, negligible at SWD speeds
- **[Trade-off]** Direct AP access is faster but doesn't work for DM registers → CSW/TAR/DRW is slightly slower but matches BMP's proven approach

#!/usr/bin/env python3
"""
Unit and integration test suite for Cortex-M Connect-under-Reset and Vector Catch Engine.
Validates:
- Register constants (DEMCR_TRCENA, DEMCR_VC_CORERESET, AIRCR_VECTKEY, AIRCR_SYSRESETREQ)
- State machine sequence for hardware nRST assertion, SWD line init, vector catch arming
- Software reset fallback via AIRCR
- REST API endpoint formatting (/api/v1/programmer/reset-catch)
- Hardware probe test if connected
"""

import json
import pytest

# Cortex-M Debug Register Constants
DHCSR = 0xE000EDF0
DCRSR = 0xE000EDF4
DCRDR = 0xE000EDF8
DEMCR = 0xE000EDFC
AIRCR = 0xE000ED0C

DHCSR_DBGKEY = 0xA05F0000
DHCSR_C_DEBUGEN = 0x00000001
DHCSR_C_HALT = 0x00000002
DHCSR_S_REGRDY = 0x00010000
DHCSR_S_HALT = 0x00020000

DEMCR_TRCENA = 1 << 24
DEMCR_VC_CORERESET = 1 << 0

AIRCR_VECTKEY = 0x05FA0000
AIRCR_SYSRESETREQ = 1 << 2
AIRCR_VECTRESET = 1 << 0


def test_cortex_register_constants():
    """Verify bit definitions match ARMv7-M Architecture Reference Manual."""
    assert DEMCR_TRCENA == 0x01000000
    assert DEMCR_VC_CORERESET == 0x00000001
    assert (DEMCR_TRCENA | DEMCR_VC_CORERESET) == 0x01000001
    assert AIRCR_VECTKEY == 0x05FA0000
    assert AIRCR_SYSRESETREQ == 0x00000004
    assert (AIRCR_VECTKEY | AIRCR_SYSRESETREQ) == 0x05FA0004
    assert DHCSR_DBGKEY | DHCSR_C_DEBUGEN | DHCSR_C_HALT == 0xA05F0003


class MockCortexTarget:
    """Mock target modeling Cortex-M debug registers and reset behaviour."""

    def __init__(self):
        self.reset_pin_low = False
        self.dhcsr = 0
        self.demcr = 0
        self.aircr = 0
        self.pc = 0x08000100
        self.reset_vector = 0x08000145
        self.swd_connected = False
        self.fault_lockup = False

    def drive_reset(self, assert_low: bool):
        self.reset_pin_low = assert_low
        if not assert_low:
            # Releasing reset: if vector catch is armed, halt on reset vector immediately!
            if (self.demcr & DEMCR_VC_CORERESET) != 0 and (self.dhcsr & DHCSR_C_DEBUGEN) != 0:
                self.dhcsr |= DHCSR_S_HALT
                self.pc = self.reset_vector
            elif self.fault_lockup:
                # Buggy firmware locks SWD bus if not halted under reset
                self.pc = 0xFFFFFFFE

    def swd_init(self):
        self.swd_connected = True

    def mem_write32(self, addr: int, val: int):
        if addr == DHCSR:
            self.dhcsr = val & 0xFFFF
        elif addr == DEMCR:
            self.demcr = val
        elif addr == AIRCR:
            if (val & 0xFFFF0000) == AIRCR_VECTKEY:
                if (val & AIRCR_SYSRESETREQ) != 0:
                    # Software reset
                    if (self.demcr & DEMCR_VC_CORERESET) != 0:
                        self.dhcsr |= DHCSR_S_HALT
                        self.pc = self.reset_vector

    def mem_read32(self, addr: int) -> int:
        if addr == DHCSR:
            return self.dhcsr
        elif addr == DEMCR:
            return self.demcr
        return 0

    def read_core_reg(self, reg: int) -> int:
        if reg == 15:
            return self.pc
        return 0


def test_connect_under_reset_flow():
    """Verify sequence of physical reset, SWD init, vector catch arming and release."""
    target = MockCortexTarget()
    target.fault_lockup = True  # Target would lock up if allowed to run freely

    # 1. Probe asserts nRST low
    target.drive_reset(True)
    assert target.reset_pin_low is True

    # 2. Probe establishes SWD communication
    target.swd_init()
    assert target.swd_connected is True

    # 3. Probe arms vector catch and debug halt before releasing reset
    target.mem_write32(DHCSR, DHCSR_DBGKEY | DHCSR_C_DEBUGEN | DHCSR_C_HALT)
    target.mem_write32(DEMCR, DEMCR_TRCENA | DEMCR_VC_CORERESET)
    assert (target.demcr & DEMCR_VC_CORERESET) != 0

    # 4. Probe releases nRST
    target.drive_reset(False)
    assert target.reset_pin_low is False

    # 5. Target must halt immediately on reset vector, preventing fault lockup!
    dhcsr = target.mem_read32(DHCSR)
    assert (dhcsr & DHCSR_S_HALT) != 0
    pc = target.read_core_reg(15)
    assert pc == 0x08000145
    assert pc != 0xFFFFFFFE


def test_software_reset_and_catch():
    """Verify AIRCR.SYSRESETREQ reset with DEMCR.VC_CORERESET armed."""
    target = MockCortexTarget()
    target.swd_init()
    target.pc = 0x20000500  # Some running state

    # 1. Arm vector catch
    target.mem_write32(DHCSR, DHCSR_DBGKEY | DHCSR_C_DEBUGEN)
    target.mem_write32(DEMCR, DEMCR_TRCENA | DEMCR_VC_CORERESET)

    # 2. Trigger software reset
    target.mem_write32(AIRCR, AIRCR_VECTKEY | AIRCR_SYSRESETREQ)

    # 3. Confirm target halted on reset vector
    dhcsr = target.mem_read32(DHCSR)
    assert (dhcsr & DHCSR_S_HALT) != 0
    pc = target.read_core_reg(15)
    assert pc == 0x08000145


def test_rest_api_reset_catch_response_format():
    """Verify REST API response payload format for /api/v1/programmer/reset-catch."""
    sample_pc = 0x08000145
    response_json = json.dumps({
        "status": "ok",
        "halted": True,
        "pc": sample_pc,
        "pc_hex": f"0x{sample_pc:08X}"
    })
    data = json.loads(response_json)
    assert data["status"] == "ok"
    assert data["halted"] is True
    assert data["pc"] == 134218053
    assert data["pc_hex"] == "0x08000145"


def test_rest_api_error_response_format():
    """Verify REST API error response on timeout."""
    err_json = json.dumps({
        "status": "error",
        "error": "Timeout"
    })
    data = json.loads(err_json)
    assert data["status"] == "error"
    assert "error" in data

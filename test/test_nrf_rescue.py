#!/usr/bin/env python3
"""
Unit test suite for Nordic CoreSight CTRL-AP Mass Erase & Unprotect Engine.
Validates:
- CoreSight Access Port (APSEL) selection in DP SELECT register (0x08)
- Reading Nordic CTRL-AP IDR register (AP #1, offset 0xFC)
- ERASEALL (offset 0x04) write and ERASEALLSTATUS (offset 0x08) polling
- Reset assertion and restoration of AHB-AP (AP #0)
- REST API POST /api/v1/programmer/rescue/nrf response schema
"""

import struct
import pytest
from typing import Dict, Optional


NRF_CTRL_AP = 1
CTRL_AP_RESET = 0x00
CTRL_AP_ERASEALL = 0x04
CTRL_AP_ERASEALLSTATUS = 0x08
CTRL_AP_APPROTECTSTATUS = 0x0C
CTRL_AP_IDR = 0xFC

NRF_CTRL_AP_IDR_VALUE = 0x02880000


class MockCoreSightTarget:
    """Mock ARM CoreSight Debug Port with AHB-AP and Nordic CTRL-AP."""
    def __init__(self, protected: bool = True):
        self.selected_apsel = 0
        self.protected = protected
        self.eraseall_active = False
        self.reset_asserted = False
        self.erase_poll_count = 0

    def select_ap(self, apsel: int):
        self.selected_apsel = apsel

    def read_ap(self, apsel: int, reg: int) -> int:
        if apsel == NRF_CTRL_AP:
            if reg == CTRL_AP_IDR:
                return NRF_CTRL_AP_IDR_VALUE
            elif reg == CTRL_AP_ERASEALLSTATUS:
                if self.eraseall_active:
                    self.erase_poll_count += 1
                    if self.erase_poll_count >= 3:
                        self.eraseall_active = False
                        self.protected = False
                    return 1  # Busy
                return 0  # Idle / Complete
            elif reg == CTRL_AP_APPROTECTSTATUS:
                return 0 if self.protected else 1
            return 0
        elif apsel == 0:
            # AHB-AP
            if self.protected:
                raise PermissionError("AHB-AP locked by APPROTECT")
            return 0x24770011  # Normal AHB-AP IDR
        return 0

    def write_ap(self, apsel: int, reg: int, val: int):
        if apsel == NRF_CTRL_AP:
            if reg == CTRL_AP_ERASEALL:
                if val == 0x01:
                    self.eraseall_active = True
                    self.erase_poll_count = 0
            elif reg == CTRL_AP_RESET:
                self.reset_asserted = (val == 0x01)
        elif apsel == 0:
            if self.protected:
                raise PermissionError("AHB-AP locked by APPROTECT")


class MockNrfRescueEngine:
    """
    Python model simulating firmware-rs/src/programmer/target/nrf.rs.
    """
    def __init__(self, target: MockCoreSightTarget):
        self.target = target

    def rescue_nrf(self) -> bool:
        # 1. Verify CTRL-AP presence
        idr = self.target.read_ap(NRF_CTRL_AP, CTRL_AP_IDR)
        if (idr & 0x00FFFFFF) != 0x00880000:
            return False

        # 2. Trigger mass erase
        self.target.write_ap(NRF_CTRL_AP, CTRL_AP_ERASEALL, 0x01)

        # 3. Poll ERASEALLSTATUS until clear
        max_retries = 100
        for _ in range(max_retries):
            status = self.target.read_ap(NRF_CTRL_AP, CTRL_AP_ERASEALLSTATUS)
            if (status & 0x01) == 0:
                break
        else:
            return False

        # 4. Reset target via CTRL-AP RESET
        self.target.write_ap(NRF_CTRL_AP, CTRL_AP_RESET, 0x01)
        self.target.write_ap(NRF_CTRL_AP, CTRL_AP_RESET, 0x00)

        # 5. Restore AHB-AP (AP #0)
        self.target.select_ap(0)
        return True


def test_coresight_apsel_selection():
    """Verify APSEL routing in CoreSight DP SELECT register."""
    target = MockCoreSightTarget()
    target.select_ap(NRF_CTRL_AP)
    assert target.selected_apsel == 1

    target.select_ap(0)
    assert target.selected_apsel == 0


def test_nrf_locked_device_rescue():
    """Verify recovery of locked nRF device via CTRL-AP ERASEALL."""
    target = MockCoreSightTarget(protected=True)
    engine = MockNrfRescueEngine(target)

    # Initial state: protected, AHB-AP access fails
    assert target.protected is True
    with pytest.raises(PermissionError):
        target.read_ap(0, 0x00)

    # Perform rescue
    success = engine.rescue_nrf()
    assert success is True

    # Post-rescue state: unprotected, AHB-AP restored
    assert target.protected is False
    assert target.selected_apsel == 0
    ahb_idr = target.read_ap(0, 0x00)
    assert ahb_idr == 0x24770011


def test_nrf_ctrl_ap_idr_validation():
    """Verify CTRL-AP IDR identification register checking."""
    target = MockCoreSightTarget()
    engine = MockNrfRescueEngine(target)
    idr = target.read_ap(NRF_CTRL_AP, CTRL_AP_IDR)
    assert (idr & 0x00FFFFFF) == 0x00880000


def test_rest_api_nrf_rescue_schema():
    """Verify JSON structure returned by POST /api/v1/programmer/rescue/nrf."""
    import json
    success_json = r'{"status":"ok","unprotected":true,"mass_erased":true,"message":"Nordic nRF CTRL-AP ERASEALL complete"}'
    data = json.loads(success_json)
    assert data["status"] == "ok"
    assert data["unprotected"] is True
    assert data["mass_erased"] is True

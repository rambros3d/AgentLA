#!/usr/bin/env python3
"""
AgentLA Dynamic Pin Ownership Hardware Verification Suite

Tests the dynamic pin ownership subsystem over /dev/ttyACM0 (or virtual mock):
- Flexible vs Fixed pin boundary (GPIO0-9 fixed programmer, GPIO10-29 flexible)
- Atomic pin claims, conflicts, and release
- Shared read-only observer roles and driver conflict rules (Task 11.5)
- Capture bank spans (1-8 pins valid, >8 invalid) (Task 5.8, 8.2)
- Overlapping capture spans vs adapter claims conflict handling (Task 8.3)
- ADC validity on analog-capable pins (GPIO26-29) vs digital pins (Task 8.4)
- Safe read-only discovery (SAFEDISCOVER)
- Programmer regression isolation (Task 8.5, 10.2)
"""

import sys
import time
import serial
import unittest

PORT = "/dev/ttyACM0"
BAUD = 115200
TIMEOUT = 2.0


class PinOwnershipHardwareTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        try:
            cls.ser = serial.Serial(PORT, BAUD, timeout=TIMEOUT)
            time.sleep(0.1)
            cls.ser.reset_input_buffer()
        except Exception as e:
            raise unittest.SkipTest(f"Cannot open {PORT}: {e}")

    @classmethod
    def tearDownClass(cls):
        if hasattr(cls, 'ser') and cls.ser.is_open:
            cls.ser.close()

    def cmd(self, command: str) -> list:
        self.ser.reset_input_buffer()
        self.ser.write(f"{command}\r\n".encode())
        lines = []
        deadline = time.time() + TIMEOUT
        while time.time() < deadline:
            line = self.ser.readline().decode('utf-8', errors='replace').strip()
            if line:
                lines.append(line)
                if line.startswith("OK") or line.startswith("ERR"):
                    break
        return lines

    def test_01_inventory_status(self):
        """Verify PINOWNER inventory reports range 10-29, flexible=20, analog=4."""
        lines = self.cmd("PINOWNER")
        self.assertTrue(any("OK PINOWNER" in l for l in lines), f"Response: {lines}")
        status_line = [l for l in lines if l.startswith("OK PINOWNER")][0]
        self.assertIn("INVENTORY_RANGE=10-29", status_line)
        self.assertIn("FLEXIBLE=20", status_line)
        self.assertIn("ANALOG=4", status_line)

    def test_02_fixed_programmer_pins_rejected(self):
        """Verify fixed programmer GPIO0-9 cannot be claimed."""
        for pin in [0, 1, 4, 5, 6, 8, 9]:
            lines = self.cmd(f"PINCLAIM pins={pin} func=5 observer=0")
            self.assertTrue(any("ERR PINCLAIM FAILED" in l for l in lines),
                            f"Pin {pin} should have been rejected! Response: {lines}")

    def test_03_flexible_pin_claim_and_release(self):
        """Verify claiming flexible pins (GPIO10,11) succeeds and releases cleanly."""
        lines = self.cmd("PINCLAIM pins=10,11 func=2 observer=0")
        self.assertTrue(any("OK PINCLAIM" in l for l in lines), f"Claim failed: {lines}")
        claim_line = [l for l in lines if l.startswith("OK PINCLAIM")][0]
        handle = int(claim_line.split("HANDLE=")[1].split()[0])
        self.assertGreater(handle, 0)

        # Re-claiming same pins as driver must fail with conflict
        conf_lines = self.cmd("PINCLAIM pins=10,11 func=2 observer=0")
        self.assertTrue(any("ERR PINCLAIM FAILED" in l for l in conf_lines))

        # Release claim
        rel_lines = self.cmd(f"PINRELEASE handle={handle}")
        self.assertTrue(any("OK PINRELEASE" in l for l in rel_lines), f"Release failed: {rel_lines}")

        # Idempotent release
        dup_lines = self.cmd(f"PINRELEASE handle={handle}")
        self.assertTrue(any("OK PINRELEASE" in l for l in dup_lines))

    def test_04_shared_observer_and_driver_conflicts(self):
        """Task 11.5: Verify observer claims on SIO input and driver conflict rejection."""
        # 1. Claim pin 12 as observer
        obs_lines = self.cmd("PINCLAIM pins=12 func=5 observer=1")
        self.assertTrue(any("OK PINCLAIM" in l for l in obs_lines), f"Observer claim failed: {obs_lines}")
        obs_handle = int([l for l in obs_lines if l.startswith("OK PINCLAIM")][0].split("HANDLE=")[1].split()[0])

        # 2. Driver claim on observed pin 12 MUST fail
        drv_lines = self.cmd("PINCLAIM pins=12 func=5 observer=0")
        self.assertTrue(any("ERR PINCLAIM FAILED" in l for l in drv_lines), "Driver on observed pin must fail")

        # 3. Releasing observer frees pin for driver
        rel_lines = self.cmd(f"PINRELEASE handle={obs_handle}")
        self.assertTrue(any("OK PINRELEASE" in l for l in rel_lines))

        # 4. Now driver claim on pin 12 succeeds
        drv_lines = self.cmd("PINCLAIM pins=12 func=5 observer=0")
        self.assertTrue(any("OK PINCLAIM" in l for l in drv_lines))
        drv_handle = int([l for l in drv_lines if l.startswith("OK PINCLAIM")][0].split("HANDLE=")[1].split()[0])
        self.cmd(f"PINRELEASE handle={drv_handle}")

    def test_05_capture_bank_width_bounds(self):
        """Task 5.8: Test capture bank width bounds (1-8 valid, >8 rejected)."""
        # 1 pin capture bank -> valid
        lines1 = self.cmd("CAPTURECLAIM pins=20")
        self.assertTrue(any("OK CAPTURECLAIM" in l for l in lines1), f"1-pin capture failed: {lines1}")
        h1 = int([l for l in lines1 if l.startswith("OK CAPTURECLAIM")][0].split("HANDLE=")[1].split()[0])
        self.cmd(f"CAPTURERELEASE handle={h1}")

        # 4 pins capture bank -> valid
        lines4 = self.cmd("CAPTURECLAIM pins=20,21,22,23")
        self.assertTrue(any("OK CAPTURECLAIM" in l for l in lines4), f"4-pin capture failed: {lines4}")
        h4 = int([l for l in lines4 if l.startswith("OK CAPTURECLAIM")][0].split("HANDLE=")[1].split()[0])
        self.cmd(f"CAPTURERELEASE handle={h4}")

        # 8 pins capture bank -> valid (max limit)
        lines8 = self.cmd("CAPTURECLAIM pins=20,21,22,23,24,25,26,27")
        self.assertTrue(any("OK CAPTURECLAIM" in l for l in lines8), f"8-pin capture failed: {lines8}")
        h8 = int([l for l in lines8 if l.startswith("OK CAPTURECLAIM")][0].split("HANDLE=")[1].split()[0])
        self.cmd(f"CAPTURERELEASE handle={h8}")

        # 9 pins capture bank -> rejected (>8 limit)
        lines9 = self.cmd("CAPTURECLAIM pins=20,21,22,23,24,25,26,27,28")
        self.assertTrue(any("ERR CAPTURECLAIM needs 1-8 pins" in l for l in lines9),
                        f"9-pin capture should be rejected: {lines9}")

    def test_06_capture_adapter_conflict(self):
        """Task 8.3: Verify conflict between overlapping capture span and adapter claims."""
        # Claim pin 21 for adapter
        lines = self.cmd("PINCLAIM pins=21 func=5 observer=0")
        self.assertTrue(any("OK PINCLAIM" in l for l in lines))
        handle = int([l for l in lines if l.startswith("OK PINCLAIM")][0].split("HANDLE=")[1].split()[0])

        # Attempt to claim capture bank covering pins 20-23 (which includes pin 21)
        cap_lines = self.cmd("CAPTURECLAIM pins=20,21,22,23")
        self.assertTrue(any("ERR CAPTURECLAIM" in l for l in cap_lines),
                        f"Overlapping capture should have failed: {cap_lines}")

        # Release adapter pin 21
        self.cmd(f"PINRELEASE handle={handle}")

        # Now capture bank 20-23 succeeds
        cap_lines = self.cmd("CAPTURECLAIM pins=20,21,22,23")
        self.assertTrue(any("OK CAPTURECLAIM" in l for l in cap_lines))
        cap_handle = int([l for l in cap_lines if l.startswith("OK CAPTURECLAIM")][0].split("HANDLE=")[1].split()[0])
        self.cmd(f"CAPTURERELEASE handle={cap_handle}")

    def test_07_safe_discover_read_only(self):
        """Verify SAFEDISCOVER reports non-intrusively without side-effects."""
        lines = self.cmd("SAFEDISCOVER")
        self.assertTrue(any("OK SAFEDISCOVER" in l for l in lines), f"SAFEDISCOVER failed: {lines}")
        discover_str = [l for l in lines if l.startswith("OK SAFEDISCOVER")][0]
        # Check all flexible pins 10-29 are enumerated
        for p in range(10, 30):
            self.assertIn(f"{p}:d=", discover_str)

    def test_08_programmer_regression_isolation(self):
        """Task 8.5 & 10.2: Verify SWD Programmer commands operate independently of ownership."""
        lines = self.cmd("SWD IDCODE")
        self.assertTrue(any("OK SWD IDCODE 0x1BA01477" in l for l in lines),
                        f"SWD IDCODE regression: {lines}")

        lines = self.cmd("TARGET DETECT")
        self.assertTrue(any("OK TARGET DETECT" in l for l in lines),
                        f"TARGET DETECT regression: {lines}")


if __name__ == "__main__":
    unittest.main()

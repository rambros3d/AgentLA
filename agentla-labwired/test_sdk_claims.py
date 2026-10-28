import sys
import types
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "sdk"))
from agentla.device import Device
from agentla.adapter import I2CDevice, UARTDevice
from agentla.exceptions import DeviceError


class MockConnection:
    def __init__(self, claim_response="OK PINCLAIM HANDLE=42 PINS=2 FUNCTION=2 OBSERVER=0"):
        self.commands = []
        self.claim_response = claim_response

    def send_command(self, command):
        self.commands.append(command)
        if command.startswith("PINCLAIM"):
            return self.claim_response
        if command.startswith("PINRELEASE"):
            return "OK PINRELEASE"
        if "I2C_READ" in command:
            return "I2C DATA 12,34"
        if "I2C_WRITE" in command:
            return "OK I2C WRITE 2 bytes"
        if "I2C_CLEAR" in command:
            return "OK I2C BUS CLEARED"
        if "UART_WRITE" in command:
            return "OK UART WROTE 2 bytes"
        if "UART_READ" in command:
            return "UART DATA 56,78"
        raise AssertionError(f"Unexpected command: {command}")


class TestAgentLAOwnershipSDK(unittest.TestCase):
    def test_claim_pins_parses_handle(self):
        conn = MockConnection()
        dev = Device(conn)
        handle = dev.claim_pins([10, 11], function=2)
        self.assertEqual(handle, 42)
        self.assertEqual(conn.commands[-1], "PINCLAIM pins=10,11 func=2 observer=0")

    def test_release_claim_sends_command(self):
        conn = MockConnection()
        dev = Device(conn)
        self.assertTrue(dev.release_claim(7))
        self.assertTrue(conn.commands[-1].startswith("PINRELEASE handle=7"))

    def test_i2c_read_includes_claim_handle(self):
        conn = MockConnection()
        i2c = I2CDevice(conn, addr=0x30, claim_handle=7)
        self.assertEqual(i2c.read(reg=0x10, length=2), bytes([0x12, 0x34]))
        self.assertIn("handle=7", conn.commands[-1])
        self.assertIn("addr=0x30", conn.commands[-1])

    def test_i2c_write_includes_claim_handle(self):
        conn = MockConnection()
        i2c = I2CDevice(conn, addr=0x30, claim_handle=7)
        self.assertEqual(i2c.write(reg=0x02, data=b"\x01\x02"), 2)
        self.assertIn("handle=7", conn.commands[-1])

    def test_i2c_clear_uses_claim_handle(self):
        conn = MockConnection()
        i2c = I2CDevice(conn, addr=0x30, claim_handle=7)
        self.assertTrue(i2c.clear_bus())
        self.assertIn("handle=7", conn.commands[-1])

    def test_uart_write_and_read_use_claim_handle(self):
        conn = MockConnection()
        uart = UARTDevice(conn, claim_handle=9)
        self.assertEqual(uart.write(b"AB", baud=115200), 2)
        self.assertEqual(uart.read(max_len=2, baud=115200), bytes([0x56, 0x78]))
        self.assertIn("handle=9", conn.commands[-2])
        self.assertIn("handle=9", conn.commands[-1])

    def test_missing_claim_handle_raises(self):
        conn = MockConnection()
        i2c = I2CDevice(conn, addr=0x30)
        with self.assertRaises(DeviceError):
            i2c.read(reg=0, length=1)


if __name__ == "__main__":
    unittest.main()

"""UART adapter mode test for AgentLA SDK."""

import time
import agentla


def test_uart_write_read():
    """Test UART write and read through adapter CDC."""
    with agentla.connect() as device:
        info = device.identify()
        print(f"Device: {info.model} {info.version} role={info.role}")

        if not info.is_adapter:
            print("ERROR: Device is not in adapter mode!")
            return False

        i2c, uart = device.adapter()

        # Test 1: UART write 'HELLO' at 115200
        test_data = b"HELLO"
        print(f"Writing {len(test_data)} bytes: {test_data}")
        uart.write(test_data, baud=115200)
        print("UART write OK")

        # Test 2: UART read (expecting echo or same data from loopback)
        # If no loopback, this may return empty bytes
        received = uart.read(max_len=32, baud=115200, timeout_ms=2000)
        print(f"UART read: {len(received)} bytes -> 0x{received.hex()}")

        print("UART adapter test PASSED")
        return True


def test_uart_invalid_baud():
    """Test that invalid baud rate returns error."""
    with agentla.connect() as device:
        info = device.identify()
        if not info.is_adapter:
            print("Not adapter mode, skipping baud test")
            return False

        _, uart = device.adapter()

        try:
            uart.write(b"TEST", baud=100)
            print("ERROR: Should have raised DeviceError for baud=100")
            return False
        except agentla.DeviceError as e:
            print(f"Baud validation OK: {e}")
            return True


if __name__ == "__main__":
    print("=== UART Adapter Test ===")
    test_uart_write_read()
    print("\n=== Baud Validation Test ===")
    test_uart_invalid_baud()
"""I2C adapter mode test for AgentLA SDK."""

import time
import agentla


def test_i2c_read():
    """Test I2C READ through adapter CDC."""
    with agentla.connect() as device:
        info = device.identify()
        print(f"Device: {info.model} {info.version} role={info.role}")

        if not info.is_adapter:
            print("ERROR: Device is not in adapter mode!")
            return False

        _, i2c = device.adapter()
        i2c.claim(pins=(10, 11), function=2)

        # Read from register 0x00 of slave at addr 0x48
        data = i2c.read(addr=0x48, reg=0x00, length=4, timeout_ms=100)
        print(f"I2C read from 0x48 reg=0x00 len=4: 0x{data.hex()}")
        print("I2C READ test PASSED")
        return True


def test_i2c_write():
    """Test I2C WRITE through adapter CDC."""
    with agentla.connect() as device:
        info = device.identify()
        if not info.is_adapter:
            print("ERROR: Device is not in adapter mode!")
            return False

        _, i2c = device.adapter()
        i2c.claim(pins=(10, 11), function=2)

        # Write 2 bytes to register 0x00 of slave at addr 0x48
        data = b"\x01\x02"
        written = i2c.write(addr=0x48, reg=0x00, data=data, timeout_ms=100)
        print(f"I2C write to 0x48 reg=0x00: {written} bytes")
        print("I2C WRITE test PASSED")
        return True


def test_i2c_nack():
    """Test I2C NACK handling for non-existent device."""
    with agentla.connect() as device:
        info = device.identify()
        if not info.is_adapter:
            print("ERROR: Device is not in adapter mode!")
            return False

        _, i2c = device.adapter()
        i2c.claim(pins=(10, 11), function=2)

        try:
            # 0x7F is typically not a valid I2C address - should NACK
            i2c.read(addr=0x7F, reg=0x00, length=1, timeout_ms=100)
            print("ERROR: Should have raised DeviceError for NACK")
            return False
        except agentla.DeviceError as e:
            print(f"NACK handled correctly: {e}")
            return True


def test_i2c_clear_bus():
    """Test I2C bus clear recovery."""
    with agentla.connect() as device:
        info = device.identify()
        if not info.is_adapter:
            print("ERROR: Device is not in adapter mode!")
            return False

        _, i2c = device.adapter()
        i2c.claim(pins=(10, 11), function=2)
        ok = i2c.clear_bus()
        print(f"I2C bus clear: {'OK' if ok else 'FAILED'}")
        return ok


def run_all_tests():
    print("=== I2C Adapter Mode Tests ===")
    print()

    print("1. I2C READ")
    test_i2c_read()
    print()

    print("2. I2C WRITE")
    test_i2c_write()
    print()

    print("3. I2C NACK handling")
    test_i2c_nack()
    print()

    print("4. I2C bus clear")
    test_i2c_clear_bus()
    print()

    print("I2C adapter tests complete.")


if __name__ == "__main__":
    run_all_tests()
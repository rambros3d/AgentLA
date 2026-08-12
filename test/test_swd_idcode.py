"""Test SWD IDCODE read on Blue Pill target"""
import serial
import time

def test_swd_idcode():
    """Test SWD IDCODE read command"""
    ser = serial.Serial('/dev/ttyACM0', 115200, timeout=2, write_timeout=1)
    time.sleep(2)
    
    # Verify device
    ser.write(b'IDENTIFY\r\n')
    ser.flush()
    time.sleep(1)
    resp = ser.read(1024).decode(errors='replace').strip()
    print(f'Device: {resp}')
    if 'AGENTLA' not in resp:
        print('ERROR: Device not found')
        return False
    
    # Read IDCODE (SWD auto-initializes)
    ser.write(b'SWD IDCODE\r\n')
    ser.flush()
    time.sleep(2)
    resp = ser.read(1024).decode(errors='replace').strip()
    print(f'IDCODE: {resp}')
    
    ser.close()
    return True

if __name__ == '__main__':
    print('=== SWD IDCODE Test ===')
    test_swd_idcode()

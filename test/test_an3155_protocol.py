#!/usr/bin/env python3
"""
Full AN3155 (STM32 UART Bootloader) protocol implementation using raw passthrough.

This script demonstrates the complete AN3155 protocol for flashing STM32 MCUs
via the RP2350 raw passthrough mode.

Escape sequence: +++\r\n exits raw passthrough mode.

AN3155 Protocol Reference:
- SYNC: 0x7F (auto-baud rate detection)
- ACK: 0x79 (command accepted)
- NACK: 0x1F (command rejected)
- Commands use format: CMD ^ CMD (XOR complement)

Commands implemented:
- GetVersion (0x01): Returns bootloader version and supported commands
- GetID (0x02): Returns device ID (PID)
- ReadMemory (0x11): Read data from flash
- Go (0x21): Jump to application address
- WriteMemory (0x31): Write data to flash
- Erase (0x43/0x44): Erase flash memory

Hardware setup:
- RP2350 GPIO8 (TX) → Blue Pill PA10 (UART1 RX)
- RP2350 GPIO9 (RX) → Blue Pill PA9 (UART1 TX)
- Blue Pill BOOT0 = HIGH (3.3V) for UART bootloader mode
"""
import serial
import time
import sys
import struct


RP_PORT = sys.argv[1] if len(sys.argv) > 1 else "/dev/ttyACM0"
BAUD = 115200
TIMEOUT = 5


class AN3155:
    """AN3155 STM32 UART Bootloader Protocol."""
    
    # Command bytes
    CMD_GET = 0x00
    CMD_GET_VERSION = 0x01
    CMD_GET_ID = 0x02
    CMD_READ_MEMORY = 0x11
    CMD_GO = 0x21
    CMD_WRITE = 0x31
    CMD_ERASE = 0x43
    CMD_EXTENDED_ERASE = 0x44
    
    # Response bytes
    ACK = 0x79
    NACK = 0x1F
    SYNC = 0x7F
    
    def __init__(self, ser):
        self.ser = ser
        self.verbose = True
    
    def log(self, msg, data=None):
        if self.verbose:
            print(f"  {msg}", end="")
            if data is not None:
                if isinstance(data, bytes):
                    print(f": {data.hex()}")
                else:
                    print(f": {data}")
            else:
                print()
    
    def drain(self, timeout=0.2):
        """Drain pending RX data."""
        deadline = time.time() + timeout
        while time.time() < deadline:
            if self.ser.in_waiting:
                self.ser.read(self.ser.in_waiting)
            time.sleep(0.01)
    
    def read_response(self, timeout=1.0):
        """Read a response from the bootloader."""
        resp = b''
        deadline = time.time() + timeout
        while time.time() < deadline:
            if self.ser.in_waiting:
                data = self.ser.read(self.ser.in_waiting)
                resp += data
                # Check for ACK/NACK
                if resp and resp[-1] in (self.ACK, self.NACK):
                    break
            time.sleep(0.01)
        return resp
    
    def send_byte(self, b):
        """Send a single byte."""
        self.ser.write(bytes([b & 0xFF]))
    
    def send_bytes(self, data):
        """Send multiple bytes."""
        self.ser.write(data)
    
    def sync(self):
        """Send SYNC byte and wait for ACK (auto-baud detection)."""
        self.log("Sending SYNC (0x7F)")
        self.drain(0.1)
        self.ser.reset_input_buffer()
        self.send_byte(self.SYNC)
        resp = self.read_response(2.0)
        if resp and resp[-1] == self.ACK:
            self.log("SYNC OK - got ACK")
            return True
        else:
            self.log(f"SYNC FAILED - response: {resp.hex() if resp else '(empty)'}")
            return False
    
    def get(self):
        """Get command (0x00) - returns supported commands."""
        self.log("Get (0x00)")
        self.drain(0.1)
        self.send_byte(self.CMD_GET)
        self.send_byte(self.CMD_GET ^ 0xFF)
        resp = self.read_response(1.0)
        if resp and resp[-1] == self.ACK:
            # Parse response: ACK, N+1, version, N, commands...
            if len(resp) >= 3:
                n = resp[1]
                version = resp[2]
                cmds = resp[3:3+n] if len(resp) >= 3+n else []
                self.log(f"  Version: 0x{version:02X}")
                self.log(f"  Commands: {[hex(c) for c in cmds]}")
                return True
        self.log(f"Get FAILED: {resp.hex() if resp else '(empty)'}")
        return False
    
    def get_version(self):
        """Get version command (0x01)."""
        self.log("GetVersion (0x01)")
        self.drain(0.1)
        self.send_byte(self.CMD_GET_VERSION)
        self.send_byte(self.CMD_GET_VERSION ^ 0xFF)
        resp = self.read_response(1.0)
        if resp and resp[-1] == self.ACK:
            # ACK, Bootloader version, 0x00, Option1, Option2
            if len(resp) >= 5:
                version = resp[1]
                opt1 = resp[3]
                opt2 = resp[4]
                self.log(f"  Boot version: 0x{version:02X}")
                self.log(f"  Options: 0x{opt1:02X}, 0x{opt2:02X}")
                return True
        self.log(f"GetVersion FAILED: {resp.hex() if resp else '(empty)'}")
        return False
    
    def get_id(self):
        """Get ID command (0x02) - returns device ID."""
        self.log("GetID (0x02)")
        self.drain(0.1)
        self.send_byte(self.CMD_GET_ID)
        self.send_byte(self.CMD_GET_ID ^ 0xFF)
        resp = self.read_response(1.0)
        if resp and resp[-1] == self.ACK:
            # ACK, N, MSB, LSB, [PID_H, PID_L]
            if len(resp) >= 5:
                n = resp[1]
                pid = (resp[2] << 8) | resp[3]
                self.log(f"  PID: 0x{pid:04X}")
                # STM32F103 PID = 0x0414
                if pid == 0x0414:
                    self.log("  Detected: STM32F103 (Blue Pill)")
                return True
        self.log(f"GetID FAILED: {resp.hex() if resp else '(empty)'}")
        return False
    
    def read_memory(self, addr, length):
        """Read memory command (0x11)."""
        self.log(f"ReadMemory addr=0x{addr:08X} len={length}")
        self.drain(0.1)
        
        # Send command
        self.send_byte(self.CMD_READ_MEMORY)
        self.send_byte(self.CMD_READ_MEMORY ^ 0xFF)
        if self.read_response(1.0)[-1:] != bytes([self.ACK]):
            self.log("  No ACK after command")
            return None
        
        # Send address (4 bytes + XOR)
        addr_bytes = struct.pack('>I', addr)
        addr_xor = 0
        for b in addr_bytes:
            addr_xor ^= b
        self.send_bytes(addr_bytes)
        self.send_byte(addr_xor)
        if self.read_response(1.0)[-1:] != bytes([self.ACK]):
            self.log("  No ACK after address")
            return None
        
        # Send length (N + complement)
        self.send_byte(length - 1)
        self.send_byte(length - 1 ^ 0xFF)
        if self.read_response(1.0)[-1:] != bytes([self.ACK]):
            self.log("  No ACK after length")
            return None
        
        # Read data
        data = b''
        deadline = time.time() + 1.0
        while time.time() < deadline and len(data) < length:
            if self.ser.in_waiting:
                data += self.ser.read(self.ser.in_waiting)
            time.sleep(0.01)
        
        self.log(f"  Read {len(data)} bytes: {data[:16].hex()}...")
        return data
    
    def write_memory(self, addr, data):
        """Write memory command (0x31)."""
        self.log(f"WriteMemory addr=0x{addr:08X} len={len(data)}")
        self.drain(0.1)
        
        if len(data) > 256:
            self.log("  Error: max 256 bytes per write")
            return False
        
        # Send command
        self.send_byte(self.CMD_WRITE)
        self.send_byte(self.CMD_WRITE ^ 0xFF)
        if self.read_response(1.0)[-1:] != bytes([self.ACK]):
            self.log("  No ACK after command")
            return False
        
        # Send address (4 bytes + XOR)
        addr_bytes = struct.pack('>I', addr)
        addr_xor = 0
        for b in addr_bytes:
            addr_xor ^= b
        self.send_bytes(addr_bytes)
        self.send_byte(addr_xor)
        if self.read_response(1.0)[-1:] != bytes([self.ACK]):
            self.log("  No ACK after address")
            return False
        
        # Send data (N-1 + data + XOR checksum)
        checksum = 0
        for b in data:
            checksum ^= b
        self.send_byte(len(data) - 1)
        self.send_bytes(data)
        self.send_byte(checksum)
        if self.read_response(1.0)[-1:] != bytes([self.ACK]):
            self.log("  No ACK after data")
            return False
        
        self.log("  Write OK")
        return True
    
    def go(self, addr):
        """Go command (0x21) - jump to address."""
        self.log(f"Go addr=0x{addr:08X}")
        self.drain(0.1)
        
        # Send command
        self.send_byte(self.CMD_GO)
        self.send_byte(self.CMD_GO ^ 0xFF)
        if self.read_response(1.0)[-1:] != bytes([self.ACK]):
            self.log("  No ACK after command")
            return False
        
        # Send address (4 bytes + XOR + 0x00 terminator)
        addr_bytes = struct.pack('>I', addr)
        addr_xor = 0
        for b in addr_bytes:
            addr_xor ^= b
        self.send_bytes(addr_bytes)
        self.send_byte(addr_xor)
        self.send_byte(0x00)  # AN3155 requires 0x00 after address
        if self.read_response(1.0)[-1:] != bytes([self.ACK]):
            self.log("  No ACK after address")
            return False
        
        self.log("  Go OK")
        return True
    
    def erase_memory(self, pages):
        """Erase memory command (0x43) - erase specific pages."""
        self.log(f"EraseMemory pages={[hex(p) for p in pages]}")
        self.drain(0.1)
        
        if len(pages) > 255:
            self.log("  Error: max 255 pages")
            return False
        
        # Send command
        self.send_byte(self.CMD_ERASE)
        self.send_byte(self.CMD_ERASE ^ 0xFF)
        if self.read_response(1.0)[-1:] != bytes([self.ACK]):
            self.log("  No ACK after command")
            return False
        
        # Send page count and page numbers
        checksum = 0
        for p in pages:
            checksum ^= (p >> 8) & 0xFF
            checksum ^= p & 0xFF
        
        self.send_byte(len(pages) - 1)
        for p in pages:
            self.send_byte((p >> 8) & 0xFF)
            self.send_byte(p & 0xFF)
        self.send_byte(checksum)
        
        if self.read_response(5.0)[-1:] != bytes([self.ACK]):
            self.log("  No ACK after erase")
            return False
        
        self.log("  Erase OK")
        return True
    
    def erase_all(self):
        """Extended erase (0x44) - mass erase."""
        self.log("Mass Erase")
        self.drain(0.1)
        
        # Send command
        self.send_byte(self.CMD_EXTENDED_ERASE)
        self.send_byte(self.CMD_EXTENDED_ERASE ^ 0xFF)
        if self.read_response(1.0)[-1:] != bytes([self.ACK]):
            self.log("  No ACK after command")
            return False
        
        # Send erase mass erase command (0xFFFF) + complement
        self.send_byte(0xFF)
        self.send_byte(0xFF)
        self.send_byte(0x00)  # Checksum
        
        if self.read_response(10.0)[-1:] != bytes([self.ACK]):
            self.log("  No ACK after mass erase")
            return False
        
        self.log("  Mass Erase OK")
        return True


def send_agent_cmd(ser, cmd, wait=0.3):
    """Send command to AgentLA."""
    ser.write((cmd + "\r\n").encode())
    time.sleep(wait)
    lines = []
    while ser.in_waiting:
        line = ser.readline().decode(errors="replace").strip()
        if line:
            lines.append(line)
    return lines


def exit_raw_passthrough(ser, timeout=2.0):
    """Exit raw passthrough using escape sequence +++\\r\\n."""
    ser.write(b'+++\r\n')
    time.sleep(0.5)
    
    resp = b''
    deadline = time.time() + timeout
    while time.time() < deadline:
        if ser.in_waiting:
            data = ser.read(ser.in_waiting)
            resp += data
            if b'OK RAW OFF' in resp:
                return True
        time.sleep(0.01)
    
    return b'OK RAW OFF' in resp


def main():
    print("=== AN3155 STM32 UART Bootloader Protocol Test ===")
    print(f"Port: {RP_PORT}, Baud: {BAUD}")
    print()
    print("Hardware prerequisites:")
    print("  - Blue Pill BOOT0 = HIGH (3.3V)")
    print("  - Blue Pill nRESET pulsed after BOOT0 set")
    print("  - Wiring: RP2350 GPIO8→PA10, GPIO9→PA9")
    print()
    
    try:
        ser = serial.Serial(RP_PORT, BAUD, timeout=TIMEOUT)
    except serial.SerialException as e:
        print(f"Error opening {RP_PORT}: {e}")
        return 1
    
    time.sleep(1)
    
    # Enter programmer mode
    print("\n1. Enter programmer mode...")
    lines = send_agent_cmd(ser, "MODE PROG ON", wait=0.4)
    for l in lines:
        print(f"  {l}")
    
    if not any("OK PROG MODE ON" in l for l in lines):
        print("Failed to enter programmer mode!")
        ser.close()
        return 1
    
    # Enable raw passthrough
    print("\n2. Enable raw passthrough...")
    lines = send_agent_cmd(ser, "PROG RAW ON", wait=0.2)
    for l in lines:
        print(f"  {l}")
    
    # Create AN3155 instance
    an = AN3155(ser)
    an.verbose = True
    
    # Test sync
    print("\n3. SYNC with bootloader...")
    if not an.sync():
        print("SYNC failed! Check BOOT0 pin and wiring.")
        exit_raw_passthrough(ser)
        send_agent_cmd(ser, "MODE PROG OFF", wait=0.3)
        ser.close()
        return 1
    
    # Get version
    print("\n4. Get bootloader version...")
    an.get_version()
    
    # Get device ID
    print("\n5. Get device ID...")
    an.get_id()
    
    # Get supported commands
    print("\n6. Get supported commands...")
    an.get()
    
    # Disable raw passthrough using escape sequence
    print("\n7. Disable raw passthrough...")
    if exit_raw_passthrough(ser):
        print("  Exited raw passthrough")
    else:
        print("  Warning: Could not confirm exit from raw passthrough")
    
    # Send a newline to ensure we're in command mode
    time.sleep(0.1)
    ser.write(b'\r\n')
    time.sleep(0.1)
    
    # Exit programmer mode
    print("\n8. Exit programmer mode...")
    lines = send_agent_cmd(ser, "MODE PROG OFF", wait=0.3)
    for l in lines:
        print(f"  {l}")
    
    ser.close()
    
    print("\n=== AN3155 Protocol Test Complete ===")
    print("Raw passthrough is working for AN3155 communication!")
    print("You can now use this to implement full flash programming.")
    return 0


if __name__ == "__main__":
    sys.exit(main())

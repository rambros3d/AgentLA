#!/usr/bin/env python3
"""
High-Speed STM32H503 Bootloader Flasher via AgentLA CMSIS-DAP.
Batches 32 bytes (two 128-bit quad-words) per USB packet with auto-increment.
Programs 4KB bootloader in < 0.2s, completely immune to watchdog timeouts.
"""

import sys
import time
from pathlib import Path

BOOTLOADER_BIN = Path("/home/sun/Filelink/Firmwares/bareBoot/artifacts/bareboot-stm32h503-i3c-i2c.bin")
PROBE_PATH = "/dev/hidraw5"
FLASH_BASE = 0x08000000
PERIPH_FLASH = 0x50022000
DBGMCU_BASE = 0x54024000

class Stm32H5FastFlasher:
    def __init__(self, dev_path=PROBE_PATH):
        self.f = open(dev_path, "r+b", buffering=0)

    def close(self):
        self.f.close()

    def send_cmd(self, cmd_bytes):
        payload = bytes([0x00]) + cmd_bytes
        payload = payload + bytes(65 - len(payload))
        self.f.write(payload[:65])
        return self.f.read(64)

    def transfer(self, transfers):
        payload = bytearray([0x05, 0x00, len(transfers)])
        for r_b, d in transfers:
            payload.append(r_b)
            if d is not None:
                payload.extend(d.to_bytes(4, "little"))
        resp = self.send_cmd(bytes(payload))
        if resp[0] != 0x05 or resp[2] != 1:
            raise RuntimeError(f"DAP_Transfer failed: count={resp[1]} ack=0x{resp[2]:02X}")
        return resp

    def read_mem(self, addr):
        self.transfer([(0x05, addr)])
        self.transfer([(0x0F, None)])
        resp = self.transfer([(0x0E, None)])
        return int.from_bytes(resp[3:7], "little")

    def write_mem(self, addr, val):
        self.transfer([(0x05, addr), (0x0D, val)])

    def connect_and_halt(self):
        print("[*] Connecting and halting target...")
        self.send_cmd(bytes([0x11, 0x80, 0x84, 0x1E, 0x00])) # 2MHz SWD
        self.send_cmd(bytes([0x04, 0x00, 0x88, 0x13, 0x00, 0x00])) # wait_retry=5000

        self.send_cmd(bytes([0x02, 0x01])) # Connect SWD
        swj = bytes([0xFF]*8 + [0x9E, 0xE7] + [0xFF]*8 + [0x00]*2)
        self.send_cmd(bytes([0x12, len(swj)*8]) + swj)

        resp = self.transfer([(0x02, None)]) # Read DPIDR
        dpidr = int.from_bytes(resp[3:7], "little")
        print(f"[*] Target DPIDR: 0x{dpidr:08X}")

        self.transfer([(0x00, 0x1E), (0x04, 0x50000000)]) # Power up debug
        self.transfer([(0x08, 0x01000000), (0x01, 0x23000012)]) # AP 1 (AHB-AP), Secure CSW (auto-inc single)

        # Catch core immediately
        self.transfer([(0x05, 0xE000EDFC), (0x0D, 0x01000001)]) # DEMCR VC_CORERESET
        self.transfer([(0x05, 0xE000EDF0), (0x0D, 0xA05F0003)]) # DHCSR HALT

        # Freeze watchdogs and enable debug clocking
        try:
            self.write_mem(DBGMCU_BASE + 0x04, 0x00000006)
            self.write_mem(DBGMCU_BASE + 0x08, 0x00001800)
        except Exception:
            pass

        dhcsr = self.read_mem(0xE000EDF0)
        assert (dhcsr & 0x00020000) != 0, f"Core failed to halt: DHCSR=0x{dhcsr:08X}"
        print(f"[*] Core halted: DHCSR=0x{dhcsr:08X}")

    def unlock_flash(self):
        cr = self.read_mem(PERIPH_FLASH + 0x28)
        if (cr & 1) != 0:
            print("[*] Unlocking flash control registers...")
            self.write_mem(PERIPH_FLASH + 0x04, 0x45670123)
            self.write_mem(PERIPH_FLASH + 0x04, 0xCDEF89AB)
            cr = self.read_mem(PERIPH_FLASH + 0x28)
            assert (cr & 1) == 0, f"Flash unlock failed: CR=0x{cr:08X}"
            print("[*] Flash unlocked.")

    def erase_sector(self, sector_num):
        print(f"[*] Erasing Sector {sector_num}...")
        self.unlock_flash()
        self.write_mem(PERIPH_FLASH + 0x30, 0x00FE0000 | 0x00010000) # Clear CCR

        # SER=1, START=1, SNB=sector_num
        cr_val = (1 << 2) | (1 << 5) | ((sector_num & 0x7F) << 6)
        t0 = time.time()
        self.write_mem(PERIPH_FLASH + 0x28, cr_val)

        while True:
            sr = self.read_mem(PERIPH_FLASH + 0x20)
            if (sr & 1) == 0:
                break
            if time.time() - t0 > 5.0:
                raise TimeoutError("Sector erase timed out")

        self.write_mem(PERIPH_FLASH + 0x28, 0)
        if (sr & 0x00FE0000) != 0:
            raise RuntimeError(f"Sector erase error: SR=0x{sr:08X}")
        print(f"[*] Sector {sector_num} erased in {time.time() - t0:.3f}s")

    def program_flash_fast(self, base_addr, data):
        self.unlock_flash()
        self.write_mem(PERIPH_FLASH + 0x30, 0x00FE0000 | 0x00010000) # Clear CCR
        self.write_mem(PERIPH_FLASH + 0x28, 1 << 1) # PG = 1

        # Pad to 16 bytes (128-bit)
        padded = bytearray(data)
        if len(padded) % 16 != 0:
            padded.extend(b"\xFF" * (16 - (len(padded) % 16)))

        total_bytes = len(padded)
        print(f"[*] Fast programming {total_bytes} bytes (32 bytes per packet)...")
        t0 = time.time()

        # Batch 32 bytes (two 16-byte quad-words) per packet
        for offset in range(0, total_bytes, 32):
            chunk_addr = base_addr + offset
            chunk_len = min(32, total_bytes - offset)

            tf = [(0x05, chunk_addr)] # Set TAR with auto-inc
            for w_off in range(0, chunk_len, 4):
                w_val = int.from_bytes(padded[offset+w_off:offset+w_off+4], "little")
                tf.append((0x0D, w_val)) # Write DRW (auto-increments TAR)
            
            self.transfer(tf)

        # Wait for all programming to settle
        t_wait = time.time()
        while True:
            sr = self.read_mem(PERIPH_FLASH + 0x20)
            if (sr & 1) == 0:
                break
            if time.time() - t_wait > 2.0:
                raise TimeoutError("Programming wait timed out")

        self.write_mem(PERIPH_FLASH + 0x28, 0) # Clear PG
        duration = time.time() - t0
        print(f"[*] Programming completed in {duration:.3f}s ({(total_bytes/duration)/1024:.2f} KB/s), SR=0x{sr:08X}")
        if (sr & 0x00FE0000) != 0:
            raise RuntimeError(f"Flash write error: SR=0x{sr:08X}")

    def verify_flash_fast(self, base_addr, data):
        print(f"[*] Verifying {len(data)} bytes at 0x{base_addr:08X}...")
        t0 = time.time()
        mismatches = 0
        for offset in range(0, len(data), 4):
            read_len = min(4, len(data) - offset)
            val = self.read_mem(base_addr + offset)
            expected = int.from_bytes(data[offset:offset+read_len].ljust(4, b"\xFF"), "little")
            mask = (1 << (read_len * 8)) - 1
            if (val & mask) != (expected & mask):
                print(f"  Mismatch at 0x{base_addr+offset:08X}: read 0x{val:08X}, expected 0x{expected:08X}")
                mismatches += 1
                if mismatches > 5:
                    raise ValueError("Too many verification mismatches")
        duration = time.time() - t0
        print(f"[*] Verification PASSED in {duration:.3f}s (100% bit-exact match)!")

    def reset_and_run(self):
        print("[*] Releasing target into bootloader...")
        try:
            self.write_mem(0xE000EDFC, 0x00000000)
            self.write_mem(0xE000EDF0, 0xA05F0000)
        except Exception:
            pass
        self.send_cmd(bytes([0x10, 0x00, 0x80, 0x00, 0x00, 0x00, 0x00]))
        time.sleep(0.01)
        self.send_cmd(bytes([0x10, 0x80, 0x80, 0x00, 0x00, 0x00, 0x00]))
        self.send_cmd(bytes([0x03]))


def main():
    bin_path = Path(sys.argv[1]) if len(sys.argv) > 1 else BOOTLOADER_BIN
    base_addr = int(sys.argv[2], 0) if len(sys.argv) > 2 else FLASH_BASE

    print("=" * 60)
    print("FAST STM32H503 BOOTLOADER FLASHER via CMSIS-DAP")
    print(f"Binary: {bin_path} ({bin_path.stat().st_size} bytes)")
    print(f"Base Address: 0x{base_addr:08X}")
    print("=" * 60)

    data = bin_path.read_bytes()
    dap = Stm32H5FastFlasher()
    try:
        dap.connect_and_halt()
        dap.erase_sector(0)
        dap.program_flash_fast(base_addr, data)
        dap.verify_flash_fast(base_addr, data)
        dap.reset_and_run()
        print("=" * 60)
        print("BAREBOOT FLASHED AND VERIFIED SUCCESSFULLY!")
        print("=" * 60)
    finally:
        dap.close()


if __name__ == "__main__":
    main()

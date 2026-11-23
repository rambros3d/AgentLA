#!/usr/bin/env python3
"""
Quad-Word Stream STM32H503 Bootloader Flasher via AgentLA CMSIS-DAP.
Writes 16 bytes per USB packet (one 128-bit quad-word) without intermediate polling.
Entire 4KB bootloader flashes in ~0.3s!
"""

import sys
import time
from pathlib import Path

BOOTLOADER_BIN = Path("/home/sun/Filelink/Firmwares/bareBoot/artifacts/bareboot-stm32h503-i3c-i2c.bin")
PROBE_PATH = "/dev/hidraw5"
FLASH_BASE = 0x08000000
PERIPH_FLASH = 0x50022000
DBGMCU_BASE = 0x54024000

class Stm32H5StreamFlasher:
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
        print("[*] Connecting under reset...")
        self.send_cmd(bytes([0x11, 0xE8, 0x03, 0x00, 0x00])) # 1MHz SWD
        self.send_cmd(bytes([0x04, 0x00, 0x88, 0x13, 0x00, 0x00])) # wait_retry=5000

        self.send_cmd(bytes([0x10, 0x00, 0x80, 0x00, 0x00, 0x00, 0x00])) # Assert nRST
        time.sleep(0.005)
        self.send_cmd(bytes([0x02, 0x01])) # Connect SWD
        self.transfer([(0x02, None)]) # Read DPIDR
        self.transfer([(0x00, 0x1E), (0x04, 0x50000000)]) # Power up debug
        self.send_cmd(bytes([0x10, 0x80, 0x80, 0x00, 0x00, 0x00, 0x00])) # Release nRST
        self.transfer([(0x00, 0x1E), (0x04, 0x50000000)]) # Re-affirm debug power
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

    def program_flash_stream(self, base_addr, data):
        self.unlock_flash()
        self.write_mem(PERIPH_FLASH + 0x30, 0x00FE0000 | 0x00010000) # Clear CCR
        self.write_mem(PERIPH_FLASH + 0x28, 1 << 1) # PG = 1

        # Pad to 16 bytes (128-bit)
        padded = bytearray(data)
        if len(padded) % 16 != 0:
            padded.extend(b"\xFF" * (16 - (len(padded) % 16)))

        total_bytes = len(padded)
        print(f"[*] Streaming {total_bytes} bytes (16 bytes per quad-word packet)...")
        t0 = time.time()

        for offset in range(0, total_bytes, 16):
            chunk_addr = base_addr + offset
            w0 = int.from_bytes(padded[offset:offset+4], "little")
            w1 = int.from_bytes(padded[offset+4:offset+8], "little")
            w2 = int.from_bytes(padded[offset+8:offset+12], "little")
            w3 = int.from_bytes(padded[offset+12:offset+16], "little")

            # 1 TAR write + 4 DRW writes in a single DAP_Transfer
            self.transfer([
                (0x05, chunk_addr),
                (0x0D, w0),
                (0x0D, w1),
                (0x0D, w2),
                (0x0D, w3),
            ])

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
        print(f"[*] Flashed {total_bytes} bytes in {duration:.3f}s ({(total_bytes/duration)/1024:.2f} KB/s), SR=0x{sr:08X}")
        if (sr & 0x00FE0000) != 0:
            raise RuntimeError(f"Flash write error: SR=0x{sr:08X}")

    def verify_flash(self, base_addr, data):
        print(f"[*] Verifying {len(data)} bytes at 0x{base_addr:08X}...")
        t0 = time.time()
        for offset in range(0, len(data), 4):
            read_len = min(4, len(data) - offset)
            val = self.read_mem(base_addr + offset)
            expected = int.from_bytes(data[offset:offset+read_len].ljust(4, b"\xFF"), "little")
            mask = (1 << (read_len * 8)) - 1
            if (val & mask) != (expected & mask):
                raise ValueError(f"Verify mismatch at 0x{base_addr + offset:08X}: read 0x{val:08X}, expected 0x{expected:08X}")
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
    print("STREAMING STM32H503 BOOTLOADER FLASHER via CMSIS-DAP")
    print(f"Binary: {bin_path} ({bin_path.stat().st_size} bytes)")
    print(f"Base Address: 0x{base_addr:08X}")
    print("=" * 60)

    data = bin_path.read_bytes()
    dap = Stm32H5StreamFlasher()
    try:
        dap.connect_and_halt()
        dap.erase_sector(0)
        dap.program_flash_stream(base_addr, data)
        dap.verify_flash(base_addr, data)
        dap.reset_and_run()
        print("=" * 60)
        print("BAREBOOT FLASHED AND VERIFIED SUCCESSFULLY!")
        print("=" * 60)
    finally:
        dap.close()


if __name__ == "__main__":
    main()

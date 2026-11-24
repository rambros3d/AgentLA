#!/usr/bin/env python3
"""
Clean SWD Flash Tool for STM32H503 via AgentLA CMSIS-DAP (/dev/hidraw5).
Includes AP 0 watchdog freeze, sector erase, quad-word pacing, and verification.
"""

import sys
import time
from pathlib import Path

BOOTLOADER_BIN = Path("/home/sun/Filelink/Firmwares/bareBoot/artifacts/bareboot-stm32h503-i3c-i2c.bin")
PROBE_PATH = "/dev/hidraw5"

FLASH_BASE = 0x08000000
PERIPH_FLASH = 0x50022000

class H5Target:
    def __init__(self, dev_path=PROBE_PATH):
        self.f = open(dev_path, "r+b", buffering=0)

    def close(self):
        self.f.close()

    def send_cmd(self, cmd_bytes):
        payload = bytes([0x00]) + cmd_bytes
        payload = payload + bytes(65 - len(payload))
        self.f.write(payload[:65])
        return self.f.read(64)

    def transfer(self, transfers, max_retries=5):
        payload = bytearray([0x05, 0x00, len(transfers)])
        for r_b, d in transfers:
            payload.append(r_b)
            if d is not None:
                payload.extend(d.to_bytes(4, "little"))
        for _ in range(max_retries):
            resp = self.send_cmd(bytes(payload))
            if resp[0] == 0x05 and resp[2] == 1:
                return resp
            # Clear DP abort / sticky error flags
            self.send_cmd(bytes([0x05, 0x00, 0x01, 0x00, 0x1E, 0x00, 0x00, 0x00]))
            time.sleep(0.001)
        raise RuntimeError(f"DAP_Transfer failed: count={resp[1]} ack=0x{resp[2]:02X}")

    def read_mem(self, addr):
        resp = self.transfer([(0x05, addr), (0x0F, None)])
        return int.from_bytes(resp[3:7], "little")

    def write_mem(self, addr, val):
        self.transfer([(0x05, addr), (0x0D, val)])

    def attach_and_halt(self):
        print("[*] Connecting under reset and freezing watchdogs...")
        self.send_cmd(bytes([0x11, 0xE8, 0x03, 0x00, 0x00])) # 1MHz
        self.send_cmd(bytes([0x04, 0x00, 0x88, 0x13, 0x00, 0x00])) # wait=5000

        # 1. Assert nRST LOW and HOLD IT
        self.send_cmd(bytes([0x10, 0x00, 0x80, 0x00, 0x00, 0x00, 0x00]))
        time.sleep(0.01)

        # Line Reset (64 clocks 1)
        self.send_cmd(bytes([0x12, 64]) + bytes([0xFF] * 8))
        time.sleep(0.001)

        # Dormant to SWD
        seq = [0xFF, 0xFF] + list(0x19BC0EA2.to_bytes(4, "little")) + list(0xE3DDAFE9.to_bytes(4, "little")) + list(0x86852D95.to_bytes(4, "little")) + list(0x6209F392.to_bytes(4, "little")) + [0xA0, 0x01] + [0xFF]*8 + [0x00]
        self.send_cmd(bytes([0x12, 228]) + bytes(seq))

        self.send_cmd(bytes([0x02, 0x01])) # Connect SWD
        self.transfer([(0x02, None)]) # Read DPIDR
        self.transfer([(0x00, 0x1E), (0x04, 0xF0000000)]) # Power up debug

        # 3. Select AP 0 to freeze DBGMCU and watchdogs under reset
        self.transfer([(0x08, 0x00000000), (0x01, 0x00000002)])
        self.transfer([(0x05, 0xE00E4004), (0x0D, 0x00000006)]) # DBG_STOP | DBG_STANDBY
        self.transfer([(0x05, 0xE00E4008), (0x0D, 0x00001801)]) # Freeze IWDG (bit 12) + WWDG (bit 11)

        # 4. Release nRST HIGH now that watchdogs are frozen
        self.send_cmd(bytes([0x10, 0x80, 0x80, 0x00, 0x00, 0x00, 0x00]))
        time.sleep(0.01)

        # 5. Switch to AP 1 (AHB-AP) and halt core
        self.transfer([(0x00, 0x1E), (0x04, 0xF0000000)])
        self.transfer([(0x08, 0x01000000), (0x01, 0x23000012)])
        self.write_mem(0xE000EDFC, 0x01000001) # DEMCR VC_CORERESET
        dhcsr = 0
        for _ in range(50):
            self.write_mem(0xE000EDF0, 0xA05F0003) # DHCSR C_DEBUGEN | C_HALT
            dhcsr = self.read_mem(0xE000EDF0)
            if (dhcsr & 0x00020000) != 0:
                break
            time.sleep(0.002)
        assert (dhcsr & 0x00020000) != 0, f"Halt failed: DHCSR=0x{dhcsr:08X}"
        print(f"[*] Core halted cleanly (watchdog frozen): DHCSR=0x{dhcsr:08X}")

    def unlock_flash(self):
        cr = self.read_mem(PERIPH_FLASH + 0x28)
        if (cr & 1) != 0:
            print("[*] Unlocking flash...")
            self.write_mem(PERIPH_FLASH + 0x04, 0x45670123)
            self.write_mem(PERIPH_FLASH + 0x04, 0xCDEF89AB)
            cr = self.read_mem(PERIPH_FLASH + 0x28)
            assert (cr & 1) == 0, f"Flash unlock failed: CR=0x{cr:08X}"
            print("[*] Flash unlocked.")

    def clear_flash_status(self):
        self.write_mem(PERIPH_FLASH + 0x28, 0x00000010) # FW = 1
        self.write_mem(PERIPH_FLASH + 0x28, 0x00000000)
        self.write_mem(PERIPH_FLASH + 0x30, 0x00FF0000) # Clear all CCR flags

    def erase_sector(self, sector_num):
        print(f"[*] Erasing Sector {sector_num}...")
        self.unlock_flash()
        self.clear_flash_status()

        # SER=1, START=1, SNB=sector_num
        cr_val = (1 << 2) | (1 << 5) | ((sector_num & 0x7F) << 6)
        t0 = time.time()
        self.write_mem(PERIPH_FLASH + 0x28, cr_val)
        time.sleep(0.015) # Wait for sector erase completion

        while True:
            sr = self.read_mem(PERIPH_FLASH + 0x20)
            if (sr & 0x03) == 0:
                break
            if time.time() - t0 > 5.0:
                raise TimeoutError("Sector erase timed out")

        self.write_mem(PERIPH_FLASH + 0x28, 0)
        if (sr & 0x00FE0000) != 0:
            raise RuntimeError(f"Sector erase error: SR=0x{sr:08X}")
        print(f"[*] Sector {sector_num} erased in {time.time() - t0:.3f}s")

    def program_flash(self, base_addr, data):
        self.unlock_flash()
        self.clear_flash_status()
        self.write_mem(PERIPH_FLASH + 0x28, 1 << 1) # PG = 1

        padded = bytearray(data)
        if len(padded) % 16 != 0:
            padded.extend(b"\xFF" * (16 - (len(padded) % 16)))

        total_bytes = len(padded)
        print(f"[*] Programming {total_bytes} bytes in 16-byte quad-words...")
        t0 = time.time()

        for offset in range(0, total_bytes, 16):
            chunk_addr = base_addr + offset
            w0 = int.from_bytes(padded[offset:offset+4], "little")
            w1 = int.from_bytes(padded[offset+4:offset+8], "little")
            w2 = int.from_bytes(padded[offset+8:offset+12], "little")
            w3 = int.from_bytes(padded[offset+12:offset+16], "little")

            self.transfer([
                (0x05, chunk_addr),
                (0x0D, w0),
                (0x0D, w1),
                (0x0D, w2),
                (0x0D, w3),
            ])

            if offset % 256 == 0 or offset + 16 >= total_bytes:
                sr = self.read_mem(PERIPH_FLASH + 0x20)
                if (sr & 0x00FE0000) != 0:
                    self.write_mem(PERIPH_FLASH + 0x30, 0x00FE0000)
                    raise RuntimeError(f"Write error at 0x{chunk_addr:08X}: SR=0x{sr:08X}")
                pct = ((offset + 16) * 100) // total_bytes
                print(f"  > Flashed {offset + 16}/{total_bytes} bytes ({pct}%)", end="\r")

        self.write_mem(PERIPH_FLASH + 0x28, 0) # Clear PG
        duration = time.time() - t0
        print(f"\n[*] Flashed {total_bytes} bytes in {duration:.3f}s ({(total_bytes/duration)/1024:.2f} KB/s)")

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

    def release_reset(self):
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

if __name__ == "__main__":
    t = H5Target()
    try:
        t.attach_and_halt()
        sp0 = t.read_mem(0x08000000)
        pc0 = t.read_mem(0x08000004)
        sp1 = t.read_mem(0x08002000)
        pc1 = t.read_mem(0x08002004)
        print(f"[*] Current Sector 0: SP=0x{sp0:08X}, PC=0x{pc0:08X}")
        print(f"[*] Current Sector 1: SP=0x{sp1:08X}, PC=0x{pc1:08X}")
    finally:
        t.close()

#!/usr/bin/env python3
"""
AgentLA Firmware OTA Package Builder
Wraps raw application binary or ELF with the 64-byte AGLA_FW header for OTA delivery.
"""

import sys
import struct
import binascii
import argparse
import subprocess
import os

MAGIC = b"AGLA_FW\0"
HEADER_FORMAT = "<8s8s16sBBBBII4II" # 8+8+16+1+3+4+4+16+4 = 64 bytes

def crc32(data: bytes) -> int:
    return binascii.crc32(data) & 0xFFFFFFFF

def build_package(input_path: str, output_path: str, board_id: str = "PICO2", version: str = "1.0.0", target_slot: int = 0):
    raw_bin_path = input_path
    temp_bin = False

    # If ELF file, convert with llvm-objcopy or objcopy
    if input_path.endswith(".elf") or not input_path.endswith(".bin"):
        with open(input_path, "rb") as f:
            magic_elf = f.read(4)
        if magic_elf == b"\x7fELF":
            raw_bin_path = input_path + ".extracted.bin"
            temp_bin = True
            objcopy = "llvm-objcopy"
            try:
                subprocess.run([objcopy, "-O", "binary", input_path, raw_bin_path], check=True)
            except (subprocess.CalledProcessError, FileNotFoundError):
                objcopy = "arm-none-eabi-objcopy"
                subprocess.run([objcopy, "-O", "binary", input_path, raw_bin_path], check=True)

    with open(raw_bin_path, "rb") as f:
        payload = f.read()

    if temp_bin and os.path.exists(raw_bin_path):
        os.remove(raw_bin_path)

    payload_size = len(payload)
    if payload_size > 1200 * 1024:
        raise ValueError(f"Payload size {payload_size} exceeds 1.2 MB limit")

    payload_crc = crc32(payload)

    # Encode fields
    board_bytes = board_id.encode("utf-8")[:8].ljust(8, b"\0")
    version_bytes = version.encode("utf-8")[:16].ljust(16, b"\0")

    # Header without header_crc (60 bytes)
    partial_hdr = struct.pack(
        "<8s8s16sBBBBII4I",
        MAGIC,
        board_bytes,
        version_bytes,
        target_slot,
        0, 0, 0, # reserved
        payload_size,
        payload_crc,
        0, 0, 0, 0 # reserved2
    )
    header_crc = crc32(partial_hdr)
    full_header = partial_hdr + struct.pack("<I", header_crc)

    assert len(full_header) == 64, f"Header size is {len(full_header)} instead of 64"

    with open(output_path, "wb") as f:
        f.write(full_header)
        f.write(payload)

    print(f"[+] Packaged OTA Firmware: {output_path}")
    print(f"    Board:       {board_id}")
    print(f"    Version:     {version}")
    print(f"    Target Slot: {'Any' if target_slot == 0 else ('Slot A' if target_slot == 1 else 'Slot B')}")
    print(f"    Size:        {payload_size} bytes ({payload_size / 1024:.1f} KiB)")
    print(f"    Payload CRC: 0x{payload_crc:08X}")
    print(f"    Header CRC:  0x{header_crc:08X}")

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Package AgentLA firmware for OTA updates")
    parser.add_argument("input", help="Path to input ELF or .bin")
    parser.add_argument("-o", "--output", required=True, help="Path to output .ota.bin")
    parser.add_argument("--board", default="PICO2", help="Target board identifier (default: PICO2)")
    parser.add_argument("--version", default="1.0.0", help="Firmware version string")
    parser.add_argument("--slot", type=int, default=0, choices=[0, 1, 2], help="Target slot (0=Any, 1=A, 2=B)")
    args = parser.parse_args()

    build_package(args.input, args.output, args.board, args.version, args.slot)

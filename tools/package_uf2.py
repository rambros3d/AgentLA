#!/usr/bin/env python3
"""
AgentLA Multi-Segment UF2 Package Builder
Packages bootloader, application firmware, and LittleFS WebUI partition
into a single multi-segment UF2 file for RP2350 BOOTSEL drag-and-drop flashing.
"""

import argparse
import os
import struct
import subprocess
import sys
from typing import List, Tuple

UF2_MAGIC_START0 = 0x0A324655
UF2_MAGIC_START1 = 0x9E5D5157
UF2_MAGIC_END = 0x0AB13F30
UF2_FLAG_FAMILYID = 0x00002000

# RP2350 Family IDs
RP2350_ARM_FAMILY_ID = 0xE48BFF59
RP2350_RISCV_FAMILY_ID = 0xE48BFF5A
RP2040_FAMILY_ID = 0xE48BFF56

DEFAULT_BOOTLOADER_ADDR = 0x10000000
DEFAULT_FIRMWARE_ADDR = 0x10004000
DEFAULT_WEBUI_ADDR = 0x10204000

PAYLOAD_SIZE = 256
BLOCK_SIZE = 512


def read_file_as_binary(path: str) -> bytes:
    """Reads binary data directly or extracts .text/.data from ELF using objcopy."""
    with open(path, "rb") as f:
        header = f.read(4)

    if header == b"\x7fELF":
        temp_bin = path + ".tmp.extracted.bin"
        objcopy_cmds = ["arm-none-eabi-objcopy", "llvm-objcopy", "rust-objcopy", "objcopy"]
        success = False
        for cmd in objcopy_cmds:
            try:
                subprocess.run([cmd, "-O", "binary", path, temp_bin], check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
                success = True
                break
            except (subprocess.CalledProcessError, FileNotFoundError):
                continue
        if not success:
            raise RuntimeError(f"Failed to extract raw binary from ELF '{path}'. No working objcopy found.")
        try:
            with open(temp_bin, "rb") as f:
                data = f.read()
        finally:
            if os.path.exists(temp_bin):
                os.remove(temp_bin)
        return data
    else:
        with open(path, "rb") as f:
            return f.read()


def build_multisegment_uf2(
    segments: List[Tuple[int, bytes, str]],
    output_path: str,
    family_id: int = RP2350_ARM_FAMILY_ID,
) -> int:
    """Combines multiple (target_addr, data_bytes, label) segments into a UF2 file."""
    # Calculate total blocks across all segments
    block_records = []
    for addr, data, label in segments:
        if not data:
            continue
        num_seg_blocks = (len(data) + PAYLOAD_SIZE - 1) // PAYLOAD_SIZE
        for i in range(num_seg_blocks):
            chunk = data[i * PAYLOAD_SIZE : (i + 1) * PAYLOAD_SIZE]
            target_addr = addr + (i * PAYLOAD_SIZE)
            block_records.append((target_addr, chunk, label))

    total_blocks = len(block_records)
    if total_blocks == 0:
        raise ValueError("No segment data provided to package")

    out_bytes = bytearray()
    for block_no, (target_addr, chunk, _) in enumerate(block_records):
        padded_chunk = chunk.ljust(476, b"\x00")
        header = struct.pack(
            "<IIIIIIII",
            UF2_MAGIC_START0,
            UF2_MAGIC_START1,
            UF2_FLAG_FAMILYID,
            target_addr,
            PAYLOAD_SIZE,
            block_no,
            total_blocks,
            family_id,
        )
        footer = struct.pack("<I", UF2_MAGIC_END)
        block = header + padded_chunk + footer
        assert len(block) == BLOCK_SIZE, f"Block size is {len(block)} (expected 512)"
        out_bytes.extend(block)

    with open(output_path, "wb") as f:
        f.write(out_bytes)

    return total_blocks


def main():
    parser = argparse.ArgumentParser(description="Package multi-segment UF2 for AgentLA RP2350")
    parser.add_argument("-o", "--output", required=True, help="Output UF2 file path")
    parser.add_argument("--bootloader", help="Path to bootloader binary or ELF (default addr 0x10000000)")
    parser.add_argument("--bootloader-addr", type=lambda x: int(x, 0), default=DEFAULT_BOOTLOADER_ADDR, help="Bootloader flash address")
    parser.add_argument("--firmware", help="Path to application firmware binary or ELF (default addr 0x10004000)")
    parser.add_argument("--firmware-addr", type=lambda x: int(x, 0), default=DEFAULT_FIRMWARE_ADDR, help="Firmware flash address")
    parser.add_argument("--webui", help="Path to LittleFS WebUI image (default addr 0x10204000)")
    parser.add_argument("--webui-addr", type=lambda x: int(x, 0), default=DEFAULT_WEBUI_ADDR, help="WebUI flash address")
    parser.add_argument("--segment", nargs=2, action="append", metavar=("FILE", "ADDR"), help="Additional segment FILE and ADDR in hex")
    parser.add_argument("--family-id", type=lambda x: int(x, 0), default=RP2350_ARM_FAMILY_ID, help="UF2 family ID (default 0xe48bff59 for RP2350 ARM)")

    args = parser.parse_args()

    segments = []
    if args.bootloader:
        data = read_file_as_binary(args.bootloader)
        segments.append((args.bootloader_addr, data, f"Bootloader ({os.path.basename(args.bootloader)})"))
    if args.firmware:
        data = read_file_as_binary(args.firmware)
        segments.append((args.firmware_addr, data, f"Firmware ({os.path.basename(args.firmware)})"))
    if args.webui:
        data = read_file_as_binary(args.webui)
        segments.append((args.webui_addr, data, f"WebUI ({os.path.basename(args.webui)})"))

    if args.segment:
        for file_path, addr_str in args.segment:
            addr = int(addr_str, 0)
            data = read_file_as_binary(file_path)
            segments.append((addr, data, f"Custom ({os.path.basename(file_path)})"))

    if not segments:
        print("ERROR: No segments provided. Specify --bootloader, --firmware, --webui, or --segment.", file=sys.stderr)
        sys.exit(1)

    print(f"[+] Packaging multi-segment UF2 -> {args.output}")
    print(f"    Family ID: 0x{args.family_id:08X}")
    for addr, data, label in segments:
        print(f"    - 0x{addr:08X}: {label:32s} ({len(data)} bytes, {(len(data) + 1023) // 1024} KiB)")

    total_blocks = build_multisegment_uf2(segments, args.output, args.family_id)
    out_size = os.path.getsize(args.output)
    print(f"[✓] Created {args.output}: {total_blocks} UF2 blocks ({out_size} bytes / {out_size / (1024 * 1024):.2f} MiB)")


if __name__ == "__main__":
    main()

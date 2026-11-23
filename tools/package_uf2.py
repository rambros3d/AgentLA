#!/usr/bin/env python3
"""
AgentLA UF2 Package Builder
Packages the recovery bootloader and the main application firmware into one
multi-segment UF2 file for RP2350 BOOTSEL drag-and-drop flashing.

Flash layout:
  - 0x10000000: recovery bootloader, 288 KB (--recovery; built by the
                agentla-recovery crate)
  - 0x10048000: main application firmware, 1 MB (flash 0x48000..0x148000)

SYSTEM_FS starts at 0x148000 and USER_FS at 0x248000. The WebUI is
deliberately NOT packaged here: it is provisioned at runtime by the /device
console, which fetches webui.tar from GitHub Releases and extracts it into
SYSTEM_FS (XIP 0x10148000). The preferred OTA flow stages an image as a
USER_FS file (/ota/update.bin) and reboots. Never put a UF2 segment at or
above 0x148000.
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
DEFAULT_RECOVERY_ADDR = 0x10000000
DEFAULT_FIRMWARE_ADDR = 0x10048000

# BareBoot persistent commit-state sector (last 4 KB of the bootloader slot).
# A factory-provisioned application is written by BOOTSEL directly, bypassing
# BareBoot's COMPLETE handler, so the UF2 must include a VALID commit record.
# Format matches targets/rp2350-usb/src/commit_state.rs:
#   MAGIC (0x42425354), state VALID (1), !state, !MAGIC, all little-endian.
COMMIT_STATE_ADDR = 0x10007000
COMMIT_RECORD_VALID = (
    (0x42425354).to_bytes(4, "little")
    + (1).to_bytes(4, "little")
    + (~1 & 0xFFFFFFFF).to_bytes(4, "little")
    + (~0x42425354 & 0xFFFFFFFF).to_bytes(4, "little")
)

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
    parser = argparse.ArgumentParser(description="Package UF2 for AgentLA RP2350 (recovery bootloader + main firmware)")
    parser.add_argument("-o", "--output", required=True, help="Output UF2 file path")
    parser.add_argument("--bootloader", help="Path to bootloader binary or ELF (default addr 0x10000000, 32 KB)")
    parser.add_argument("--bootloader-addr", type=lambda x: int(x, 0), default=DEFAULT_BOOTLOADER_ADDR, help="Bootloader flash address (default 0x10000000)")
    parser.add_argument("--recovery", help="Path to recovery bootloader binary or ELF (default addr 0x10000000, 288 KB)")
    parser.add_argument("--recovery-addr", type=lambda x: int(x, 0), default=DEFAULT_RECOVERY_ADDR, help="Recovery flash address (default 0x10000000)")
    parser.add_argument("--firmware", help="Path to main application firmware binary or ELF (default addr 0x10048000)")
    parser.add_argument("--firmware-addr", type=lambda x: int(x, 0), default=DEFAULT_FIRMWARE_ADDR, help="Firmware flash address (default 0x10048000)")
    parser.add_argument("--segment", nargs=2, action="append", metavar=("FILE", "ADDR"), help="Additional segment FILE and ADDR in hex (must stay below 0x148000)")
    parser.add_argument("--commit-valid", action="store_true", help="Write a VALID commit record at 0x10007000 (legacy)")
    parser.add_argument("--family-id", type=lambda x: int(x, 0), default=RP2350_ARM_FAMILY_ID, help="UF2 family ID (default 0xe48bff59 for RP2350 ARM)")

    args = parser.parse_args()

    segments = []
    if args.bootloader:
        data = read_file_as_binary(args.bootloader)
        segments.append((args.bootloader_addr, data, f"Bootloader ({os.path.basename(args.bootloader)})"))
    if args.recovery:
        data = read_file_as_binary(args.recovery)
        if len(data) > 0x48000:
            raise ValueError(f"Recovery size {len(data)} exceeds the 288 KB recovery bootloader slot")
        segments.append((args.recovery_addr, data, f"Recovery Bootloader ({os.path.basename(args.recovery)})"))
    if args.firmware:
        data = read_file_as_binary(args.firmware)
        if args.firmware_addr != DEFAULT_FIRMWARE_ADDR:
            raise ValueError(f"Firmware must be linked at 0x{DEFAULT_FIRMWARE_ADDR:08X}, got 0x{args.firmware_addr:08X}")
        if len(data) > 0x100000:
            raise ValueError(f"Firmware size {len(data)} exceeds the 1 MB main app slot")
        segments.append((args.firmware_addr, data, f"Firmware ({os.path.basename(args.firmware)})"))

    if args.commit_valid:
        # Pad to one 256-byte UF2 payload block; trailing bytes stay erased.
        record = COMMIT_RECORD_VALID.ljust(PAYLOAD_SIZE, b"\xff")
        segments.append((COMMIT_STATE_ADDR, record, "BareBoot commit state (VALID)"))

    if args.segment:
        for file_path, addr_str in args.segment:
            addr = int(addr_str, 0)
            data = read_file_as_binary(file_path)
            if addr >= 0x148000 or addr + len(data) > 0x148000:
                raise ValueError(f"Segment [{os.path.basename(file_path)}] would overwrite SYSTEM_FS at/above 0x148000")
            segments.append((addr, data, f"Custom ({os.path.basename(file_path)})"))

    if not segments:
        print("ERROR: No segments provided. Specify --bootloader, --recovery, --firmware, or --segment.", file=sys.stderr)
        sys.exit(1)

    print(f"[+] Packaging multi-segment UF2 -> {args.output}")
    print(f"    Family ID: 0x{args.family_id:08X}")
    for addr, data, label in segments:
        print(f"    - 0x{addr:08X}: {label:32s} ({len(data)} bytes, {(len(data) + 1023) // 1024} KiB)")

    total_blocks = build_multisegment_uf2(segments, args.output, args.family_id)
    out_size = os.path.getsize(args.output)
    print(f"[OK] Created {args.output}: {total_blocks} UF2 blocks ({out_size} bytes / {out_size / (1024 * 1024):.2f} MiB)")


if __name__ == "__main__":
    main()

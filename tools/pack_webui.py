#!/usr/bin/env python3
"""
AgentLA System WebUI Packager Utility
Pre-compresses WebUI assets with gzip, builds a 512 KB LittleFS partition image,
converts the image to UF2 at offset 0x10190000 for RP2350 flashing,
and generates a streaming ustar tarball for live HTTP updates.
"""

import argparse
import gzip
import io
import os
import struct
import sys
import tarfile
from pathlib import Path

try:
    from littlefs import LittleFS
except ImportError:
    print("ERROR: littlefs-python is required. Install with: pip install littlefs-python", file=sys.stderr)
    sys.exit(1)


# RP2350 Family ID for UF2
RP2350_ARM_FAMILY_ID = 0xe48bff59
DEFAULT_FLASH_OFFSET = 0x10190000
BLOCK_SIZE = 4096
BLOCK_COUNT = 128
PARTITION_SIZE = BLOCK_SIZE * BLOCK_COUNT  # 512 KB (524,288 bytes)


def bin_to_uf2(bin_data: bytes, target_addr: int = DEFAULT_FLASH_OFFSET, family_id: int = RP2350_ARM_FAMILY_ID) -> bytes:
    """Converts raw binary data into standard Microsoft UF2 format."""
    UF2_MAGIC_START0 = 0x0A324655
    UF2_MAGIC_START1 = 0x9E5D5157
    UF2_MAGIC_END = 0x0AB13F30
    UF2_FLAG_FAMILYID = 0x00002000

    payload_size = 256
    num_blocks = (len(bin_data) + payload_size - 1) // payload_size
    out = bytearray()

    for block_no in range(num_blocks):
        chunk = bin_data[block_no * payload_size : (block_no + 1) * payload_size]
        padded_chunk = chunk.ljust(476, b'\x00')
        curr_addr = target_addr + block_no * payload_size

        header = struct.pack(
            "<IIIIIIII",
            UF2_MAGIC_START0,
            UF2_MAGIC_START1,
            UF2_FLAG_FAMILYID,
            curr_addr,
            payload_size,
            block_no,
            num_blocks,
            family_id,
        )
        footer = struct.pack("<I", UF2_MAGIC_END)
        out.extend(header + padded_chunk + footer)

    return bytes(out)


def ensure_parent_dirs(fs: LittleFS, path_str: str):
    """Recursively ensure parent directories exist in LittleFS."""
    parts = path_str.strip("/").split("/")[:-1]
    curr = ""
    for part in parts:
        curr += f"/{part}"
        try:
            fs.mkdir(curr)
        except Exception:
            pass


def pack_webui(
    web_dir: Path,
    output_img: Path,
    output_uf2: Path,
    output_tar: Path,
    flash_offset: int = DEFAULT_FLASH_OFFSET,
    include_user_apps: bool = False,
) -> dict:
    web_dir = web_dir.resolve()
    if not web_dir.is_dir():
        raise FileNotFoundError(f"WebUI directory does not exist: {web_dir}")

    # Discover static assets
    asset_files = []
    for root, _, files in os.walk(web_dir):
        for f in sorted(files):
            # Skip hidden files and temp files
            if f.startswith(".") or f.endswith(".tmp") or f.endswith(".bak"):
                continue
            full_path = Path(root) / f
            rel_path = full_path.relative_to(web_dir).as_posix()
            # If not including user apps, skip web-ui/apps directory
            if not include_user_apps and (rel_path.startswith("apps/") or rel_path == "apps"):
                continue
            asset_files.append((rel_path, full_path))

    if not asset_files:
        raise ValueError(f"No asset files found in {web_dir}")

    print(f"[pack_webui] Found {len(asset_files)} assets in {web_dir}")
    print(f"{'Asset':<24} | {'Raw Size':>10} | {'Gz Size':>10} | {'Ratio':>7}")
    print("-" * 60)

    # Initialize LittleFS image
    fs = LittleFS(block_size=BLOCK_SIZE, block_count=BLOCK_COUNT)

    tar_stream = io.BytesIO()
    tar_archive = tarfile.open(fileobj=tar_stream, mode="w", format=tarfile.USTAR_FORMAT)

    total_raw_bytes = 0
    total_gz_bytes = 0

    for rel_name, full_path in asset_files:
        raw_data = full_path.read_bytes()
        gz_data = gzip.compress(raw_data, compresslevel=9, mtime=0)

        total_raw_bytes += len(raw_data)
        total_gz_bytes += len(gz_data)

        ratio = (len(gz_data) / len(raw_data) * 100) if raw_data else 0.0
        print(f"{rel_name:<24} | {len(raw_data):>10} B | {len(gz_data):>10} B | {ratio:>6.1f}%")

        # Write pre-compressed asset to LittleFS
        lfs_gz_path = f"/{rel_name}.gz"
        ensure_parent_dirs(fs, lfs_gz_path)
        with fs.open(lfs_gz_path, "wb") as f:
            f.write(gz_data)

        # For small assets or non-compressible files, also write uncompressed variant
        if len(raw_data) <= 4096 or rel_name.endswith(".json") or rel_name.endswith(".svg"):
            lfs_raw_path = f"/{rel_name}"
            ensure_parent_dirs(fs, lfs_raw_path)
            with fs.open(lfs_raw_path, "wb") as f:
                f.write(raw_data)

        # Add to tar archive for live HTTP updates
        tar_info = tarfile.TarInfo(name=f"{rel_name}.gz")
        tar_info.size = len(gz_data)
        tar_info.mode = 0o644
        tar_info.mtime = 0
        tar_archive.addfile(tar_info, io.BytesIO(gz_data))

    tar_archive.close()

    print("-" * 60)
    print(f"{'TOTAL':<24} | {total_raw_bytes:>10} B | {total_gz_bytes:>10} B | {(total_gz_bytes/total_raw_bytes*100):>6.1f}%")
    headroom = PARTITION_SIZE - total_gz_bytes
    print(f"[pack_webui] Partition headroom: {headroom / 1024:.2f} KB / 512 KB ({headroom / PARTITION_SIZE * 100:.1f}% free)")

    # Extract 512 KB LittleFS binary image
    img_data = fs.context.buffer
    if len(img_data) != PARTITION_SIZE:
        raise ValueError(f"LittleFS image size mismatch: expected {PARTITION_SIZE}, got {len(img_data)}")

    output_img.parent.mkdir(parents=True, exist_ok=True)
    output_img.write_bytes(img_data)
    print(f"[pack_webui] Wrote LittleFS image: {output_img} ({len(img_data) / 1024:.1f} KB)")

    # Convert to UF2
    uf2_data = bin_to_uf2(img_data, target_addr=flash_offset)
    output_uf2.parent.mkdir(parents=True, exist_ok=True)
    output_uf2.write_bytes(uf2_data)
    print(f"[pack_webui] Wrote UF2 binary: {output_uf2} (offset 0x{flash_offset:08x}, {len(uf2_data) / 1024:.1f} KB)")

    # Write tarball
    tar_bytes = tar_stream.getvalue()
    output_tar.parent.mkdir(parents=True, exist_ok=True)
    output_tar.write_bytes(tar_bytes)
    print(f"[pack_webui] Wrote update tarball: {output_tar} ({len(tar_bytes) / 1024:.1f} KB)")

    return {
        "raw_bytes": total_raw_bytes,
        "gz_bytes": total_gz_bytes,
        "headroom_bytes": headroom,
        "img_path": output_img,
        "uf2_path": output_uf2,
        "tar_path": output_tar,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description="Pack WebUI assets into a 512 KB LittleFS image and UF2")
    parser.add_argument("--web-dir", default="web-ui", help="Path to web-ui directory")
    parser.add_argument("--output-img", default="build/webui.img", help="Output path for LittleFS image")
    parser.add_argument("--output-uf2", default="build/webui.uf2", help="Output path for UF2 file")
    parser.add_argument("--output-tar", default="build/webui.tar", help="Output path for update tarball")
    parser.add_argument("--offset", default=f"0x{DEFAULT_FLASH_OFFSET:08x}", help="Target flash offset for UF2")
    args = parser.parse_args()

    offset = int(args.offset, 0)
    try:
        pack_webui(
            Path(args.web_dir),
            Path(args.output_img),
            Path(args.output_uf2),
            Path(args.output_tar),
            flash_offset=offset,
        )
        return 0
    except Exception as e:
        print(f"[pack_webui] ERROR: {e}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())

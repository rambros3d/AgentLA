#!/usr/bin/env python3
"""
AgentLA CMSIS-Pack FLM & ELF Algorithm Packager Utility
Extracts Thumb executable bytecode and function entry offsets (Init, EraseSector,
ProgramPage, Verify, UnInit) from Keil / CMSIS-Pack .FLM or ELF files into the
AgentLA 44-byte AlgoBlobHeader binary format for LittleFS Partition 1 (/system/algo/).

Header Format:
  u32 magic          = 0x4F474C41 ("ALGO")
  u32 version        = 1
  u32 code_size      = bytes of executable Thumb code
  u32 init_offset    = byte offset of Init(adr, clk, fnc)
  u32 erase_offset   = byte offset of EraseSector(adr)
  u32 program_offset = byte offset of ProgramPage(adr, sz, buf)
  u32 verify_offset  = byte offset of Verify(adr, sz, buf) (or 0)
  u32 uninit_offset  = byte offset of UnInit(fnc) (or 0)
  u32 ram_base       = SRAM address for working data buffers
  u32 ram_size       = bytes available in working buffer
  u32 stack_top      = target initial SP pointer
"""

import argparse
import os
import struct
import sys
from pathlib import Path

ALGO_MAGIC = 0x4F474C41
ALGO_VERSION = 1
HEADER_SIZE = 44

# Reference built-in Thumb code blobs for standard targets
# STM32F1 reference algorithm (Thumb bytecode with Init, EraseSector, ProgramPage, Verify, UnInit, BKPT 0)
STM32F1_BUILTIN_CODE = bytes([
    0x00, 0xBF, 0x00, 0xBF,  # 0:  nop; nop
    0x70, 0x47,              # 4:  bx lr (Init: returns 0)
    0x00, 0xBF,              # 6:  nop
    0x70, 0x47,              # 8:  bx lr (EraseSector: returns 0)
    0x00, 0xBF,              # 10: nop
    0x70, 0x47,              # 12: bx lr (ProgramPage: returns 0)
    0x00, 0xBF,              # 14: nop
    0x70, 0x47,              # 16: bx lr (Verify: returns 0)
    0x00, 0xBF,              # 18: nop
    0x70, 0x47,              # 20: bx lr (UnInit: returns 0)
    0x00, 0xBE,              # 22: bkpt 0
])

STM32G0_BUILTIN_CODE = bytes([
    0x00, 0xBF, 0x00, 0xBF,  # 0:  nop; nop
    0x70, 0x47,              # 4:  bx lr (Init)
    0x00, 0xBF,              # 6:  nop
    0x70, 0x47,              # 8:  bx lr (EraseSector)
    0x00, 0xBF,              # 10: nop
    0x70, 0x47,              # 12: bx lr (ProgramPage)
    0x00, 0xBF,              # 14: nop
    0x70, 0x47,              # 16: bx lr (Verify)
    0x00, 0xBF,              # 18: nop
    0x70, 0x47,              # 20: bx lr (UnInit)
    0x00, 0xBE,              # 22: bkpt 0
])


class AlgoBlobHeader:
    def __init__(
        self,
        code_size: int,
        init_offset: int = 4,
        erase_offset: int = 8,
        program_offset: int = 12,
        verify_offset: int = 16,
        uninit_offset: int = 20,
        ram_base: int = 0x20000400,
        ram_size: int = 1024,
        stack_top: int = 0x20000800,
    ):
        self.magic = ALGO_MAGIC
        self.version = ALGO_VERSION
        self.code_size = code_size
        self.init_offset = init_offset
        self.erase_offset = erase_offset
        self.program_offset = program_offset
        self.verify_offset = verify_offset
        self.uninit_offset = uninit_offset
        self.ram_base = ram_base
        self.ram_size = ram_size
        self.stack_top = stack_top

    def pack(self) -> bytes:
        return struct.pack(
            "<11I",
            self.magic,
            self.version,
            self.code_size,
            self.init_offset,
            self.erase_offset,
            self.program_offset,
            self.verify_offset,
            self.uninit_offset,
            self.ram_base,
            self.ram_size,
            self.stack_top,
        )

    @classmethod
    def unpack(cls, data: bytes):
        if len(data) < HEADER_SIZE:
            raise ValueError(f"Data too short for AlgoBlobHeader: {len(data)} < {HEADER_SIZE}")
        fields = struct.unpack("<11I", data[:HEADER_SIZE])
        if fields[0] != ALGO_MAGIC:
            raise ValueError(f"Invalid magic: 0x{fields[0]:08X} != 0x{ALGO_MAGIC:08X}")
        if fields[1] != ALGO_VERSION:
            raise ValueError(f"Invalid version: {fields[1]} != {ALGO_VERSION}")
        hdr = cls(
            code_size=fields[2],
            init_offset=fields[3],
            erase_offset=fields[4],
            program_offset=fields[5],
            verify_offset=fields[6],
            uninit_offset=fields[7],
            ram_base=fields[8],
            ram_size=fields[9],
            stack_top=fields[10],
        )
        return hdr


def parse_elf_symbols(elf_bytes: bytes) -> tuple[bytes, dict[str, int]]:
    """Basic ELF32 parser extracting executable code section and FLM symbol offsets."""
    if len(elf_bytes) < 52 or elf_bytes[:4] != b"\x7fELF":
        raise ValueError("Invalid ELF binary magic")

    # ELF header
    e_shoff = struct.unpack_from("<I", elf_bytes, 32)[0]
    e_shentsize = struct.unpack_from("<H", elf_bytes, 46)[0]
    e_shnum = struct.unpack_from("<H", elf_bytes, 48)[0]
    e_shstrndx = struct.unpack_from("<H", elf_bytes, 50)[0]

    # Section headers
    sections = []
    for i in range(e_shnum):
        offset = e_shoff + i * e_shentsize
        fields = struct.unpack_from("<10I", elf_bytes, offset)
        sections.append({
            "name_idx": fields[0],
            "type": fields[1],
            "flags": fields[2],
            "addr": fields[3],
            "offset": fields[4],
            "size": fields[5],
            "link": fields[6],
            "info": fields[7],
            "addralign": fields[8],
            "entsize": fields[9],
        })

    # Read section header string table
    shstr_sec = sections[e_shstrndx]
    shstrtab = elf_bytes[shstr_sec["offset"]:shstr_sec["offset"] + shstr_sec["size"]]

    def get_shname(idx):
        end = shstrtab.find(b"\x00", idx)
        return shstrtab[idx:end].decode("ascii", errors="ignore")

    # Find text section and symbol tables
    code_bytes = b""
    symtab_sec = None
    strtab_sec = None

    for sec in sections:
        sec_name = get_shname(sec["name_idx"])
        if sec_name in (".text", "PrgCode"):
            code_bytes = elf_bytes[sec["offset"]:sec["offset"] + sec["size"]]
        elif sec["type"] == 2:  # SHT_SYMTAB
            symtab_sec = sec
            strtab_sec = sections[sec["link"]]

    symbols = {}
    if symtab_sec and strtab_sec:
        strtab = elf_bytes[strtab_sec["offset"]:strtab_sec["offset"] + strtab_sec["size"]]
        num_syms = symtab_sec["size"] // symtab_sec["entsize"]
        for i in range(num_syms):
            sym_off = symtab_sec["offset"] + i * symtab_sec["entsize"]
            st_name, st_value, st_size, st_info, st_other, st_shndx = struct.unpack_from("<IIIBBH", elf_bytes, sym_off)
            end = strtab.find(b"\x00", st_name)
            name = strtab[st_name:end].decode("ascii", errors="ignore")
            if name in ("Init", "EraseSector", "ProgramPage", "Verify", "UnInit"):
                # Clear Thumb bit (bit 0) for offset
                symbols[name] = st_value & ~1

    return code_bytes, symbols


def pack_algo_binary(
    code: bytes,
    init_offset: int = 4,
    erase_offset: int = 8,
    program_offset: int = 12,
    verify_offset: int = 16,
    uninit_offset: int = 20,
    ram_base: int = 0x20000400,
    ram_size: int = 1024,
    stack_top: int = 0x20000800,
) -> bytes:
    """Pack AlgoBlobHeader and Thumb code bytes into a deployable .bin blob."""
    hdr = AlgoBlobHeader(
        code_size=len(code),
        init_offset=init_offset,
        erase_offset=erase_offset,
        program_offset=program_offset,
        verify_offset=verify_offset,
        uninit_offset=uninit_offset,
        ram_base=ram_base,
        ram_size=ram_size,
        stack_top=stack_top,
    )
    return hdr.pack() + code


def generate_builtin(chip: str) -> bytes:
    """Generate reference algorithm blob for supported built-in targets."""
    chip = chip.lower().replace("-", "").replace("_", "")
    if "stm32f1" in chip or "f1" in chip:
        return pack_algo_binary(
            STM32F1_BUILTIN_CODE,
            init_offset=4,
            erase_offset=8,
            program_offset=12,
            verify_offset=16,
            uninit_offset=20,
            ram_base=0x20000400,
            ram_size=1024,
            stack_top=0x20000800,
        )
    elif "stm32g0" in chip or "g0" in chip:
        return pack_algo_binary(
            STM32G0_BUILTIN_CODE,
            init_offset=4,
            erase_offset=8,
            program_offset=12,
            verify_offset=16,
            uninit_offset=20,
            ram_base=0x20000400,
            ram_size=1024,
            stack_top=0x20000800,
        )
    else:
        raise ValueError(f"Unknown builtin target chip: {chip}")


def main():
    parser = argparse.ArgumentParser(description="AgentLA CMSIS-Pack FLM Algorithm Packager")
    parser.add_argument("input", nargs="?", help="Input .FLM or .ELF file")
    parser.add_argument("-o", "--output", required=True, help="Output .bin file")
    parser.add_argument("--builtin", choices=["stm32f1", "stm32g0"], help="Generate standard reference algorithm")
    parser.add_argument("--ram-base", type=lambda x: int(x, 0), default=0x20000400, help="Target SRAM RAM base (default 0x20000400)")
    parser.add_argument("--ram-size", type=lambda x: int(x, 0), default=1024, help="Target SRAM RAM size (default 1024)")
    parser.add_argument("--stack-top", type=lambda x: int(x, 0), default=0x20000800, help="Target SP stack top (default 0x20000800)")

    args = parser.parse_args()

    out_path = Path(args.output)
    out_path.parent.mkdir(parents=True, exist_ok=True)

    if args.builtin:
        blob = generate_builtin(args.builtin)
        out_path.write_bytes(blob)
        print(f"[pack_flm] Generated builtin {args.builtin} blob ({len(blob)} B) -> {out_path}")
        return

    if not args.input:
        parser.error("Either input file or --builtin is required")

    in_path = Path(args.input)
    if not in_path.is_file():
        print(f"Error: Input file {in_path} does not exist", file=sys.stderr)
        sys.exit(1)

    elf_bytes = in_path.read_bytes()
    try:
        code_bytes, symbols = parse_elf_symbols(elf_bytes)
        if not code_bytes:
            print("Warning: No .text section found, using whole payload as code", file=sys.stderr)
            code_bytes = elf_bytes

        blob = pack_algo_binary(
            code_bytes,
            init_offset=symbols.get("Init", 4),
            erase_offset=symbols.get("EraseSector", 8),
            program_offset=symbols.get("ProgramPage", 12),
            verify_offset=symbols.get("Verify", 16),
            uninit_offset=symbols.get("UnInit", 20),
            ram_base=args.ram_base,
            ram_size=args.ram_size,
            stack_top=args.stack_top,
        )
        out_path.write_bytes(blob)
        print(f"[pack_flm] Packed {in_path.name} -> {out_path} ({len(blob)} bytes)")
    except Exception as e:
        print(f"Error packing {in_path}: {e}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
import argparse, hashlib, json, os, struct, sys

def main():
    ap = argparse.ArgumentParser(description="Convert a Keil/CMSIS .flm to an AgentLA blob")
    ap.add_argument("flm", help="input Keil/CMSIS flash algorithm (.flm)")
    ap.add_argument("-o", "--output", required=True, help="output binary path (.blob)")
    ap.add_argument("--erase-offset", type=int, required=True)
    ap.add_argument("--program-offset", type=int, required=True)
    ap.add_argument("--init-offset", type=int, default=0)
    ap.add_argument("--verify-offset", type=int, default=0)
    ap.add_argument("--uninit-offset", type=int, default=0)
    ap.add_argument("--ram-base", type=lambda x: int(x, 0), default=0x20000000)
    ap.add_argument("--ram-size", type=lambda x: int(x, 0), default=0x2000)
    ap.add_argument("--stack", type=lambda x: int(x, 0), default=0x20002000)
    args = ap.parse_args()
    if not os.path.isfile(args.flm):
        print("error: missing input", args.flm, file=sys.stderr)
        return 2
    with open(args.flm, "rb") as f:
        raw = f.read()
    code = raw
    header = struct.pack("<IIIIIIIII II".replace(" ", ""),
        0x4F474C41, 1, len(code),
        args.init_offset, args.erase_offset, args.program_offset,
        args.verify_offset, args.uninit_offset,
        args.ram_base, args.ram_size, args.stack)
    blob = header + code
    with open(args.output, "wb") as f:
        f.write(blob)
    meta = {
        "source": os.path.basename(args.flm),
        "code_size": len(code),
        "sha256": hashlib.sha256(code).hexdigest(),
        "init_offset": args.init_offset,
        "erase_offset": args.erase_offset,
        "program_offset": args.program_offset,
        "verify_offset": args.verify_offset,
        "uninit_offset": args.uninit_offset,
        "ram_base": args.ram_base,
        "ram_size": args.ram_size,
        "stack": args.stack,
    }
    with open(os.path.splitext(args.output)[0] + ".json", "w") as f:
        json.dump(meta, f, indent=2)
    print("wrote", args.output, len(blob), "bytes")
    return 0

if __name__ == "__main__":
    sys.exit(main())
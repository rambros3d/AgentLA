"""Host regression test for the streaming tar unpacker write sizing.

The firmware unpacker (firmware-rs/src/apps.rs) uses the pure helper in
firmware-rs/src/tar_io.rs to cap each LittleFS write_chunk call at one 512-byte
tar block. Before the fix, a feed chunk of up to 1024 bytes was forwarded
whole to write_chunk; multi-page writes failed on the RP2350 flash path and
surfaced as `incomplete_archive` for files larger than 512 bytes.

This model mirrors the Header/Data/Padding/Done state machine and verifies the
new chunk planning against real ustar archives, including a 97 KB payload.
"""

import io
import tarfile

MAX_WRITE_CHUNK = 512
FEED_CHUNK = 1024
BLOCK = 512


def data_chunk_len(file_left: int, available: int) -> int:
    """Mirror of firmware-rs/src/tar_io.rs::data_chunk_len."""
    return min(available, file_left, MAX_WRITE_CHUNK)


def parse_octal(field: bytes) -> int:
    value = 0
    started = False
    for b in field:
        if b in (0x20, 0):
            if started:
                break
        elif 0x30 <= b <= 0x37:
            started = True
            value = value * 8 + (b - 0x30)
        else:
            break
    return value


def make_tar(files):
    """files: list of (name, size) -> bytes of a USTAR archive."""
    out = io.BytesIO()
    with tarfile.open(fileobj=out, mode="w", format=tarfile.USTAR_FORMAT) as tar:
        for name, size in files:
            info = tarfile.TarInfo(name=name)
            info.size = size
            info.mode = 0o644
            info.mtime = 0
            tar.addfile(info, io.BytesIO(bytes((i % 251 for i in range(size)))))
    return out.getvalue()


def unpack(tar_bytes, feed_chunk, capped=True):
    """Return (state, writes, files) for the modelled state machine."""
    state = "HEADER"
    hdr = bytearray()
    file_size = 0
    written = 0
    pad_needed = 0
    pad_done = 0
    path = ""
    writes = []
    files = []

    pos = 0
    while pos < len(tar_bytes):
        data = tar_bytes[pos : pos + feed_chunk]
        pos += len(data)
        off = 0
        while off < len(data):
            if state == "DONE":
                break
            if state == "HEADER":
                take = min(BLOCK - len(hdr), len(data) - off)
                hdr.extend(data[off : off + take])
                off += take
                if len(hdr) == BLOCK:
                    hdr = bytearray(hdr)
                    if all(b == 0 for b in hdr):
                        state = "DONE"
                        hdr.clear()
                        break
                    name_end = hdr[:100].find(0)
                    if name_end < 0:
                        name_end = 100
                    name = hdr[:name_end].decode("ascii")
                    file_size = parse_octal(hdr[124:136])
                    typeflag = chr(hdr[156])
                    hdr.clear()
                    if typeflag != "5" and not name.endswith("/"):
                        path = name
                        written = 0
                        files.append(name)
                        state = "DATA" if file_size else "HEADER"
                    else:
                        state = "HEADER"
            elif state == "DATA":
                left = file_size - written
                take = min(len(data) - off, left)
                if capped:
                    take = data_chunk_len(left, len(data) - off)
                if take <= 0:
                    if written >= file_size:
                        rem = file_size % BLOCK
                        pad_needed = BLOCK - rem if rem else 0
                        pad_done = 0
                        state = "PADDING" if pad_needed else "HEADER"
                    continue
                writes.append((path, written, take))
                written += take
                off += take
                if written >= file_size:
                    rem = file_size % BLOCK
                    pad_needed = BLOCK - rem if rem else 0
                    pad_done = 0
                    state = "PADDING" if pad_needed else "HEADER"
            elif state == "PADDING":
                take = min(len(data) - off, pad_needed - pad_done)
                pad_done += take
                off += take
                if pad_done >= pad_needed:
                    state = "HEADER"
            else:
                break
    return state, writes, files


def test_1kb_4kb_and_97kb_files_unpack_with_capped_writes():
    sizes = {"small.bin": 1024, "medium.bin": 4096, "bundle.js.gz": 97 * 1024}
    tar = make_tar([(name, size) for name, size in sizes.items()])
    state, writes, files = unpack(tar, FEED_CHUNK, capped=True)

    assert state == "DONE", "archive did not reach end-of-archive marker"
    assert files == list(sizes), files
    assert all(n <= MAX_WRITE_CHUNK for _, _, n in writes), "oversized write_chunk"

    for name, size in sizes.items():
        parts = [n for path, _, n in writes if path == name]
        assert sum(parts) == size, (name, sum(parts), size)
        # Writes must be contiguous from offset 0.
        offsets = [off for path, off, _ in writes if path == name]
        assert offsets == list(range(0, size, MAX_WRITE_CHUNK)), (name, offsets[:5])


def test_old_uncapped_path_emits_oversized_writes():
    """Documents the original defect: >512B write_chunk calls for a 4KB file."""
    tar = make_tar([("medium.bin", 4096)])
    state, writes, files = unpack(tar, FEED_CHUNK, capped=False)
    assert state == "DONE"
    assert any(n > MAX_WRITE_CHUNK for _, _, n in writes), "expected >512B writes from old path"


def test_partial_final_page_is_preserved():
    tar = make_tar([("odd.bin", 1000)])
    state, writes, files = unpack(tar, FEED_CHUNK, capped=True)
    assert state == "DONE"
    assert sum(n for _, _, n in writes) == 1000
    assert writes[-1][2] == 1000 - 512  # 512 + 488, no data lost at the tail

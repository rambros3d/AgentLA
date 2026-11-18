//! Page-sized write planning for the streaming tar unpacker.
//!
//! Every `write_chunk` call opens, seeks, writes and closes a LittleFS file.
//! On the RP2350 the flash driver programs inside a critical section with
//! CORE1 paused, so passing multi-page chunks from the HTTP read buffer
//! stretched IRQ latency and was observed to fail on the WebUI update path.
//! The unpacker therefore issues at most one 512-byte tar block (two 256-byte
//! flash pages) per call and loops within the same `feed` invocation.

/// Maximum bytes handed to a single `write_chunk` call.
pub const MAX_WRITE_CHUNK: usize = 512;

/// How many of `available` bytes may be written now for a file that still has
/// `file_left` bytes remaining. Never more than one tar block, never past the
/// end of the file, never more than the bytes in the current feed chunk.
#[inline]
pub fn data_chunk_len(file_left: u32, available: usize) -> usize {
    available.min(file_left as usize).min(MAX_WRITE_CHUNK)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn caps_at_one_tar_block() {
        assert_eq!(data_chunk_len(4096, 1024), 512);
        assert_eq!(data_chunk_len(97 * 1024, 4096), 512);
        assert_eq!(data_chunk_len(0x138000, 1024), 512);
    }

    #[test]
    fn respects_file_end_and_feed_size() {
        assert_eq!(data_chunk_len(300, 1024), 300);
        assert_eq!(data_chunk_len(4096, 200), 200);
        assert_eq!(data_chunk_len(0, 1024), 0);
        assert_eq!(data_chunk_len(4096, 0), 0);
    }

    #[test]
    fn streams_large_file_in_page_sized_writes() {
        // A 97 KiB file fed in 1 KiB chunks must never produce a single write
        // larger than 512 B, and the whole file must be emitted exactly once.
        let file_size = 97u32 * 1024;
        let mut written = 0u32;
        let mut in_chunk = 0usize;
        let mut max_write = 0usize;
        let mut writes = 0usize;
        while written < file_size {
            if in_chunk >= 1024 {
                in_chunk = 0;
            }
            let n = data_chunk_len(file_size - written, 1024 - in_chunk);
            assert!(n > 0 && n <= MAX_WRITE_CHUNK);
            max_write = max_write.max(n);
            writes += 1;
            written += n as u32;
            in_chunk += n;
        }
        assert_eq!(written, file_size);
        assert_eq!(max_write, MAX_WRITE_CHUNK);
        assert_eq!(writes, (file_size as usize + 511) / 512);
    }
}

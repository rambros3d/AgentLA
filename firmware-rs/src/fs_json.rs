//! Pure, allocation-free JSON string helpers for the filesystem REST API.
//!
//! This module is deliberately independent of littlefs2, picoserve and the
//! embedded runtime so the escaping logic can be exercised by host-side unit
//! tests (`rustc --test src/fs_json.rs`) while the firmware uses the exact same
//! implementation. Every writer is generic over [`core::fmt::Write`]: on the
//! device the sink is `heapless::String<N>`, in tests it is `std::string::String`.

/// Append `value` to `out` as a complete, escaped JSON string literal
/// (including surrounding double quotes).
///
/// Quotes, backslashes, the standard control escapes and every other C0
/// control character are escaped so a filesystem-derived name can never break
/// out of the JSON payload. Returns `Err` as soon as `out` refuses a byte,
/// which callers treat as an oversized response rather than emitting a
/// truncated one.
pub fn write_json_string<W: core::fmt::Write>(out: &mut W, value: &str) -> core::fmt::Result {
    out.write_char('"')?;
    for ch in value.chars() {
        match ch {
            '"' => out.write_str("\\\"")?,
            '\\' => out.write_str("\\\\")?,
            '\u{08}' => out.write_str("\\b")?,
            '\u{0c}' => out.write_str("\\f")?,
            '\n' => out.write_str("\\n")?,
            '\r' => out.write_str("\\r")?,
            '\t' => out.write_str("\\t")?,
            c if (c as u32) < 0x20 => {
                const HEX: &[u8; 16] = b"0123456789abcdef";
                let code = c as u32;
                out.write_str("\\u00")?;
                out.write_char(HEX[((code >> 4) & 0x0f) as usize] as char)?;
                out.write_char(HEX[(code & 0x0f) as usize] as char)?;
            }
            c => out.write_char(c)?,
        }
    }
    out.write_char('"')
}

/// Append one directory-entry object to a JSON array being built in `out`.
///
/// `first` tracks whether an object has already been emitted so commas are
/// placed correctly. Any write failure (including a full `heapless::String`)
/// is propagated to the caller.
pub fn write_dir_entry<W: core::fmt::Write>(
    out: &mut W,
    first: &mut bool,
    name: &str,
    is_dir: bool,
    size: usize,
) -> core::fmt::Result {
    if !*first {
        out.write_char(',')?;
    }
    *first = false;
    out.write_str(r#"{"name":"#)?;
    write_json_string(out, name)?;
    write!(out, r#","is_dir":{},"size":{}}}"#, is_dir, size)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn escaped(value: &str) -> String {
        let mut out = String::new();
        write_json_string(&mut out, value).expect("escape must succeed for a growable buffer");
        out
    }

    fn entry(name: &str, is_dir: bool, size: usize) -> String {
        let mut out = String::new();
        let mut first = true;
        write_dir_entry(&mut out, &mut first, name, is_dir, size).expect("entry write must succeed");
        out
    }

    /// A `fmt::Write` sink with a hard byte cap, used to prove that overflow is
    /// reported instead of silently truncating (the original 2048-byte bug).
    struct Capped {
        bytes: usize,
        cap: usize,
    }

    impl core::fmt::Write for Capped {
        fn write_str(&mut self, s: &str) -> core::fmt::Result {
            self.bytes += s.len();
            if self.bytes > self.cap {
                Err(core::fmt::Error)
            } else {
                Ok(())
            }
        }
    }

    #[test]
    fn plain_name_is_unchanged() {
        assert_eq!(escaped("readme.txt"), "\"readme.txt\"");
        assert_eq!(escaped(""), "\"\"");
    }

    #[test]
    fn double_quote_is_escaped() {
        // Requirement: a filename containing a double quote yields valid JSON.
        assert_eq!(escaped(r#"odd"name"#), r#""odd\"name""#);
    }

    #[test]
    fn backslash_is_escaped() {
        assert_eq!(escaped(r"a\b\c"), r#""a\\b\\c""#);
    }

    #[test]
    fn quote_and_backslash_combined() {
        assert_eq!(escaped(r#"a"b\c"#), r#""a\"b\\c""#);
    }

    #[test]
    fn control_characters_are_escaped() {
        assert_eq!(escaped("a\nb\tc\r"), r#""a\nb\tc\r""#);
        assert_eq!(escaped("\u{01}"), r#""\u0001""#);
        assert_eq!(escaped("\u{1f}"), r#""\u001f""#);
    }

    #[test]
    fn dir_entry_json_shape_and_escaping() {
        assert_eq!(
            entry(r#"a"b"#, false, 512),
            r#"{"name":"a\"b","is_dir":false,"size":512}"#
        );
        assert_eq!(
            entry("fonts", true, 0),
            r#"{"name":"fonts","is_dir":true,"size":0}"#
        );
    }

    #[test]
    fn comma_placement_across_entries() {
        let mut out = String::from("[");
        let mut first = true;
        write_dir_entry(&mut out, &mut first, "one", false, 1).unwrap();
        write_dir_entry(&mut out, &mut first, "two", true, 0).unwrap();
        out.push(']');
        assert_eq!(
            out,
            r#"[{"name":"one","is_dir":false,"size":1},{"name":"two","is_dir":true,"size":0}]"#
        );
    }

    #[test]
    fn overflow_is_reported_not_truncated() {
        let mut capped = Capped { bytes: 0, cap: 8 };
        assert!(write_json_string(&mut capped, "a-long-filename").is_err());
    }

    #[test]
    fn large_listing_serializes_as_complete_json() {
        // Simulate a directory whose entries exceed the old 2048-byte limit.
        let mut out = String::from("[");
        let mut first = true;
        for i in 0..64 {
            let name = format!("asset_with_a_deliberately_long_name_{:04}.bin", i);
            write_dir_entry(&mut out, &mut first, &name, false, 4096).unwrap();
        }
        out.push(']');

        assert!(out.len() > 2048, "fixture must exceed the old buffer");
        assert!(out.starts_with('[') && out.ends_with(']'));
        assert_eq!(out.matches(r#"{"name":"#).count(), 64);
        assert!(!out.contains(",,"), "no empty slots from failed writes");
        assert_eq!(
            out.matches('{').count(),
            out.matches('}').count(),
            "braces must balance"
        );
    }
}

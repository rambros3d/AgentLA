//! Run-length and bit-delta compression algorithms for logic analyzer samples.

pub const TAG_ZERO_SHORT: u8 = 0x00;
pub const TAG_WORD_DELTA: u8 = 0x02;

pub const fn rle_encode(src: &[u8], dst: &mut [u8], dst_cap: usize) -> usize {
    let mut i: usize = 0;
    let mut out: usize = 0;
    'outer: while i < src.len() {
        let val = src[i];
        let mut run: usize = 1;
        while i + run < src.len() && run < 255 && src[i + run] == val {
            run += 1;
        }
        if out + 2 > dst_cap || out + 2 > dst.len() {
            break 'outer;
        }
        dst[out] = run as u8;
        out += 1;
        dst[out] = val;
        out += 1;
        i += run;
    }
    out
}

pub const fn rle_decode(src: &[u8], dst: &mut [u8], dst_cap: usize) -> usize {
    let mut inpos: usize = 0;
    let mut out: usize = 0;
    while inpos + 1 < src.len() {
        let run = src[inpos];
        inpos += 1;
        let val = src[inpos];
        inpos += 1;
        if out + (run as usize) > dst_cap || out + (run as usize) > dst.len() {
            break;
        }
        let mut r: usize = 0;
        while r < (run as usize) {
            dst[out + r] = val;
            r += 1;
        }
        out += run as usize;
    }
    out
}

pub fn bit_delta_transform_u32(src: &[u32], delta_out: &mut [u32], prev_state: &mut u32) {
    let mut prev = *prev_state;
    let limit = src.len().min(delta_out.len());
    let mut i = 0;
    while i < limit {
        let s = src[i];
        delta_out[i] = s ^ prev;
        prev = s;
        i += 1;
    }
    *prev_state = prev;
}

pub fn bit_delta_untransform_u32(delta: &[u32], dest: &mut [u32], prev_state: &mut u32) {
    let mut prev = *prev_state;
    let limit = delta.len().min(dest.len());
    let mut i = 0;
    while i < limit {
        prev ^= delta[i];
        dest[i] = prev;
        i += 1;
    }
    *prev_state = prev;
}

pub fn bit_delta_compress_block(src_words: &[u32], dst: &mut [u8]) -> usize {
    let mut prev: u32 = 0;
    let mut inpos: usize = 0;
    let mut out: usize = 0;
    let dst_cap = dst.len();
    while inpos < src_words.len() {
        let delta = src_words[inpos] ^ prev;
        if delta == 0 {
            let mut run: usize = 1;
            while inpos + run < src_words.len()
                && run < 65535
                && (src_words[inpos + run] ^ src_words[inpos + run - 1]) == 0
            {
                run += 1;
            }
            if run <= 255 {
                if out + 2 > dst_cap {
                    return 0;
                }
                dst[out] = TAG_ZERO_SHORT;
                out += 1;
                dst[out] = run as u8;
                out += 1;
            }
            inpos += run;
        } else {
            if out + 5 > dst_cap {
                return 0;
            }
            dst[out] = TAG_WORD_DELTA;
            out += 1;
            dst[out] = (delta & 0xFF) as u8;
            out += 1;
            dst[out] = ((delta >> 8) & 0xFF) as u8;
            out += 1;
            dst[out] = ((delta >> 16) & 0xFF) as u8;
            out += 1;
            dst[out] = ((delta >> 24) & 0xFF) as u8;
            out += 1;
            prev = src_words[inpos];
            inpos += 1;
        }
    }
    out
}

pub fn bit_delta_decompress_block(src: &[u8], dst_words: &mut [u32]) -> usize {
    let limit = src.len();
    let mut prev: u32 = 0;
    let mut inpos: usize = 0;
    let mut out: usize = 0;
    while inpos < limit {
        let tag = src[inpos];
        inpos += 1;
        if tag == TAG_ZERO_SHORT {
            if inpos >= limit {
                return 0;
            }
            let run = src[inpos] as usize;
            inpos += 1;
            if out + run > dst_words.len() {
                return 0;
            }
            for r in 0..run {
                dst_words[out + r] = prev;
            }
            out += run;
        } else if tag == TAG_WORD_DELTA {
            if inpos + 4 > limit {
                return 0;
            }
            let mut delta = src[inpos] as u32;
            inpos += 1;
            delta |= (src[inpos] as u32) << 8;
            inpos += 1;
            delta |= (src[inpos] as u32) << 16;
            inpos += 1;
            delta |= (src[inpos] as u32) << 24;
            inpos += 1;
            if out >= dst_words.len() {
                return 0;
            }
            prev ^= delta;
            dst_words[out] = prev;
            out += 1;
        } else {
            return 0;
        }
    }
    out
}

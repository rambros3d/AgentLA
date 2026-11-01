#include "rle_util.h"
#include <string.h>

/* Simple RLE encode: emits (run_length, value) pairs, max 255 per run.
   Returns number of output bytes written or 0 on overrun. */
size_t rle_encode(const uint8_t *src, size_t src_len, uint8_t *dst, size_t dst_cap) {
    if (!src || !dst || dst_cap == 0) return 0;
    size_t i = 0, out = 0;
    while (i < src_len) {
        uint8_t val = src[i];
        size_t run = 1;
        while (i + run < src_len && run < 255 && src[i + run] == val) run++;
        if (out + 2 > dst_cap) return 0;
        dst[out++] = (uint8_t)run;
        dst[out++] = val;
        i += run;
    }
    return out;
}

/* Simple RLE decode: decodes (run_length, value) pairs.
   Returns number of bytes written to dst, or 0 on overrun. */
size_t rle_decode(const uint8_t *src, size_t src_len, uint8_t *dst, size_t dst_cap) {
    if (!src || !dst || dst_cap == 0) return 0;
    size_t in = 0, out = 0;
    while (in + 1 < src_len) {
        uint8_t run = src[in++];
        uint8_t val = src[in++];
        if (out + run > dst_cap) return 0;
        memset(dst + out, val, run);
        out += run;
    }
    return out;
}

/* Cortex-M33 DSP single-cycle EOR bit-delta transformation:
   delta[0] = src[0] ^ prev_state;
   delta[i] = src[i] ^ src[i-1];
   Computes word differences in single-cycle EOR instructions. */
void bit_delta_transform_u32(const uint32_t *src, uint32_t *delta_out, size_t num_words, uint32_t *prev_state) {
    if (!src || !delta_out || !prev_state) return;
    uint32_t prev = *prev_state;
    size_t i = 0;

#if defined(__arm__) || defined(__thumb__)
    while (i + 4 <= num_words) {
        uint32_t s0 = src[i];
        uint32_t s1 = src[i+1];
        uint32_t s2 = src[i+2];
        uint32_t s3 = src[i+3];
        uint32_t d0, d1, d2, d3;

        __asm__ volatile (
            "eor %0, %4, %5\n\t"
            "eor %1, %6, %4\n\t"
            "eor %2, %7, %6\n\t"
            "eor %3, %8, %7\n\t"
            : "=r"(d0), "=r"(d1), "=r"(d2), "=r"(d3)
            : "r"(s0), "r"(prev), "r"(s1), "r"(s2), "r"(s3)
        );

        delta_out[i]   = d0;
        delta_out[i+1] = d1;
        delta_out[i+2] = d2;
        delta_out[i+3] = d3;

        prev = s3;
        i += 4;
    }
#endif

    while (i < num_words) {
        uint32_t s = src[i];
        delta_out[i] = s ^ prev;
        prev = s;
        i++;
    }
    *prev_state = prev;
}

/* Untransform delta back to original words */
void bit_delta_untransform_u32(const uint32_t *delta, uint32_t *dest, size_t num_words, uint32_t *prev_state) {
    if (!delta || !dest || !prev_state) return;
    uint32_t prev = *prev_state;
    for (size_t i = 0; i < num_words; i++) {
        prev ^= delta[i];
        dest[i] = prev;
    }
    *prev_state = prev;
}

/* Fast bit-delta compression for 64 KB DMA blocks:
   Encodes runs of 0 delta words and non-zero delta words. */
#define TAG_ZERO_SHORT 0x00
#define TAG_ZERO_LONG  0x01
#define TAG_WORD_DELTA 0x02
#define TAG_RAW_RUN    0x03

size_t bit_delta_compress_block(const uint32_t *src_words, size_t num_words, uint8_t *dst, size_t dst_cap) {
    if (!src_words || !dst || num_words == 0 || dst_cap == 0) return 0;

    uint32_t prev = 0;
    size_t in = 0;
    size_t out = 0;

    while (in < num_words) {
        uint32_t s = src_words[in];
        uint32_t delta = s ^ prev;

        if (delta == 0) {
            /* Zero run (static pin state) */
            size_t run = 1;
            while (in + run < num_words && run < 65535 && (src_words[in + run] ^ src_words[in + run - 1]) == 0) {
                run++;
            }
            if (run <= 255) {
                if (out + 2 > dst_cap) return 0;
                dst[out++] = TAG_ZERO_SHORT;
                dst[out++] = (uint8_t)run;
            } else {
                if (out + 3 > dst_cap) return 0;
                dst[out++] = TAG_ZERO_LONG;
                dst[out++] = (uint8_t)(run & 0xFF);
                dst[out++] = (uint8_t)(run >> 8);
            }
            in += run;
            /* prev remains identical because all intermediate samples were identical */
        } else {
            /* Single non-zero delta word */
            if (out + 5 > dst_cap) return 0;
            dst[out++] = TAG_WORD_DELTA;
            dst[out++] = (uint8_t)(delta & 0xFF);
            dst[out++] = (uint8_t)((delta >> 8) & 0xFF);
            dst[out++] = (uint8_t)((delta >> 16) & 0xFF);
            dst[out++] = (uint8_t)((delta >> 24) & 0xFF);
            prev = s;
            in++;
        }
    }

    return out;
}

/* Decompress bit-delta compressed block */
size_t bit_delta_decompress_block(const uint8_t *src, size_t src_len, uint32_t *dst_words, size_t dst_cap_words) {
    if (!src || !dst_words || src_len == 0 || dst_cap_words == 0) return 0;

    uint32_t prev = 0;
    size_t in = 0;
    size_t out = 0;

    while (in < src_len) {
        uint8_t tag = src[in++];
        if (tag == TAG_ZERO_SHORT) {
            if (in >= src_len) return 0;
            uint8_t run = src[in++];
            if (out + run > dst_cap_words) return 0;
            for (size_t r = 0; r < run; r++) dst_words[out++] = prev;
        } else if (tag == TAG_ZERO_LONG) {
            if (in + 1 >= src_len) return 0;
            uint16_t run = src[in++];
            run |= ((uint16_t)src[in++]) << 8;
            if (out + run > dst_cap_words) return 0;
            for (size_t r = 0; r < run; r++) dst_words[out++] = prev;
        } else if (tag == TAG_WORD_DELTA) {
            if (in + 4 > src_len) return 0;
            uint32_t delta = src[in++];
            delta |= ((uint32_t)src[in++]) << 8;
            delta |= ((uint32_t)src[in++]) << 16;
            delta |= ((uint32_t)src[in++]) << 24;
            if (out >= dst_cap_words) return 0;
            prev ^= delta;
            dst_words[out++] = prev;
        } else {
            return 0; /* Unknown tag */
        }
    }

    return out;
}

/* Pipeline statistics */
static compression_pipeline_stats_t g_pipeline_stats;
static uint32_t g_consecutive_overflows = 0;

void compression_pipeline_get_stats(compression_pipeline_stats_t *stats) {
    if (!stats) return;
    *stats = g_pipeline_stats;
}

void compression_pipeline_reset_stats(void) {
    memset(&g_pipeline_stats, 0, sizeof(g_pipeline_stats));
    g_consecutive_overflows = 0;
}

void compression_pipeline_record_buffer(size_t in_bytes, size_t out_bytes, bool overflow) {
    g_pipeline_stats.total_in_bytes += (uint32_t)in_bytes;
    g_pipeline_stats.total_out_bytes += (uint32_t)out_bytes;
    g_pipeline_stats.buffers_processed++;

    if (overflow) {
        g_consecutive_overflows++;
        if (g_consecutive_overflows >= 3) {
            g_pipeline_stats.backpressure_events++;
            g_pipeline_stats.dropped_frames++;
            g_pipeline_stats.backpressure_flag = true;
        }
    } else {
        g_consecutive_overflows = 0;
        g_pipeline_stats.backpressure_flag = false;
    }
}

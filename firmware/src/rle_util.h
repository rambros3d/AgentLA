#ifndef __RLE_UTIL_H__
#define __RLE_UTIL_H__

#include <stddef.h>
#include <stdint.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Standard byte-level RLE encoder/decoder */
size_t rle_encode(const uint8_t *src, size_t src_len, uint8_t *dst, size_t dst_cap);
size_t rle_decode(const uint8_t *src, size_t src_len, uint8_t *dst, size_t dst_cap);

/* Cortex-M33 DSP single-cycle EOR bit-delta transformation:
   delta[0] = src[0] ^ prev_state;
   delta[i] = src[i] ^ src[i-1];
   Computes word differences in single-cycle EOR instructions. */
void bit_delta_transform_u32(const uint32_t *src, uint32_t *delta_out, size_t num_words, uint32_t *prev_state);
void bit_delta_untransform_u32(const uint32_t *delta, uint32_t *dest, size_t num_words, uint32_t *prev_state);

/* Fast bit-delta compression for 64 KB DMA blocks:
   Applies single-cycle EOR difference followed by zero-run / delta packing.
   Returns compressed byte size, or 0 on buffer overflow. */
size_t bit_delta_compress_block(const uint32_t *src_words, size_t num_words, uint8_t *dst, size_t dst_cap);
size_t bit_delta_decompress_block(const uint8_t *src, size_t src_len, uint32_t *dst_words, size_t dst_cap_words);

/* Dual-Core Compression Pipeline state and metrics */
typedef struct {
    uint32_t total_in_bytes;
    uint32_t total_out_bytes;
    uint32_t buffers_processed;
    uint32_t dropped_frames;
    uint32_t backpressure_events;
    bool backpressure_flag;
} compression_pipeline_stats_t;

void compression_pipeline_get_stats(compression_pipeline_stats_t *stats);
void compression_pipeline_reset_stats(void);
void compression_pipeline_record_buffer(size_t in_bytes, size_t out_bytes, bool overflow);

#ifdef __cplusplus
}
#endif

#endif /* __RLE_UTIL_H__ */

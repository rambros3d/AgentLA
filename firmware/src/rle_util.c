#include <stddef.h>
#include <stdint.h>

/* Simple RLE encode: emits (run_length, value) pairs, max 255 per run.
   Returns number of output bytes written or 0 on overrun. */
size_t rle_encode(const uint8_t *src, size_t src_len, uint8_t *dst, size_t dst_cap) {
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

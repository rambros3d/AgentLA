#ifndef __AGENTLA_DECODE_SWD__
#define __AGENTLA_DECODE_SWD__

#include "decode.h"
#include <stdint.h>
#include <stdbool.h>

void swd_decode(const uint8_t* buffer, uint32_t length, uint32_t sample_rate,
                uint8_t clk_pin, uint8_t dio_pin, uint8_t pin_base,
                uint32_t buf_offset, DECODER_EVENT_ARENA* arena);

#endif

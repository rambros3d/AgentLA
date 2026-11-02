#ifndef __AGENTLA_DECODE_I3C__
#define __AGENTLA_DECODE_I3C__

#include "decode.h"
#include <stdint.h>
#include <stdbool.h>

void i3c_decode(const uint8_t* buffer, uint32_t length, uint32_t sample_rate,
                uint8_t sda_pin, uint8_t scl_pin, uint8_t pin_base,
                uint32_t buf_offset, DECODER_EVENT_ARENA* arena);

#endif

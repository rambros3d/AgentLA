#ifndef __AGENTLA_DECODE_1WIRE__
#define __AGENTLA_DECODE_1WIRE__

#include "decode.h"
#include <stdint.h>
#include <stdbool.h>

void onewire_decode(const uint8_t* buffer, uint32_t length, uint32_t sample_rate,
                    uint8_t pin, uint8_t pin_base, uint32_t buf_offset,
                    DECODER_EVENT_ARENA* arena);

#endif

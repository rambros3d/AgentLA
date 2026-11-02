#ifndef __AGENTLA_DECODE_SPIFLASH__
#define __AGENTLA_DECODE_SPIFLASH__

#include "decode.h"
#include <stdint.h>
#include <stdbool.h>

void spiflash_decode(const uint8_t* buffer, uint32_t length, uint32_t sample_rate,
                     uint8_t cs_pin, uint8_t sclk_pin, uint8_t mosi_pin, uint8_t miso_pin,
                     uint8_t pin_base, uint32_t buf_offset, DECODER_EVENT_ARENA* arena);

#endif

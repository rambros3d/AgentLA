#ifndef __AGENTLA_CAPTURE__
#define __AGENTLA_CAPTURE__

#include "board_settings.h"
#include "pin_owner.h"
#include "structs.h"

#define SCOPE_PRE_SIZE   (64 * 1024)
#define SCOPE_POST_SIZE  (352 * 1024)
#define SCOPE_TOTAL_SIZE (SCOPE_PRE_SIZE + SCOPE_POST_SIZE) /* 416 KB unified acquisition */

extern uint8_t captureBuffer[];

typedef enum {
    CAPTURE_ENGINE_RAW = 0,
    CAPTURE_ENGINE_STREAM = 1,
    CAPTURE_ENGINE_TURBO = 2,
    CAPTURE_ENGINE_SCOPE = 3
} capture_engine_mode_t;

typedef struct {
    uint32_t mask;
    uint32_t match;
    uint8_t type;       /* 0: pattern/level, 1: rising edge, 2: falling edge */
    uint8_t pre_pct;    /* pre-trigger percent (default 50) */
    uint32_t pre_size;  /* bytes (default 64KB) */
    uint32_t post_size; /* bytes (default 384KB) */
} trigger_config_t;

void capture_set_engine_mode(capture_engine_mode_t mode);
capture_engine_mode_t capture_get_engine_mode(void);
const char* capture_get_engine_mode_name(void);
void capture_set_rle_threshold(uint32_t ticks);
uint32_t capture_get_rle_threshold(void);
void capture_set_trigger_config(const trigger_config_t *cfg);
const trigger_config_t* capture_get_trigger_config(void);

/* Mode A: In-Hardware PIO RLE Streaming */
bool StartCaptureRle(uint32_t freq, const uint8_t* capturePins, uint8_t capturePinCount);
uint32_t* capture_get_rle_buffer(uint32_t *packet_count);

/* Mode B: Turbo Dual-Core Compression */
bool StartCaptureTurbo(uint32_t freq, uint32_t length, const uint8_t* capturePins, uint8_t capturePinCount, CHANNEL_MODE captureMode);
uint8_t* capture_get_turbo_buffer(uint32_t *compressed_size);

/* Mode C: Oscilloscope Circular Pre/Post-Trigger */
bool StartCaptureScope(uint32_t freq, uint32_t preBytes, uint32_t postBytes, const uint8_t* capturePins, uint8_t capturePinCount, uint8_t triggerPin, uint8_t triggerType, CHANNEL_MODE captureMode);
uint8_t* capture_get_scope_unified_buffer(uint32_t *total_bytes, uint32_t *pre_bytes, uint32_t *post_bytes);

bool StartCaptureSimple(uint32_t freq, uint32_t preLength, uint32_t postLength, const uint8_t* capturePins, uint8_t capturePinCount, uint8_t triggerPin, bool invertTrigger, CHANNEL_MODE captureMode);
bool StartCaptureNoTrigger(uint32_t freq, uint32_t length, const uint8_t* capturePins, uint8_t capturePinCount, CHANNEL_MODE captureMode);
bool StartCaptureNoTriggerSkip(uint32_t freq, uint32_t length, const uint8_t* capturePins, uint8_t capturePinCount, CHANNEL_MODE captureMode, uint8_t skipPin);
bool StartCaptureBlast(uint32_t freq, uint32_t length, const uint8_t* capturePins, uint8_t capturePinCount, uint8_t triggerPin, bool invertTrigger, CHANNEL_MODE captureMode);
bool StartCaptureComplex(uint32_t freq, uint32_t preLength, uint32_t postLength, const uint8_t* capturePins, uint8_t capturePinCount, uint8_t triggerPinBase, uint8_t triggerPinCount, uint16_t triggerValue, CHANNEL_MODE captureMode);
bool StartCaptureFast(uint32_t freq, uint32_t preLength, uint32_t postLength, const uint8_t* capturePins, uint8_t capturePinCount, uint8_t triggerPinBase, uint8_t triggerPinCount, uint16_t triggerValue, CHANNEL_MODE captureMode);
void StopCapture();
bool IsCapturing();
uint GetCaptureSM();
uint8_t* GetBuffer(uint32_t* bufferSize, uint32_t* firstSample, CHANNEL_MODE* captureMode);
void check_fast_interrupt();
uint8_t gpio_to_pindex(uint8_t gpio);

extern const uint8_t pinMap[];

bool StartAdcCapture(uint32_t pin, uint32_t freq, uint32_t samples);
bool IsAdcCapturing();
uint16_t* GetAdcBuffer(uint32_t* size, uint32_t* first);

#endif

bool capture_claim_bank(const uint8_t* channels, uint8_t channel_count, uint8_t trigger_pin, uint32_t* claim_handle);
bool capture_release_bank(uint32_t claim_handle, pin_owner_result_t* result);
uint32_t capture_active_bank_handle(void);
bool capture_validate_claimed_pins(const uint8_t* channels, uint8_t channel_count, uint8_t trigger_pin);

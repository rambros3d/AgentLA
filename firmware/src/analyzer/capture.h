#ifndef __AGENTLA_CAPTURE__
#define __AGENTLA_CAPTURE__

#include "board_settings.h"
#include "pin_owner.h"
#include "structs.h"

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

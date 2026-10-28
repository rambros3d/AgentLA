#ifndef __AGENTLA_PIN_OWNER__
#define __AGENTLA_PIN_OWNER__

#include "pico/stdlib.h"
#include "hardware/gpio.h"

#include <stdbool.h>
#include <stdint.h>

#define PIN_OWNER_FLEX_MIN 10
#define PIN_OWNER_FLEX_MAX 29
#define PIN_OWNER_OBSERVER_MAX 8

typedef enum {
    PIN_OWNER_RESOURCE_PIN = 0,
    PIN_OWNER_RESOURCE_INSTANCE = 1,
    PIN_OWNER_RESOURCE_STREAM_BUFFER = 2,
    PIN_OWNER_RESOURCE_CAPTURE_BANK = 3,
} pin_owner_resource_kind_t;

typedef enum {
    PIN_OWNER_OK = 0,
    PIN_OWNER_ERROR_INVALID_PIN,
    PIN_OWNER_ERROR_FIXED_PIN,
    PIN_OWNER_ERROR_UNSUPPORTED_FUNCTION,
    PIN_OWNER_ERROR_DRIVER_CONFLICT,
    PIN_OWNER_ERROR_OBSERVERS,
    PIN_OWNER_ERROR_QUIESCE_FAILED,
    PIN_OWNER_ERROR_NOT_FOUND,
    PIN_OWNER_ERROR_ALREADY_RELEASED,
    PIN_OWNER_ERROR_CAPACITY,
} pin_owner_result_t;

typedef struct {
    uint32_t claim_id;
    uint8_t pins[8];
    uint8_t pin_count;
    uint8_t function;
    uint8_t observer_only;
    uint8_t release_state;
    uint8_t release_ok;
    uint8_t active;
} pin_owner_claim_t;

typedef struct {
    uint8_t package_class;
    uint8_t flexible_pin_count;
    uint8_t analog_capable_pin_count;
} pin_owner_inventory_t;

void pin_owner_init(void);
bool pin_owner_is_fixed(uint8_t pin);
bool pin_owner_is_flexible(uint8_t pin);
bool pin_owner_function_supported(uint8_t pin, uint8_t function);
bool pin_owner_can_claim_observer(uint8_t function);
bool pin_owner_claim_pins(const uint8_t *pins, uint8_t pin_count,
                          uint8_t function, uint8_t observer_only,
                          uint32_t *claim_id);
bool pin_owner_release_claim(uint32_t claim_id, pin_owner_result_t *result);
const pin_owner_claim_t *pin_owner_get_claim(uint32_t claim_id);
bool pin_owner_release_in_progress(uint32_t claim_id);
uint8_t pin_owner_observer_count(uint8_t pin);
uint8_t pin_owner_has_driver(uint8_t pin);
const pin_owner_inventory_t *pin_owner_get_inventory(void);

#endif /* __AGENTLA_PIN_OWNER__ */
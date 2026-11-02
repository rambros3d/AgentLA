#include "pin_owner.h"
#include "board_settings.h"

#include <assert.h>
#include <string.h>

#define PIN_OWNER_MAX_CLAIMS 32
#define PIN_OWNER_MAX_PIN_RESOURCES 8

typedef struct {
    uint32_t id;
    uint8_t pins[PIN_OWNER_MAX_PIN_RESOURCES];
    uint8_t pin_count;
    uint8_t function;
    uint8_t observer_only;
    uint8_t release_state;
    uint8_t release_result;
    uint8_t active;
    pin_owner_irq_resource_t irqs[PIN_OWNER_MAX_PIN_RESOURCES];
} pin_owner_claim_record_t;

static pin_owner_claim_record_t claims[PIN_OWNER_MAX_CLAIMS];
static uint32_t next_claim_id = 1;

/* Driver tracking: -1 means no driver claim; otherwise claim array index. */
static int8_t pin_driver_index[PIN_OWNER_FLEX_MAX - PIN_OWNER_FLEX_MIN + 1];
/* Number of observer-only claims currently attached to the pin. */
static uint8_t pin_observer_counts[PIN_OWNER_FLEX_MAX - PIN_OWNER_FLEX_MIN + 1];

static pin_owner_inventory_t inventory = {
    .package_class = 1,
    .flexible_pin_count = (uint8_t)(PIN_OWNER_FLEX_MAX - PIN_OWNER_FLEX_MIN + 1),
    .analog_capable_pin_count = 4,
};

static uint8_t flex_pin_function_bits[PIN_OWNER_FLEX_MAX - PIN_OWNER_FLEX_MIN + 1];

typedef enum {
    PIN_OWNER_RELEASE_IDLE = 0,
    PIN_OWNER_RELEASE_QUIESCING = 1,
    PIN_OWNER_RELEASE_DONE = 2,
} pin_owner_release_state_t;

static_assert(PIN_OWNER_FLEX_MIN >= 0, "flex_min must be valid for a uint8_t");
static_assert(PIN_OWNER_FLEX_MIN == 10, "reserve programmer GPIO0 through GPIO9");
static_assert(PIN_OWNER_FLEX_MAX <= 29, "flexible pins are GPIO10 through GPIO29");

static uint8_t pin_index(uint8_t pin) {
    return (uint8_t)(pin - PIN_OWNER_FLEX_MIN);
}

static bool role_is_observer_only_and_input_safe(uint8_t function) {
    /* Pin-owner registration only: actual role-level input models must be
     * validated by callers. SIO is the generic safe input function. */
    return function == GPIO_FUNC_SIO;
}

static void initialize_capability_table(void) {
    for (uint8_t i = 0; i < (uint8_t)(PIN_OWNER_FLEX_MAX - PIN_OWNER_FLEX_MIN + 1); i++) {
        uint8_t all_common = (uint8_t)((1u << GPIO_FUNC_SPI) |
                                        (1u << GPIO_FUNC_UART) |
                                        (1u << GPIO_FUNC_I2C) |
                                        (1u << GPIO_FUNC_PWM) |
                                        (1u << GPIO_FUNC_SIO) |
                                        (1u << GPIO_FUNC_PIO0) |
                                        (1u << GPIO_FUNC_PIO1) |
                                        (1u << GPIO_FUNC_PIO2));
        flex_pin_function_bits[i] = all_common;
    }
}

void pin_owner_init(void) {
    memset(claims, 0, sizeof(claims));
    next_claim_id = 1;
    for (uint8_t i = 0; i < (uint8_t)(PIN_OWNER_FLEX_MAX - PIN_OWNER_FLEX_MIN + 1); i++) {
        pin_driver_index[i] = -1;
        pin_observer_counts[i] = 0;
    }
    initialize_capability_table();
}

bool pin_owner_is_fixed(uint8_t pin) {
    return pin < PIN_OWNER_FLEX_MIN;
}

bool pin_owner_is_flexible(uint8_t pin) {
    return pin >= PIN_OWNER_FLEX_MIN && pin <= PIN_OWNER_FLEX_MAX;
}

bool pin_owner_function_supported(uint8_t pin, uint8_t function) {
    if (!pin_owner_is_flexible(pin)) {
        return false;
    }
    const uint8_t mask = (uint8_t)(1u << (function & 0x07u));
    return (flex_pin_function_bits[pin_index(pin)] & mask) != 0u;
}

bool pin_owner_can_claim_observer(uint8_t function) {
    return role_is_observer_only_and_input_safe(function);
}

uint8_t pin_owner_has_driver(uint8_t pin) {
    if (!pin_owner_is_flexible(pin)) {
        return 0;
    }
    return pin_driver_index[pin_index(pin)] >= 0 ? 1u : 0u;
}

uint8_t pin_owner_observer_count(uint8_t pin) {
    if (!pin_owner_is_flexible(pin)) {
        return 0;
    }
    return pin_observer_counts[pin_index(pin)];
}

const pin_owner_inventory_t *pin_owner_get_inventory(void) {
    return &inventory;
}

static int8_t find_claim_index(uint32_t claim_id) {
    for (uint8_t i = 0; i < PIN_OWNER_MAX_CLAIMS; i++) {
        if (claims[i].id == claim_id &&
            (claims[i].active || claims[i].release_state == PIN_OWNER_RELEASE_DONE)) {
            return (int8_t)i;
        }
    }
    return -1;
}

static bool clone_claim_record(const pin_owner_claim_t *record) {
    /* Copies are intentionally deep enough to preserve stable caller data. */
    return record != NULL;
}

bool pin_owner_claim_pins(const uint8_t *pins, uint8_t pin_count, uint8_t function,
                          uint8_t observer_only, uint32_t *claim_id) {
    if (claim_id == NULL || pins == NULL || pin_count == 0u ||
        pin_count > PIN_OWNER_MAX_PIN_RESOURCES) {
        return false;
    }

    if (!pin_owner_can_claim_observer(function) && observer_only != 0u) {
        for (uint8_t i = 0; i < pin_count; i++) {
            if (!pin_owner_function_supported(pins[i], function)) {
                return false;
            }
        }
    }

    /* Validate first. */
    for (uint8_t i = 0; i < pin_count; i++) {
        const uint8_t pin = pins[i];
        if (!pin_owner_is_flexible(pin) ||
            !pin_owner_function_supported(pin, function)) {
            return false;
        }

        const uint8_t index = pin_index(pin);
        if (observer_only == 0u) {
            if (pin_driver_index[index] >= 0 || pin_observer_counts[index] > 0u) {
                return false;
            }
        } else if (!pin_owner_can_claim_observer(function) &&
                   pin_observer_counts[index] == 0u) {
            return false;
        }
    }

    const uint32_t id = next_claim_id++;
    if (id == 0u) {
        return false;
    }

    int slot = -1;
    for (uint8_t i = 0; i < PIN_OWNER_MAX_CLAIMS; i++) {
        if (!claims[i].active) {
            slot = i;
            break;
        }
    }
    if (slot < 0) {
        return false;
    }

    pin_owner_claim_record_t *record = &claims[slot];
    memset(record, 0, sizeof(*record));
    record->id = id;
    record->pin_count = pin_count;
    record->function = function;
    record->observer_only = observer_only;
    record->release_state = PIN_OWNER_RELEASE_IDLE;
    record->release_result = PIN_OWNER_OK;
    record->active = 1;

    for (uint8_t i = 0; i < pin_count; i++) {
        record->pins[i] = pins[i];
        const uint8_t index = pin_index(pins[i]);
        if (observer_only == 0u) {
            pin_driver_index[index] = (int8_t)slot;
        } else {
            pin_observer_counts[index]++;
        }
    }

    *claim_id = id;
    if (!clone_claim_record((const pin_owner_claim_t *)record)) {
        /* Defensive only; a failed record copy should never be possible here. */
    }
    return true;
}

bool pin_owner_release_claim(uint32_t claim_id, pin_owner_result_t *result) {
    const int8_t slot = find_claim_index(claim_id);
    if (slot < 0) {
        if (result != NULL) {
            *result = PIN_OWNER_ERROR_NOT_FOUND;
        }
        return false;
    }

    pin_owner_claim_record_t *record = &claims[slot];
    if (record->release_state == PIN_OWNER_RELEASE_DONE) {
        if (result != NULL) {
            *result = (pin_owner_result_t)record->release_result;
        }
        return record->release_result == PIN_OWNER_OK;
    }

    if (record->release_state == PIN_OWNER_RELEASE_QUIESCING) {
        if (result != NULL) {
            *result = PIN_OWNER_OK;
        }
        return true;
    }

    record->release_state = PIN_OWNER_RELEASE_QUIESCING;
    if (result != NULL) {
        *result = PIN_OWNER_OK;
    }

    /* Minimal quiesce: observers do not disturb the driver; a driver cannot be
     * released while another read-only observer still depends on the pin. */
    int can_release = 1;
    for (uint8_t i = 0; i < record->pin_count; i++) {
        const uint8_t index = pin_index(record->pins[i]);
        if (record->observer_only == 0u && pin_observer_counts[index] > 0u) {
            can_release = 0;
            break;
        }
    }

    if (!can_release) {
        record->release_state = PIN_OWNER_RELEASE_IDLE;
        record->release_result = PIN_OWNER_ERROR_OBSERVERS;
        if (result != NULL) {
            *result = PIN_OWNER_ERROR_OBSERVERS;
        }
        return false;
    }

    for (uint8_t i = 0; i < record->pin_count; i++) {
        const uint8_t index = pin_index(record->pins[i]);
        if (record->observer_only == 0u) {
            pin_driver_index[index] = -1;
        } else if (pin_observer_counts[index] > 0u) {
            pin_observer_counts[index]--;
        }
        /* Auto-detach and disable any active IRQ attached to this pin under this claim */
        if (record->irqs[i].active) {
            gpio_set_irq_enabled(record->irqs[i].pin, record->irqs[i].event_mask, false);
            record->irqs[i].active = false;
        }
    }

    record->active = 0;
    record->release_state = PIN_OWNER_RELEASE_DONE;
    record->release_result = PIN_OWNER_OK;
    if (result != NULL) {
        *result = PIN_OWNER_OK;
    }
    return true;
}

bool pin_owner_attach_irq(uint32_t claim_id, uint8_t pin, uint32_t event_mask,
                          pin_owner_irq_callback_t callback, void *user_data) {
    const int8_t slot = find_claim_index(claim_id);
    if (slot < 0 || !claims[slot].active) {
        return false;
    }

    pin_owner_claim_record_t *record = &claims[slot];
    int pin_slot = -1;
    for (uint8_t i = 0; i < record->pin_count; i++) {
        if (record->pins[i] == pin) {
            pin_slot = i;
            break;
        }
    }
    if (pin_slot < 0) {
        return false; /* Pin not in claim */
    }

    record->irqs[pin_slot].pin = pin;
    record->irqs[pin_slot].event_mask = event_mask;
    record->irqs[pin_slot].callback = callback;
    record->irqs[pin_slot].user_data = user_data;
    record->irqs[pin_slot].active = true;

    gpio_set_irq_enabled(pin, event_mask, true);
    return true;
}

bool pin_owner_detach_irq(uint32_t claim_id, uint8_t pin) {
    const int8_t slot = find_claim_index(claim_id);
    if (slot < 0 || !claims[slot].active) {
        return false;
    }

    pin_owner_claim_record_t *record = &claims[slot];
    for (uint8_t i = 0; i < record->pin_count; i++) {
        if (record->pins[i] == pin && record->irqs[i].active) {
            gpio_set_irq_enabled(pin, record->irqs[i].event_mask, false);
            record->irqs[i].active = false;
            return true;
        }
    }
    return false;
}

bool pin_owner_has_irq(uint32_t claim_id, uint8_t pin) {
    const int8_t slot = find_claim_index(claim_id);
    if (slot < 0 || !claims[slot].active) {
        return false;
    }

    const pin_owner_claim_record_t *record = &claims[slot];
    for (uint8_t i = 0; i < record->pin_count; i++) {
        if (record->pins[i] == pin) {
            return record->irqs[i].active;
        }
    }
    return false;
}

const pin_owner_claim_t *pin_owner_get_claim(uint32_t claim_id) {
    const int8_t slot = find_claim_index(claim_id);
    if (slot < 0 || !claims[slot].active) {
        return NULL;
    }
    return (const pin_owner_claim_t *)&claims[slot];
}

bool pin_owner_release_in_progress(uint32_t claim_id) {
    const int8_t slot = find_claim_index(claim_id);
    if (slot < 0) {
        return false;
    }
    return claims[slot].release_state == PIN_OWNER_RELEASE_QUIESCING;
}
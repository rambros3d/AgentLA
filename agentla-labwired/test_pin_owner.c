#include "pin_owner.h"

#include <assert.h>
#include <stdbool.h>
#include <stdio.h>
#include <stdint.h>

static int check(const char *name, bool ok) {
    printf("%s: %s\n", name, ok ? "PASS" : "FAIL");
    return ok ? 0 : 1;
}

int main(void) {
    uint32_t handle_a = 0;
    uint32_t handle_b = 0;
    uint32_t handle_observer = 0;
    pin_owner_result_t result;
    int failures = 0;

    pin_owner_init();

    failures += check("fixed programmer pins are never claimable",
                      !pin_owner_claim_pins((uint8_t[]){3}, 1, GPIO_FUNC_SIO, 0, &handle_a));
    failures += check("analog-capable 26 through 29 report flexible",
                      pin_owner_is_flexible(26) && pin_owner_is_flexible(29));
    failures += check("fixed 0 through 9 are not flexible",
                      !pin_owner_is_flexible(0) && !pin_owner_is_flexible(9));

    uint8_t pins[] = {10, 11};
    failures += check("first driver claim succeeds",
                      pin_owner_claim_pins(pins, 2, GPIO_FUNC_I2C, 0, &handle_a));
    failures += check("same driver pin conflict fails",
                      !pin_owner_claim_pins(pins, 2, GPIO_FUNC_I2C, 0, &handle_b));
    failures += check("reported handle is nonzero", handle_a != 0);

    uint8_t observer_pin[] = {12};
    failures += check("observer request on unused SIO is invalid until driven",
                      !pin_owner_can_claim_observer(GPIO_FUNC_I2C));
    failures += check("SIO observer claim on free pin succeeds",
                      pin_owner_claim_pins(observer_pin, 1, GPIO_FUNC_SIO, 1, &handle_observer));
    failures += check("second driver claim on observed pin fails",
                      !pin_owner_claim_pins((uint8_t[]){12}, 1, GPIO_FUNC_SIO, 0, &handle_b));
    failures += check(" releasing observer succeeds",
                      pin_owner_release_claim(handle_observer, &result) && result == PIN_OWNER_OK);

    failures += check("driver may be released after observer leaves",
                      pin_owner_release_claim(handle_a, &result) && result == PIN_OWNER_OK);

    uint8_t reused[2] = {10, 11};
    uint32_t duplicate = handle_a;
    pin_owner_result_t ignored = PIN_OWNER_OK;
    (void)ignored;

    failures += check("release is idempotent after first successful release",
                      pin_owner_release_claim(duplicate, &result) &&
                      pin_owner_release_claim(duplicate, &ignored) &&
                      result == PIN_OWNER_OK &&
                      ignored == PIN_OWNER_OK);

    failures += check("claim can be re-acquired after release",
                      pin_owner_claim_pins(reused, 2, GPIO_FUNC_I2C, 0, &handle_a) && handle_a != 0);
    const pin_owner_claim_t *record_before_release = pin_owner_get_claim(handle_a);
    failures += check("claim record retains pin and function metadata",
                      record_before_release != NULL &&
                      record_before_release->pin_count == 2 &&
                      record_before_release->function == GPIO_FUNC_I2C);

    if (failures == 0) {
        printf("PASS: all %d ownership checks passed\n", 14);
    } else {
        printf("FAIL: %d ownership checks failed\n", failures);
    }
    return failures == 0 ? 0 : 1;
}

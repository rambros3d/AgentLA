/**
 * @file profile_manager.c
 * @brief Hardware-Resident Profile & Safe State Management for AgentLA
 */

#include "profile_manager.h"
#include "pin_owner.h"
#include "board_settings.h"
#include "hardware/flash.h"
#include "hardware/sync.h"
#include "pico/time.h"

#include <string.h>
#include <stdio.h>

static uint8_t active_slot_id = 0;
static char active_profile_name[PROFILE_NAME_MAX_LEN] = "Safe State (Hi-Z)";

/* Standard CRC32 implementation */
static uint32_t calculate_crc32(const uint8_t *data, size_t length) {
    uint32_t crc = 0xFFFFFFFF;
    for (size_t i = 0; i < length; i++) {
        crc ^= data[i];
        for (int j = 0; j < 8; j++) {
            if (crc & 1) {
                crc = (crc >> 1) ^ 0xEDB88320;
            } else {
                crc >>= 1;
            }
        }
    }
    return ~crc;
}

static const char SAFE_STATE_PAYLOAD[] = 
    "{\"name\":\"Safe State (Hi-Z)\",\"safe\":true,\"power\":{\"vtarget_enabled\":false,\"voltage_v\":0.0},"
    "\"pins\":[],\"instruments\":{}}";

static void populate_safe_state_slot(profile_slot_t *slot) {
    memset(slot, 0, sizeof(profile_slot_t));
    slot->magic = PROFILE_MAGIC;
    slot->version = 1;
    slot->flags = PROFILE_FLAG_IN_USE | PROFILE_FLAG_READ_ONLY;
    strncpy(slot->name, "Safe State (Hi-Z)", PROFILE_NAME_MAX_LEN - 1);
    slot->payload_len = (uint32_t)strlen(SAFE_STATE_PAYLOAD);
    slot->payload_crc32 = calculate_crc32((const uint8_t *)SAFE_STATE_PAYLOAD, slot->payload_len);
    strncpy(slot->payload, SAFE_STATE_PAYLOAD, PROFILE_MAX_PAYLOAD_LEN - 1);
}

void profile_manager_init(void) {
    /* Always guarantee immediate safe state on boot */
    profile_emergency_safe_state();

    /* Scan slots for a designated boot default profile */
    for (uint8_t i = 1; i < PROFILE_TOTAL_SLOTS; i++) {
        profile_slot_t slot;
        if (profile_get_slot(i, &slot)) {
            if ((slot.flags & PROFILE_FLAG_BOOT_DEFAULT) && (slot.flags & PROFILE_FLAG_IN_USE)) {
                char err[64];
                profile_apply_slot(i, err, sizeof(err));
                break;
            }
        }
    }
}

bool profile_manager_is_safe_state_active(void) {
    return active_slot_id == 0;
}

uint32_t profile_manager_get_active_slot(void) {
    return active_slot_id;
}

const char* profile_manager_get_active_name(void) {
    return active_profile_name;
}

bool profile_get_slot(uint8_t slot_id, profile_slot_t *out_slot) {
    if (slot_id >= PROFILE_TOTAL_SLOTS || out_slot == NULL) {
        return false;
    }

    if (slot_id == 0) {
        populate_safe_state_slot(out_slot);
        return true;
    }

    const profile_slot_t *flash_slot = (const profile_slot_t *)(PROFILE_FLASH_ADDR + ((uint32_t)slot_id * PROFILE_SLOT_SIZE));
    if (flash_slot->magic != PROFILE_MAGIC) {
        return false;
    }

    if (flash_slot->payload_len > PROFILE_MAX_PAYLOAD_LEN) {
        return false;
    }

    uint32_t calc_crc = calculate_crc32((const uint8_t *)flash_slot->payload, flash_slot->payload_len);
    if (calc_crc != flash_slot->payload_crc32) {
        return false;
    }

    memcpy(out_slot, flash_slot, sizeof(profile_slot_t));
    return true;
}

bool profile_save_slot(uint8_t slot_id, const char *name, bool is_boot_default, const char *json_payload) {
    if (slot_id == 0 || slot_id >= PROFILE_TOTAL_SLOTS || name == NULL || json_payload == NULL) {
        return false;
    }

    size_t payload_len = strlen(json_payload);
    if (payload_len >= PROFILE_MAX_PAYLOAD_LEN) {
        return false;
    }

    /* Sector math: FLASH_SECTOR_SIZE = 4096 bytes (holds 2 profile slots of 2048 bytes each) */
    uint32_t sector_slot_base = slot_id & ~1u; /* 0, 2, 4, ... */
    uint32_t sector_offset = PROFILE_FLASH_OFFSET + (sector_slot_base * PROFILE_SLOT_SIZE);
    uint8_t sector_buf[FLASH_SECTOR_SIZE];

    /* Copy current 4KB sector into RAM buffer */
    memcpy(sector_buf, (const void *)(XIP_BASE + sector_offset), FLASH_SECTOR_SIZE);

    /* Locate target slot within buffer */
    uint32_t offset_in_sector = (slot_id & 1u) * PROFILE_SLOT_SIZE;
    profile_slot_t *target = (profile_slot_t *)&sector_buf[offset_in_sector];

    memset(target, 0, PROFILE_SLOT_SIZE);
    target->magic = PROFILE_MAGIC;
    target->version = 1;
    target->flags = PROFILE_FLAG_IN_USE;
    if (is_boot_default) {
        target->flags |= PROFILE_FLAG_BOOT_DEFAULT;
    }
    strncpy(target->name, name, PROFILE_NAME_MAX_LEN - 1);
    target->payload_len = (uint32_t)payload_len;
    target->payload_crc32 = calculate_crc32((const uint8_t *)json_payload, payload_len);
    memcpy(target->payload, json_payload, payload_len);
    target->payload[payload_len] = '\0';

    /* Flash programming with interrupts disabled */
    uint32_t ints = save_and_disable_interrupts();
    flash_range_erase(sector_offset, FLASH_SECTOR_SIZE);
    flash_range_program(sector_offset, sector_buf, FLASH_SECTOR_SIZE);
    restore_interrupts(ints);

    /* If this slot was marked as boot default, clear boot flag on any other slots */
    if (is_boot_default) {
        for (uint8_t i = 1; i < PROFILE_TOTAL_SLOTS; i++) {
            if (i != slot_id) {
                profile_slot_t other;
                if (profile_get_slot(i, &other) && (other.flags & PROFILE_FLAG_BOOT_DEFAULT)) {
                    profile_save_slot(i, other.name, false, other.payload);
                }
            }
        }
    }

    return true;
}

bool profile_delete_slot(uint8_t slot_id) {
    if (slot_id == 0 || slot_id >= PROFILE_TOTAL_SLOTS) {
        return false;
    }

    uint32_t sector_slot_base = slot_id & ~1u;
    uint32_t sector_offset = PROFILE_FLASH_OFFSET + (sector_slot_base * PROFILE_SLOT_SIZE);
    uint8_t sector_buf[FLASH_SECTOR_SIZE];

    memcpy(sector_buf, (const void *)(XIP_BASE + sector_offset), FLASH_SECTOR_SIZE);
    uint32_t offset_in_sector = (slot_id & 1u) * PROFILE_SLOT_SIZE;
    memset(&sector_buf[offset_in_sector], 0xFF, PROFILE_SLOT_SIZE);

    uint32_t ints = save_and_disable_interrupts();
    flash_range_erase(sector_offset, FLASH_SECTOR_SIZE);
    flash_range_program(sector_offset, sector_buf, FLASH_SECTOR_SIZE);
    restore_interrupts(ints);

    if (active_slot_id == slot_id) {
        profile_emergency_safe_state();
    }

    return true;
}

bool profile_set_boot_default(uint8_t slot_id) {
    if (slot_id == 0) {
        /* Clear boot default across all slots so system boots into Safe State */
        for (uint8_t i = 1; i < PROFILE_TOTAL_SLOTS; i++) {
            profile_slot_t slot;
            if (profile_get_slot(i, &slot) && (slot.flags & PROFILE_FLAG_BOOT_DEFAULT)) {
                profile_save_slot(i, slot.name, false, slot.payload);
            }
        }
        return true;
    }

    profile_slot_t slot;
    if (!profile_get_slot(slot_id, &slot)) {
        return false;
    }

    return profile_save_slot(slot_id, slot.name, true, slot.payload);
}

void profile_emergency_safe_state(void) {
    /* 1. Atomically reset pin ownership and drive all GPIOs to High-Z inputs */
    pin_owner_quiesce_and_reset();

    /* 2. De-energize target power (Safe 0V) */
    /* Target power control pin on BakuProbe / AgentLA (if configured) */
#ifdef TARGET_POWER_PIN
    gpio_init(TARGET_POWER_PIN);
    gpio_set_dir(TARGET_POWER_PIN, GPIO_OUT);
    gpio_put(TARGET_POWER_PIN, 0);
#endif

    active_slot_id = 0;
    strncpy(active_profile_name, "Safe State (Hi-Z)", sizeof(active_profile_name) - 1);
}

bool profile_apply_slot(uint8_t slot_id, char *err_msg, size_t err_capacity) {
    if (slot_id == 0) {
        profile_emergency_safe_state();
        if (err_msg && err_capacity > 0) err_msg[0] = '\0';
        return true;
    }

    profile_slot_t slot;
    if (!profile_get_slot(slot_id, &slot)) {
        if (err_msg && err_capacity > 0) {
            snprintf(err_msg, err_capacity, "Profile slot %u not found or corrupted", slot_id);
        }
        return false;
    }

    /* 1. Enter Safe Intermediate Tri-State first */
    profile_emergency_safe_state();
    sleep_ms(5); /* Discharge rails and settle bus signals */

    /* 2. Transition active state to new profile */
    active_slot_id = slot_id;
    strncpy(active_profile_name, slot.name, sizeof(active_profile_name) - 1);

    if (err_msg && err_capacity > 0) {
        err_msg[0] = '\0';
    }
    return true;
}

void profile_list_to_json(char *buf, size_t capacity) {
    if (buf == NULL || capacity == 0) return;

    size_t offset = snprintf(buf, capacity, "{\"active_slot\":%u,\"active_name\":\"%s\",\"slots\":[",
                             active_slot_id, active_profile_name);

    bool first = true;
    for (uint8_t i = 0; i < PROFILE_TOTAL_SLOTS; i++) {
        profile_slot_t slot;
        if (profile_get_slot(i, &slot) && (slot.flags & PROFILE_FLAG_IN_USE)) {
            if (!first && offset < capacity) {
                offset += snprintf(buf + offset, capacity - offset, ",");
            }
            first = false;

            bool is_boot = (slot.flags & PROFILE_FLAG_BOOT_DEFAULT) != 0;
            bool is_ro = (slot.flags & PROFILE_FLAG_READ_ONLY) != 0;

            if (offset < capacity) {
                offset += snprintf(buf + offset, capacity - offset,
                                   "{\"id\":%u,\"name\":\"%s\",\"is_boot\":%s,\"read_only\":%s}",
                                   i, slot.name, is_boot ? "true" : "false", is_ro ? "true" : "false");
            }
        }
    }

    if (offset < capacity) {
        snprintf(buf + offset, capacity - offset, "]}");
    }
}

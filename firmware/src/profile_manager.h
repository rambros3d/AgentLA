#ifndef __AGENTLA_PROFILE_MANAGER_H__
#define __AGENTLA_PROFILE_MANAGER_H__

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
#include "pico/stdlib.h"

#define PROFILE_FLASH_OFFSET     (1536u * 1024u)  /* 1.5 MB offset in flash */
#define PROFILE_FLASH_ADDR       (XIP_BASE + PROFILE_FLASH_OFFSET)
#define PROFILE_TOTAL_SLOTS      32
#define PROFILE_SLOT_SIZE        2048u            /* 2 KB per slot = 64 KB total */
#define PROFILE_MAGIC            0x41474C41       /* 'AGLA' */
#define PROFILE_NAME_MAX_LEN     32
#define PROFILE_MAX_PAYLOAD_LEN  (PROFILE_SLOT_SIZE - 48u) /* 2000 bytes */

#define PROFILE_FLAG_IN_USE       (1u << 0)
#define PROFILE_FLAG_BOOT_DEFAULT (1u << 1)
#define PROFILE_FLAG_READ_ONLY    (1u << 2)

typedef struct __attribute__((packed)) {
    uint32_t magic;
    uint16_t version;
    uint16_t flags;
    char name[PROFILE_NAME_MAX_LEN];
    uint32_t payload_crc32;
    uint32_t payload_len;
    char payload[PROFILE_MAX_PAYLOAD_LEN];
} profile_slot_t;

void profile_manager_init(void);
bool profile_manager_is_safe_state_active(void);
uint32_t profile_manager_get_active_slot(void);
const char* profile_manager_get_active_name(void);

bool profile_get_slot(uint8_t slot_id, profile_slot_t *out_slot);
bool profile_save_slot(uint8_t slot_id, const char *name, bool is_boot_default, const char *json_payload);
bool profile_delete_slot(uint8_t slot_id);
bool profile_set_boot_default(uint8_t slot_id);
bool profile_apply_slot(uint8_t slot_id, char *err_msg, size_t err_capacity);
void profile_emergency_safe_state(void);

void profile_list_to_json(char *buf, size_t capacity);

#endif /* __AGENTLA_PROFILE_MANAGER_H__ */

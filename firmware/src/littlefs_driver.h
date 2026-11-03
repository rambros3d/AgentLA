#ifndef __AGENTLA_LITTLEFS_DRIVER_H__
#define __AGENTLA_LITTLEFS_DRIVER_H__

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
#include "littlefs/lfs.h"

#define LITTLEFS_START_OFFSET     (0x00190000UL) /* 1.5625 MB offset (after profile slots) */
#define LITTLEFS_RESERVED_END_KB  (16UL)         /* 16 KB reserved at end of flash */

typedef struct {
    uint32_t total_bytes;
    uint32_t used_bytes;
    uint32_t free_bytes;
    uint32_t block_count;
    uint32_t block_size;
} littlefs_storage_info_t;

/* Initialize and mount LittleFS partition; formats if blank */
bool littlefs_init(void);

/* Check if LittleFS is mounted */
bool littlefs_is_mounted(void);

/* Get LittleFS instance pointer */
lfs_t* littlefs_get_instance(void);

/* Query partition storage statistics */
bool littlefs_get_storage_info(littlefs_storage_info_t *info);

/* Recursively remove directory and its contents */
int littlefs_remove_recursive(const char *path);

/* Format LittleFS JSON response for /api/v1/apps/storage */
size_t littlefs_storage_to_json(char *buf, size_t capacity);

/* List installed apps under /apps/ to JSON */
size_t littlefs_list_apps_to_json(char *buf, size_t capacity);

#endif /* __AGENTLA_LITTLEFS_DRIVER_H__ */

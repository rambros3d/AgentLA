/**
 * @file littlefs_driver.c
 * @brief LittleFS Partition Driver & Flash Block Management for AgentLA
 */

#include "littlefs_driver.h"
#include "hardware/flash.h"
#include "hardware/sync.h"
#include <string.h>
#include <stdio.h>

#if __has_include("generated/board_config.h")
#include "generated/board_config.h"
#endif

#ifndef BOARD_MCU_FLASH_BYTES
#define BOARD_MCU_FLASH_BYTES (4194304UL)
#endif

#define LFS_BLOCK_SIZE     (FLASH_SECTOR_SIZE)  /* 4096 bytes */
#define LFS_PROG_SIZE      (FLASH_PAGE_SIZE)    /* 256 bytes */
#define LFS_READ_SIZE      (FLASH_PAGE_SIZE)    /* 256 bytes */
#define LFS_CACHE_SIZE     (256)
#define LFS_LOOKAHEAD_SIZE (64)

static lfs_t lfs_instance;
static struct lfs_config lfs_cfg;
static bool lfs_is_mounted_flag = false;

static uint8_t lfs_read_buffer[LFS_CACHE_SIZE];
static uint8_t lfs_prog_buffer[LFS_CACHE_SIZE];
static uint8_t lfs_lookahead_buffer[LFS_LOOKAHEAD_SIZE];

static uint32_t lfs_flash_start_offset = LITTLEFS_START_OFFSET;
static uint32_t lfs_partition_size_bytes = 0;
static uint32_t lfs_total_blocks = 0;

static int littlefs_block_read(const struct lfs_config *c, lfs_block_t block,
                              lfs_off_t off, void *buffer, lfs_size_t size) {
    (void)c;
    uint32_t flash_addr = XIP_BASE + lfs_flash_start_offset + (block * LFS_BLOCK_SIZE) + off;
    memcpy(buffer, (const void *)flash_addr, size);
    return LFS_ERR_OK;
}

static int littlefs_block_prog(const struct lfs_config *c, lfs_block_t block,
                              lfs_off_t off, const void *buffer, lfs_size_t size) {
    (void)c;
    uint32_t flash_offset = lfs_flash_start_offset + (block * LFS_BLOCK_SIZE) + off;
    uint32_t ints = save_and_disable_interrupts();
    flash_range_program(flash_offset, (const uint8_t *)buffer, size);
    restore_interrupts(ints);
    return LFS_ERR_OK;
}

static int littlefs_block_erase(const struct lfs_config *c, lfs_block_t block) {
    (void)c;
    uint32_t flash_offset = lfs_flash_start_offset + (block * LFS_BLOCK_SIZE);
    uint32_t ints = save_and_disable_interrupts();
    flash_range_erase(flash_offset, LFS_BLOCK_SIZE);
    restore_interrupts(ints);
    return LFS_ERR_OK;
}

static int littlefs_block_sync(const struct lfs_config *c) {
    (void)c;
    return LFS_ERR_OK;
}

static void calculate_geometry(void) {
    uint32_t flash_total = BOARD_MCU_FLASH_BYTES;
    uint32_t flash_end = flash_total - (LITTLEFS_RESERVED_END_KB * 1024UL);
    lfs_flash_start_offset = LITTLEFS_START_OFFSET;

    if (flash_end > lfs_flash_start_offset) {
        lfs_partition_size_bytes = flash_end - lfs_flash_start_offset;
        lfs_total_blocks = lfs_partition_size_bytes / LFS_BLOCK_SIZE;
    } else {
        lfs_partition_size_bytes = 0;
        lfs_total_blocks = 0;
    }
}

bool littlefs_init(void) {
    if (lfs_is_mounted_flag) {
        return true;
    }

    calculate_geometry();
    if (lfs_total_blocks == 0) {
        return false;
    }

    memset(&lfs_cfg, 0, sizeof(lfs_cfg));
    lfs_cfg.read  = littlefs_block_read;
    lfs_cfg.prog  = littlefs_block_prog;
    lfs_cfg.erase = littlefs_block_erase;
    lfs_cfg.sync  = littlefs_block_sync;

    lfs_cfg.read_size      = LFS_READ_SIZE;
    lfs_cfg.prog_size      = LFS_PROG_SIZE;
    lfs_cfg.block_size     = LFS_BLOCK_SIZE;
    lfs_cfg.block_count    = lfs_total_blocks;
    lfs_cfg.block_cycles   = 500;
    lfs_cfg.cache_size     = LFS_CACHE_SIZE;
    lfs_cfg.lookahead_size = LFS_LOOKAHEAD_SIZE;

    lfs_cfg.read_buffer      = lfs_read_buffer;
    lfs_cfg.prog_buffer      = lfs_prog_buffer;
    lfs_cfg.lookahead_buffer = lfs_lookahead_buffer;

    int err = lfs_mount(&lfs_instance, &lfs_cfg);
    if (err) {
        /* Format LittleFS partition if blank or corrupt */
        err = lfs_format(&lfs_instance, &lfs_cfg);
        if (err) {
            return false;
        }
        err = lfs_mount(&lfs_instance, &lfs_cfg);
        if (err) {
            return false;
        }
    }

    lfs_is_mounted_flag = true;

    /* Ensure /apps directory exists */
    lfs_mkdir(&lfs_instance, "/apps");

    return true;
}

bool littlefs_is_mounted(void) {
    return lfs_is_mounted_flag;
}

lfs_t* littlefs_get_instance(void) {
    if (!lfs_is_mounted_flag) {
        littlefs_init();
    }
    return &lfs_instance;
}

static int count_used_blocks_cb(void *p, lfs_block_t block) {
    (void)block;
    uint32_t *count = (uint32_t *)p;
    (*count)++;
    return 0;
}

bool littlefs_get_storage_info(littlefs_storage_info_t *info) {
    if (!info) return false;
    if (!littlefs_is_mounted()) {
        if (!littlefs_init()) return false;
    }

    uint32_t used_blocks = 0;
    int err = lfs_fs_traverse(&lfs_instance, count_used_blocks_cb, &used_blocks);
    if (err) {
        used_blocks = 0;
    }

    info->block_size  = LFS_BLOCK_SIZE;
    info->block_count = lfs_total_blocks;
    info->total_bytes = lfs_total_blocks * LFS_BLOCK_SIZE;
    info->used_bytes  = used_blocks * LFS_BLOCK_SIZE;
    if (info->used_bytes > info->total_bytes) {
        info->used_bytes = info->total_bytes;
    }
    info->free_bytes  = info->total_bytes - info->used_bytes;
    return true;
}

static int remove_recursive_helper(lfs_t *lfs, const char *path) {
    lfs_dir_t dir;
    int res = lfs_dir_open(lfs, &dir, path);
    if (res < 0) {
        /* If it's a file, remove it directly */
        return lfs_remove(lfs, path);
    }

    struct lfs_info info;
    char subpath[256];
    while ((res = lfs_dir_read(lfs, &dir, &info)) > 0) {
        if (strcmp(info.name, ".") == 0 || strcmp(info.name, "..") == 0) {
            continue;
        }
        snprintf(subpath, sizeof(subpath), "%s/%s", path, info.name);
        if (info.type == LFS_TYPE_DIR) {
            remove_recursive_helper(lfs, subpath);
        } else {
            lfs_remove(lfs, subpath);
        }
    }
    lfs_dir_close(lfs, &dir);
    return lfs_remove(lfs, path);
}

int littlefs_remove_recursive(const char *path) {
    if (!path || !littlefs_is_mounted()) return -1;
    return remove_recursive_helper(&lfs_instance, path);
}

size_t littlefs_storage_to_json(char *buf, size_t capacity) {
    if (!buf || capacity == 0) return 0;
    littlefs_storage_info_t info;
    if (!littlefs_get_storage_info(&info)) {
        return (size_t)snprintf(buf, capacity, "{\"error\":\"Storage query failed\"}");
    }
    return (size_t)snprintf(buf, capacity,
        "{\"total_bytes\":%lu,\"used_bytes\":%lu,\"free_bytes\":%lu,\"block_size\":%lu,\"block_count\":%lu}",
        (unsigned long)info.total_bytes,
        (unsigned long)info.used_bytes,
        (unsigned long)info.free_bytes,
        (unsigned long)info.block_size,
        (unsigned long)info.block_count);
}

static uint32_t calc_dir_size_helper(lfs_t *lfs, const char *path) {
    lfs_dir_t dir;
    if (lfs_dir_open(lfs, &dir, path) < 0) return 0;
    struct lfs_info info;
    char subpath[256];
    uint32_t total = 0;
    while (lfs_dir_read(lfs, &dir, &info) > 0) {
        if (strcmp(info.name, ".") == 0 || strcmp(info.name, "..") == 0) continue;
        snprintf(subpath, sizeof(subpath), "%s/%s", path, info.name);
        if (info.type == LFS_TYPE_DIR) {
            total += calc_dir_size_helper(lfs, subpath);
        } else {
            total += (uint32_t)info.size;
        }
    }
    lfs_dir_close(lfs, &dir);
    return total;
}

size_t littlefs_list_apps_to_json(char *buf, size_t capacity) {
    if (!buf || capacity == 0) return 0;
    if (!littlefs_is_mounted()) {
        if (!littlefs_init()) {
            return (size_t)snprintf(buf, capacity, "[]");
        }
    }

    lfs_dir_t dir;
    if (lfs_dir_open(&lfs_instance, &dir, "/apps") < 0) {
        return (size_t)snprintf(buf, capacity, "[]");
    }

    size_t offset = 0;
    offset += (size_t)snprintf(buf + offset, capacity - offset, "[");

    struct lfs_info info;
    bool first = true;
    char app_dir_path[256];
    char manifest_path[256];

    while (lfs_dir_read(&lfs_instance, &dir, &info) > 0) {
        if (strcmp(info.name, ".") == 0 || strcmp(info.name, "..") == 0) continue;
        if (info.type != LFS_TYPE_DIR) continue;

        snprintf(app_dir_path, sizeof(app_dir_path), "/apps/%s", info.name);
        snprintf(manifest_path, sizeof(manifest_path), "/apps/%s/app.json", info.name);

        uint32_t app_size = calc_dir_size_helper(&lfs_instance, app_dir_path);

        /* Try reading app.json */
        lfs_file_t mf;
        char manifest_content[512] = "";
        bool has_manifest = false;
        if (lfs_file_open(&lfs_instance, &mf, manifest_path, LFS_O_RDONLY) >= 0) {
            lfs_ssize_t read_bytes = lfs_file_read(&lfs_instance, &mf, manifest_content, sizeof(manifest_content) - 1);
            if (read_bytes > 0) {
                manifest_content[read_bytes] = '\0';
                has_manifest = true;
            }
            lfs_file_close(&lfs_instance, &mf);
        }

        if (!first) {
            if (offset < capacity - 1) offset += (size_t)snprintf(buf + offset, capacity - offset, ",");
        }
        first = false;

        if (has_manifest && manifest_content[0] == '{') {
            /* Manifest is valid JSON object; splice in size and path */
            /* Strip trailing '}' */
            char *last_brace = strrchr(manifest_content, '}');
            if (last_brace) *last_brace = '\0';
            if (offset < capacity - 1) {
                offset += (size_t)snprintf(buf + offset, capacity - offset,
                    "%s,\"size\":%lu,\"path\":\"/apps/%s/\"}",
                    manifest_content, (unsigned long)app_size, info.name);
            }
        } else {
            /* Fallback generic app entry */
            if (offset < capacity - 1) {
                offset += (size_t)snprintf(buf + offset, capacity - offset,
                    "{\"id\":\"%s\",\"name\":\"%s\",\"version\":\"1.0.0\",\"description\":\"Installed Application\",\"size\":%lu,\"path\":\"/apps/%s/\"}",
                    info.name, info.name, (unsigned long)app_size, info.name);
            }
        }
    }

    lfs_dir_close(&lfs_instance, &dir);
    if (offset < capacity - 1) {
        offset += (size_t)snprintf(buf + offset, capacity - offset, "]");
    }
    return offset;
}

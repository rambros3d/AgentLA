/**
 * @file tar_unpacker.c
 * @brief Streaming 512-byte POSIX TAR unpacker directly into LittleFS
 */

#include "tar_unpacker.h"
#include "littlefs_driver.h"
#include <string.h>
#include <stdio.h>
#include <stdlib.h>

static void ensure_parent_dirs(lfs_t *lfs, const char *full_path) {
    char tmp[256];
    size_t len = strlen(full_path);
    if (len >= sizeof(tmp)) return;
    memcpy(tmp, full_path, len + 1);

    for (size_t i = 1; i < len; i++) {
        if (tmp[i] == '/') {
            tmp[i] = '\0';
            lfs_mkdir(lfs, tmp);
            tmp[i] = '/';
        }
    }
}

bool tar_unpacker_init(tar_unpacker_t *u, const char *app_name) {
    if (!u || !app_name || app_name[0] == '\0') return false;

    memset(u, 0, sizeof(tar_unpacker_t));
    snprintf(u->app_dir, sizeof(u->app_dir), "/apps/%s", app_name);
    u->state = TAR_STATE_HEADER;

    /* Ensure app target directory exists */
    lfs_t *lfs = littlefs_get_instance();
    if (!lfs) return false;
    lfs_mkdir(lfs, "/apps");
    lfs_mkdir(lfs, u->app_dir);

    return true;
}

size_t tar_unpacker_feed(tar_unpacker_t *u, const uint8_t *data, size_t len) {
    if (!u || !data || len == 0) return 0;
    lfs_t *lfs = littlefs_get_instance();
    if (!lfs) return 0;

    size_t total_processed = 0;

    while (len > 0) {
        switch (u->state) {
            case TAR_STATE_HEADER: {
                size_t needed = 512 - u->hdr_bytes_have;
                size_t take = (len < needed) ? len : needed;
                memcpy(u->hdr_buf + u->hdr_bytes_have, data, take);
                u->hdr_bytes_have += take;
                data += take;
                len -= take;
                total_processed += take;

                if (u->hdr_bytes_have == 512) {
                    /* Check for two 512-byte zero blocks (end of archive) */
                    bool all_zero = true;
                    for (int i = 0; i < 512; i++) {
                        if (u->hdr_buf[i] != 0) {
                            all_zero = false;
                            break;
                        }
                    }
                    if (all_zero) {
                        u->state = TAR_STATE_DONE;
                        break;
                    }

                    /* Extract filename */
                    char raw_name[101];
                    memcpy(raw_name, u->hdr_buf, 100);
                    raw_name[100] = '\0';

                    /* Parse octal size */
                    char size_str[13];
                    memcpy(size_str, u->hdr_buf + 124, 12);
                    size_str[12] = '\0';
                    u->file_size = (uint32_t)strtoul(size_str, NULL, 8);
                    u->file_bytes_written = 0;

                    char typeflag = (char)u->hdr_buf[156];

                    /* Normalize filename */
                    const char *norm = raw_name;
                    while (*norm == '.' || *norm == '/') norm++;

                    /* Check if first component matches app folder name */
                    const char *first_slash = strchr(norm, '/');
                    if (first_slash != NULL) {
                        /* Check app directory name component */
                        const char *app_basename = strrchr(u->app_dir, '/');
                        app_basename = app_basename ? app_basename + 1 : u->app_dir;
                        size_t seg_len = first_slash - norm;
                        if (strlen(app_basename) == seg_len && strncmp(norm, app_basename, seg_len) == 0) {
                            norm = first_slash + 1;
                        }
                    }

                    if (*norm == '\0') {
                        /* Skip empty entry */
                        u->hdr_bytes_have = 0;
                        u->state = TAR_STATE_HEADER;
                        break;
                    }

                    char full_path[256];
                    snprintf(full_path, sizeof(full_path), "%s/%s", u->app_dir, norm);

                    if (typeflag == '5' || raw_name[strlen(raw_name) - 1] == '/') {
                        /* Directory entry */
                        ensure_parent_dirs(lfs, full_path);
                        lfs_mkdir(lfs, full_path);
                        u->hdr_bytes_have = 0;
                        u->state = TAR_STATE_HEADER;
                    } else {
                        /* Regular file */
                        ensure_parent_dirs(lfs, full_path);
                        int res = lfs_file_open(lfs, &u->current_file, full_path,
                                               LFS_O_WRONLY | LFS_O_CREAT | LFS_O_TRUNC);
                        if (res >= 0) {
                            u->file_is_open = true;
                        }
                        u->total_files++;
                        u->hdr_bytes_have = 0;

                        if (u->file_size > 0) {
                            u->state = TAR_STATE_DATA;
                        } else {
                            if (u->file_is_open) {
                                lfs_file_close(lfs, &u->current_file);
                                u->file_is_open = false;
                            }
                            u->state = TAR_STATE_HEADER;
                        }
                    }
                }
                break;
            }

            case TAR_STATE_DATA: {
                uint32_t left = u->file_size - u->file_bytes_written;
                size_t take = (len < left) ? len : left;
                if (take > 0) {
                    if (u->file_is_open) {
                        lfs_file_write(lfs, &u->current_file, data, take);
                    }
                    u->file_bytes_written += take;
                    u->total_bytes += take;
                    data += take;
                    len -= take;
                    total_processed += take;
                }

                if (u->file_bytes_written >= u->file_size) {
                    if (u->file_is_open) {
                        lfs_file_close(lfs, &u->current_file);
                        u->file_is_open = false;
                    }
                    uint32_t rem = u->file_size % 512;
                    u->padding_needed = rem ? (512 - rem) : 0;
                    u->padding_discarded = 0;
                    if (u->padding_needed > 0) {
                        u->state = TAR_STATE_PADDING;
                    } else {
                        u->state = TAR_STATE_HEADER;
                    }
                }
                break;
            }

            case TAR_STATE_PADDING: {
                uint32_t left_pad = u->padding_needed - u->padding_discarded;
                size_t take = (len < left_pad) ? len : left_pad;
                u->padding_discarded += take;
                data += take;
                len -= take;
                total_processed += take;

                if (u->padding_discarded >= u->padding_needed) {
                    u->state = TAR_STATE_HEADER;
                }
                break;
            }

            case TAR_STATE_DONE:
            case TAR_STATE_ERROR:
                /* Consume any remaining bytes without processing */
                total_processed += len;
                len = 0;
                break;
        }
    }

    return total_processed;
}

void tar_unpacker_finish(tar_unpacker_t *u) {
    if (!u) return;
    if (u->file_is_open) {
        lfs_t *lfs = littlefs_get_instance();
        if (lfs) {
            lfs_file_close(lfs, &u->current_file);
        }
        u->file_is_open = false;
    }
}

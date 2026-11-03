#ifndef __AGENTLA_TAR_UNPACKER_H__
#define __AGENTLA_TAR_UNPACKER_H__

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
#include "littlefs/lfs.h"

typedef enum {
    TAR_STATE_HEADER,
    TAR_STATE_DATA,
    TAR_STATE_PADDING,
    TAR_STATE_DONE,
    TAR_STATE_ERROR
} tar_state_t;

typedef struct {
    tar_state_t state;
    char app_dir[128];
    uint8_t hdr_buf[512];
    size_t hdr_bytes_have;

    char current_relpath[128];
    uint32_t file_size;
    uint32_t file_bytes_written;
    uint32_t padding_needed;
    uint32_t padding_discarded;

    lfs_file_t current_file;
    bool file_is_open;
    uint32_t total_files;
    uint32_t total_bytes;
} tar_unpacker_t;

/* Initialize streaming unpacker targeting /apps/{app_name}/ */
bool tar_unpacker_init(tar_unpacker_t *u, const char *app_name);

/* Feed raw chunks of TAR archive into unpacker */
size_t tar_unpacker_feed(tar_unpacker_t *u, const uint8_t *data, size_t len);

/* Finalize unpacker and close any open files */
void tar_unpacker_finish(tar_unpacker_t *u);

#endif /* __AGENTLA_TAR_UNPACKER_H__ */

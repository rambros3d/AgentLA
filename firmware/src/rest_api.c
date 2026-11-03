/**
 * @file rest_api.c
 * @brief Dedicated REST API (/api/v1/...) Engine for AgentLA
 */

#include "rest_api.h"
#include "littlefs_driver.h"
#include "profile_manager.h"
#include "pico/time.h"
#include <string.h>
#include <stdio.h>

#if __has_include("generated/board_config.h")
#include "generated/board_config.h"
#endif

#ifndef AGENTLA_BOARD_ID
#define AGENTLA_BOARD_ID "pico2"
#endif

#ifndef AGENTLA_BOARD_NAME
#define AGENTLA_BOARD_NAME "Raspberry Pi Pico 2"
#endif

#ifndef FIRMWARE_VERSION
#define FIRMWARE_VERSION "v1.0"
#endif

#ifndef BOARD_MCU_FLASH_BYTES
#define BOARD_MCU_FLASH_BYTES (4194304UL)
#endif

#define REST_BUFFER_SIZE 1536
#define REST_HEADER_RESERVE 160
static char rest_response_buffer[REST_BUFFER_SIZE];
static char rest_last_installed_app[64] = "app";

void rest_api_set_last_installed_app(const char *name) {
    if (name) {
        snprintf(rest_last_installed_app, sizeof(rest_last_installed_app), "%s", name);
    }
}

static int build_http_response(struct fs_file *file, int status_code, const char *status_text,
                              const char *content_type, const char *body, size_t body_len) {
    char header_tmp[REST_HEADER_RESERVE];
    int header_len = snprintf(header_tmp, sizeof(header_tmp),
        "HTTP/1.1 %d %s\r\n"
        "Content-Type: %s\r\n"
        "Access-Control-Allow-Origin: *\r\n"
        "Access-Control-Allow-Methods: GET, POST, DELETE, OPTIONS\r\n"
        "Access-Control-Allow-Headers: Content-Type\r\n"
        "Content-Length: %zu\r\n"
        "Connection: close\r\n\r\n",
        status_code, status_text, content_type, body_len);

    if (header_len < 0 || (size_t)header_len >= sizeof(header_tmp)) {
        return 0;
    }

    if ((size_t)header_len + body_len >= sizeof(rest_response_buffer)) {
        body_len = sizeof(rest_response_buffer) - (size_t)header_len - 1;
    }

    /* If body is already inside rest_response_buffer at offset REST_HEADER_RESERVE, memmove */
    if (body >= rest_response_buffer && body < rest_response_buffer + sizeof(rest_response_buffer)) {
        memmove(rest_response_buffer + header_len, body, body_len);
    } else if (body && body_len > 0) {
        memcpy(rest_response_buffer + header_len, body, body_len);
    }

    memcpy(rest_response_buffer, header_tmp, header_len);
    size_t total_len = (size_t)header_len + body_len;
    rest_response_buffer[total_len] = '\0';

    file->data = rest_response_buffer;
    file->len = (int)total_len;
    file->index = file->len;
    file->flags = FS_FILE_FLAGS_CUSTOM | FS_FILE_FLAGS_HEADER_INCLUDED;
    return 1;
}

bool rest_api_matches(const char *uri) {
    if (!uri) return false;
    return (strncmp(uri, "/api/v1/", 8) == 0);
}

int rest_api_handle(struct fs_file *file, const char *uri) {
    if (!file || !uri) return 0;

    const char *path = uri + 8; /* strip "/api/v1/" */
    char *json_scratch = rest_response_buffer + REST_HEADER_RESERVE;
    size_t json_capacity = sizeof(rest_response_buffer) - REST_HEADER_RESERVE - 1;

    /* 0. POST /api/v1/apps/install_done */
    if (strcmp(path, "apps/install_done") == 0) {
        int len = snprintf(json_scratch, json_capacity,
            "{\"status\":\"installed\",\"app\":\"%s\"}", rest_last_installed_app);
        return build_http_response(file, 201, "Created", "application/json", json_scratch, (size_t)len);
    }

    /* 1. GET /api/v1/apps */
    if (strcmp(path, "apps") == 0 || strcmp(path, "apps/") == 0) {
        size_t json_len = littlefs_list_apps_to_json(json_scratch, json_capacity);
        return build_http_response(file, 200, "OK", "application/json", json_scratch, json_len);
    }

    /* 2. GET /api/v1/apps/storage */
    if (strcmp(path, "apps/storage") == 0 || strcmp(path, "apps/storage/") == 0) {
        size_t json_len = littlefs_storage_to_json(json_scratch, json_capacity);
        return build_http_response(file, 200, "OK", "application/json", json_scratch, json_len);
    }

    /* 3. GET /api/v1/apps/{name}/manifest */
    if (strncmp(path, "apps/", 5) == 0) {
        const char *sub = path + 5;
        const char *slash = strchr(sub, '/');
        if (slash != NULL && strcmp(slash, "/manifest") == 0) {
            char app_name[64];
            size_t name_len = slash - sub;
            if (name_len < sizeof(app_name)) {
                memcpy(app_name, sub, name_len);
                app_name[name_len] = '\0';

                char manifest_path[128];
                snprintf(manifest_path, sizeof(manifest_path), "/apps/%s/app.json", app_name);

                lfs_t *lfs = littlefs_get_instance();
                lfs_file_t mf;
                if (lfs && lfs_file_open(lfs, &mf, manifest_path, LFS_O_RDONLY) >= 0) {
                    lfs_ssize_t read_bytes = lfs_file_read(lfs, &mf, json_scratch, json_capacity - 1);
                    lfs_file_close(lfs, &mf);
                    if (read_bytes >= 0) {
                        json_scratch[read_bytes] = '\0';
                        return build_http_response(file, 200, "OK", "application/json", json_scratch, (size_t)read_bytes);
                    }
                }
            }
            const char *not_found = "{\"error\":\"Manifest not found\"}";
            return build_http_response(file, 404, "Not Found", "application/json", not_found, strlen(not_found));
        }

        /* 3b. DELETE /api/v1/apps/{name} (called via POST or custom redirect) */
        char app_name[64];
        const char *end = strchr(sub, '?');
        size_t name_len = end ? (size_t)(end - sub) : strlen(sub);
        if (name_len > 0 && name_len < sizeof(app_name)) {
            memcpy(app_name, sub, name_len);
            app_name[name_len] = '\0';
            /* Strip trailing slash if present */
            if (app_name[name_len - 1] == '/') app_name[name_len - 1] = '\0';

            char app_path[128];
            snprintf(app_path, sizeof(app_path), "/apps/%s", app_name);
            int res = littlefs_remove_recursive(app_path);
            if (res >= 0) {
                int len = snprintf(json_scratch, json_capacity,
                    "{\"status\":\"deleted\",\"app\":\"%s\"}", app_name);
                return build_http_response(file, 200, "OK", "application/json", json_scratch, (size_t)len);
            } else {
                const char *err = "{\"error\":\"App not found or delete failed\"}";
                return build_http_response(file, 404, "Not Found", "application/json", err, strlen(err));
            }
        }
    }

    /* 4. DELETE /api/v1/del/app/{name} (translated DELETE route) */
    if (strncmp(path, "del/app/", 8) == 0) {
        const char *app_name = path + 8;
        char app_path[128];
        snprintf(app_path, sizeof(app_path), "/apps/%s", app_name);
        /* Strip trailing slash if present */
        char *slash = strchr(app_path, '/');
        if (slash) {
            char *last = strrchr(app_path, '/');
            if (last && *(last + 1) == '\0') *last = '\0';
        }
        int res = littlefs_remove_recursive(app_path);
        if (res >= 0) {
            int len = snprintf(json_scratch, json_capacity,
                "{\"status\":\"deleted\",\"app\":\"%s\"}", app_name);
            return build_http_response(file, 200, "OK", "application/json", json_scratch, (size_t)len);
        } else {
            const char *err = "{\"error\":\"App not found\"}";
            return build_http_response(file, 404, "Not Found", "application/json", err, strlen(err));
        }
    }

    /* 5. GET /api/v1/system */
    if (strcmp(path, "system") == 0 || strcmp(path, "system/") == 0) {
        uint32_t uptime = to_ms_since_boot(get_absolute_time());
        int len = snprintf(json_scratch, json_capacity,
            "{\"board_id\":\"%s\",\"board_name\":\"%s\",\"firmware_version\":\"%s\",\"flash_bytes\":%lu,\"uptime_ms\":%lu}",
            AGENTLA_BOARD_ID, AGENTLA_BOARD_NAME, FIRMWARE_VERSION,
            (unsigned long)BOARD_MCU_FLASH_BYTES, (unsigned long)uptime);
        return build_http_response(file, 200, "OK", "application/json", json_scratch, (size_t)len);
    }

    /* 6. POST /api/v1/pins/safe or GET /api/v1/pins/safe */
    if (strcmp(path, "pins/safe") == 0 || strcmp(path, "pins/safe/") == 0) {
        profile_emergency_safe_state();
        const char *resp = "{\"status\":\"safe\",\"pins_quiesced\":true}";
        return build_http_response(file, 200, "OK", "application/json", resp, strlen(resp));
    }

    /* 7. GET /api/v1/pins */
    if (strcmp(path, "pins") == 0 || strcmp(path, "pins/") == 0) {
        bool is_safe = profile_manager_is_safe_state_active();
        const char *prof_name = profile_manager_get_active_name();
        int len = snprintf(json_scratch, json_capacity,
            "{\"status\":\"ok\",\"safe\":%s,\"active_profile\":\"%s\",\"pins\":[]}",
            is_safe ? "true" : "false", prof_name ? prof_name : "Safe State");
        return build_http_response(file, 200, "OK", "application/json", json_scratch, (size_t)len);
    }

    /* Default 404 for unknown REST endpoints */
    const char *err = "{\"error\":\"Unknown REST endpoint\"}";
    return build_http_response(file, 404, "Not Found", "application/json", err, strlen(err));
}

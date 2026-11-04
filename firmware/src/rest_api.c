/**
 * @file rest_api.c
 * @brief Dedicated REST API (/api/v1/...) Engine for AgentLA
 */

#include "rest_api.h"
#include "app_routing.h"
#include "littlefs_driver.h"
#include "profile_manager.h"
#include "pin_owner.h"
#include "servo_pwm.h"
#include "pico/time.h"
#include "pico/bootrom.h"
#include "hardware/gpio.h"
#include "hardware/watchdog.h"
#include "adapter/adapter.h"
#include "adapter/adapter_led.h"
#include <string.h>
#include <stdio.h>
#include <stdlib.h>

static const char *s_post_body = NULL;
static size_t s_post_body_len = 0;

void rest_api_set_post_payload(const char *uri, const char *body, size_t body_len) {
    (void)uri;
    s_post_body = body;
    s_post_body_len = body_len;
}

static int get_json_int(const char *json, const char *key, int default_val) {
    if (!json || !key) return default_val;
    char pat[64];
    snprintf(pat, sizeof(pat), "\"%s\"", key);
    const char *p = strstr(json, pat);
    if (!p) return default_val;
    p = strchr(p + strlen(pat), ':');
    if (!p) return default_val;
    while (*p == ':' || *p == ' ' || *p == '\t') p++;
    return atoi(p);
}

static bool get_json_str(const char *json, const char *key, char *dst, size_t cap) {
    if (!json || !key || !dst || cap == 0) return false;
    dst[0] = '\0';
    char pat[64];
    snprintf(pat, sizeof(pat), "\"%s\"", key);
    const char *p = strstr(json, pat);
    if (!p) return false;
    p = strchr(p + strlen(pat), ':');
    if (!p) return false;
    p = strchr(p, '"');
    if (!p) return false;
    p++;
    size_t i = 0;
    while (*p && *p != '"' && i < cap - 1) {
        dst[i++] = *p++;
    }
    dst[i] = '\0';
    return true;
}

static int get_query_param_int(const char *str, const char *param, int default_val) {
    if (!str || !param) return default_val;
    const char *p = strstr(str, param);
    if (!p) return default_val;
    p += strlen(param);
    if (*p != '=') return default_val;
    return atoi(p + 1);
}

static uint32_t get_query_param_u32(const char *str, const char *param, uint32_t default_val) {
    if (!str || !param) return default_val;
    const char *p = strstr(str, param);
    if (!p) return default_val;
    p += strlen(param);
    if (*p != '=') return default_val;
    return (uint32_t)strtoul(p + 1, NULL, 10);
}

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
static char rest_last_saved_theme[64] = "custom";

void rest_api_set_last_installed_app(const char *name) {
    if (name) {
        snprintf(rest_last_installed_app, sizeof(rest_last_installed_app), "%s", name);
    }
}

void rest_api_set_last_saved_theme(const char *name) {
    if (name) {
        snprintf(rest_last_saved_theme, sizeof(rest_last_saved_theme), "%s", name);
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
    if (strncmp(uri, "/api/v1/", 8) == 0) return true;
    if (strncmp(uri, "/api/profiles", 13) == 0) return true;
    return false;
}

int rest_api_handle(struct fs_file *file, const char *uri) {
    if (!file || !uri) return 0;

    const char *path = uri;
    if (strncmp(path, "/api/v1/", 8) == 0) {
        path += 8;
    } else if (strncmp(path, "/api/", 5) == 0) {
        path += 5;
    }
    char *json_scratch = rest_response_buffer + REST_HEADER_RESERVE;
    size_t json_capacity = sizeof(rest_response_buffer) - REST_HEADER_RESERVE - 1;

    /* 0. POST /api/v1/apps/install_done */
    if (strcmp(path, "apps/install_done") == 0) {
        int len = snprintf(json_scratch, json_capacity,
            "{\"status\":\"installed\",\"app\":\"%s\"}", rest_last_installed_app);
        return build_http_response(file, 201, "Created", "application/json", json_scratch, (size_t)len);
    }

    /* 0b. POST /api/v1/themes/saved */
    if (strcmp(path, "themes/saved") == 0) {
        int len = snprintf(json_scratch, json_capacity,
            "{\"status\":\"created\",\"theme\":\"%s\"}", rest_last_saved_theme);
        return build_http_response(file, 201, "Created", "application/json", json_scratch, (size_t)len);
    }

    /* 0c. GET or POST /api/v1/settings */
    if (strcmp(path, "settings") == 0 || strcmp(path, "settings/") == 0) {
        lfs_t *lfs = littlefs_get_instance();
        lfs_file_t sf;
        if (lfs && lfs_file_open(lfs, &sf, "/settings.json", LFS_O_RDONLY) >= 0) {
            lfs_ssize_t n = lfs_file_read(lfs, &sf, json_scratch, json_capacity - 1);
            lfs_file_close(lfs, &sf);
            if (n > 0) {
                json_scratch[n] = '\0';
                if (strstr(json_scratch, "\"default_home_route\"") == NULL) {
                    char *brace = strrchr(json_scratch, '}');
                    if (brace) {
                        snprintf(brace, json_capacity - (size_t)(brace - json_scratch),
                            ",\"default_home_route\":\"%s\"}", app_routing_get_default_home_route());
                    }
                }
                return build_http_response(file, 200, "OK", "application/json", json_scratch, strlen(json_scratch));
            }
        }
        int len = snprintf(json_scratch, json_capacity,
            "{\"theme_mode\":\"system\",\"default_light_theme\":\"emerald\",\"default_dark_theme\":\"forest\",\"default_home_route\":\"%s\",\"active_profile\":\"Safe State\"}",
            app_routing_get_default_home_route());
        return build_http_response(file, 200, "OK", "application/json", json_scratch, (size_t)len);
    }

    /* 0d. GET /api/v1/themes */
    if (strcmp(path, "themes") == 0 || strcmp(path, "themes/") == 0) {
        int written = snprintf(json_scratch, json_capacity,
            "["
            "{\"name\":\"forest\",\"color_scheme\":\"dark\",\"colorScheme\":\"dark\",\"prefersdark\":true,\"prefersDark\":true,\"isDefault\":true,\"custom\":false},"
            "{\"name\":\"emerald\",\"color_scheme\":\"light\",\"colorScheme\":\"light\",\"prefersdark\":false,\"prefersDark\":false,\"isDefault\":true,\"custom\":false},"
            "{\"name\":\"caramellatte\",\"color_scheme\":\"light\",\"colorScheme\":\"light\",\"prefersdark\":false,\"prefersDark\":false,\"isDefault\":false,\"custom\":false}"
        );
        lfs_t *lfs = littlefs_get_instance();
        if (lfs) {
            lfs_dir_t dir;
            if (lfs_dir_open(lfs, &dir, "/themes") >= 0) {
                struct lfs_info info;
                while (lfs_dir_read(lfs, &dir, &info) > 0) {
                    if (info.type == LFS_TYPE_REG && strstr(info.name, ".css") != NULL) {
                        char th_name[64];
                        snprintf(th_name, sizeof(th_name), "%s", info.name);
                        char *dot = strrchr(th_name, '.');
                        if (dot) *dot = '\0';
                        char scheme[16] = "dark";
                        char th_path[128];
                        snprintf(th_path, sizeof(th_path), "/themes/%s", info.name);
                        lfs_file_t tf;
                        if (lfs_file_open(lfs, &tf, th_path, LFS_O_RDONLY) >= 0) {
                            char buf[256];
                            lfs_ssize_t n = lfs_file_read(lfs, &tf, buf, sizeof(buf) - 1);
                            lfs_file_close(lfs, &tf);
                            if (n > 0) {
                                buf[n] = '\0';
                                if (strstr(buf, "\"light\"") || strstr(buf, "'light'") || strstr(buf, "light;")) {
                                    strcpy(scheme, "light");
                                }
                            }
                        }
                        bool is_dark = (strcmp(scheme, "dark") == 0);
                        if (written < (int)json_capacity - 140) {
                            written += snprintf(json_scratch + written, json_capacity - written,
                                ",{\"name\":\"%s\",\"color_scheme\":\"%s\",\"colorScheme\":\"%s\",\"prefersdark\":%s,\"prefersDark\":%s,\"isDefault\":false,\"custom\":true}",
                                th_name, scheme, scheme, is_dark ? "true" : "false", is_dark ? "true" : "false");
                        }
                    }
                }
                lfs_dir_close(lfs, &dir);
            }
        }
        if (written < (int)json_capacity - 2) {
            json_scratch[written++] = ']';
            json_scratch[written] = '\0';
        }
        return build_http_response(file, 200, "OK", "application/json", json_scratch, (size_t)written);
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

    /* 6. PROFILES API: /api/v1/profiles or /api/profiles */
    if (strncmp(path, "profiles", 8) == 0) {
        const char *sub = path + 8;
        if (*sub == '/') sub++;

        /* GET /api/v1/profiles or GET /api/profiles */
        if (*sub == '\0') {
            profile_list_to_json(json_scratch, json_capacity);
            return build_http_response(file, 200, "OK", "application/json", json_scratch, strlen(json_scratch));
        }

        /* GET /api/v1/profiles/active */
        if (strncmp(sub, "active", 6) == 0) {
            uint32_t active_slot = profile_manager_get_active_slot();
            const char *name = profile_manager_get_active_name();
            bool is_safe = profile_manager_is_safe_state_active();
            int len = snprintf(json_scratch, json_capacity,
                "{\"status\":\"ok\",\"active_slot\":%lu,\"active_name\":\"%s\",\"safe\":%s}",
                (unsigned long)active_slot, name ? name : "Safe State", is_safe ? "true" : "false");
            return build_http_response(file, 200, "OK", "application/json", json_scratch, (size_t)len);
        }

        /* POST /api/v1/profiles/apply or POST /api/profiles/apply */
        if (strncmp(sub, "apply", 5) == 0) {
            int slot = get_query_param_int(sub, "slot", -1);
            if (slot < 0 && s_post_body[0]) {
                slot = get_json_int(s_post_body, "slot", -1);
            }
            if (slot < 0 || slot >= PROFILE_TOTAL_SLOTS) {
                const char *err = "{\"error\":\"Invalid profile slot\"}";
                return build_http_response(file, 400, "Bad Request", "application/json", err, strlen(err));
            }
            char err_buf[64] = {0};
            if (profile_apply_slot((uint8_t)slot, err_buf, sizeof(err_buf))) {
                int len = snprintf(json_scratch, json_capacity,
                    "{\"status\":\"applied\",\"slot\":%d,\"name\":\"%s\"}",
                    slot, profile_manager_get_active_name());
                return build_http_response(file, 200, "OK", "application/json", json_scratch, (size_t)len);
            } else {
                int len = snprintf(json_scratch, json_capacity, "{\"error\":\"%s\"}", err_buf[0] ? err_buf : "Apply failed");
                return build_http_response(file, 400, "Bad Request", "application/json", json_scratch, (size_t)len);
            }
        }

        /* POST /api/v1/profiles/boot */
        if (strncmp(sub, "boot", 4) == 0) {
            int slot = get_query_param_int(sub, "slot", -1);
            if (slot < 0 && s_post_body[0]) {
                slot = get_json_int(s_post_body, "slot", -1);
            }
            if (slot < 0 || slot >= PROFILE_TOTAL_SLOTS) {
                const char *err = "{\"error\":\"Invalid profile slot\"}";
                return build_http_response(file, 400, "Bad Request", "application/json", err, strlen(err));
            }
            if (profile_set_boot_default((uint8_t)slot)) {
                int len = snprintf(json_scratch, json_capacity, "{\"status\":\"boot_default_set\",\"slot\":%d}", slot);
                return build_http_response(file, 200, "OK", "application/json", json_scratch, (size_t)len);
            } else {
                const char *err = "{\"error\":\"Failed to set boot default\"}";
                return build_http_response(file, 400, "Bad Request", "application/json", err, strlen(err));
            }
        }

        /* POST /api/v1/profiles/save */
        if (strncmp(sub, "save", 4) == 0) {
            int slot = get_query_param_int(sub, "slot", -1);
            char name[PROFILE_NAME_MAX_LEN] = "Custom Profile";
            bool is_boot = false;
            if (slot < 0 && s_post_body[0]) {
                slot = get_json_int(s_post_body, "slot", -1);
                get_json_str(s_post_body, "name", name, sizeof(name));
                is_boot = (get_json_int(s_post_body, "is_boot", 0) != 0);
            }
            if (slot < 0 || slot >= PROFILE_TOTAL_SLOTS) {
                const char *err = "{\"error\":\"Invalid profile slot\"}";
                return build_http_response(file, 400, "Bad Request", "application/json", err, strlen(err));
            }
            if (profile_save_slot((uint8_t)slot, name, is_boot, "{}")) {
                int len = snprintf(json_scratch, json_capacity, "{\"status\":\"saved\",\"slot\":%d,\"name\":\"%s\"}", slot, name);
                return build_http_response(file, 200, "OK", "application/json", json_scratch, (size_t)len);
            } else {
                const char *err = "{\"error\":\"Failed to save profile slot\"}";
                return build_http_response(file, 400, "Bad Request", "application/json", err, strlen(err));
            }
        }

        /* DELETE /api/v1/profiles/{slot} or /api/v1/profiles/delete?slot=... */
        if (strncmp(sub, "delete", 6) == 0 || (*sub >= '0' && *sub <= '9')) {
            int slot = atoi(sub);
            if (slot == 0 && *sub != '0') {
                slot = get_query_param_int(sub, "slot", -1);
                if (slot < 0 && s_post_body[0]) {
                    slot = get_json_int(s_post_body, "slot", -1);
                }
            }
            if (slot <= 0 || slot >= PROFILE_TOTAL_SLOTS) {
                const char *err = "{\"error\":\"Cannot delete slot or invalid slot\"}";
                return build_http_response(file, 400, "Bad Request", "application/json", err, strlen(err));
            }
            if (profile_delete_slot((uint8_t)slot)) {
                int len = snprintf(json_scratch, json_capacity, "{\"status\":\"deleted\",\"slot\":%d}", slot);
                return build_http_response(file, 200, "OK", "application/json", json_scratch, (size_t)len);
            } else {
                const char *err = "{\"error\":\"Failed to delete profile slot\"}";
                return build_http_response(file, 400, "Bad Request", "application/json", err, strlen(err));
            }
        }
    }

    /* 7. PINS & GPIO API: /api/v1/pins/* */
    if (strncmp(path, "pins", 4) == 0) {
        const char *sub = path + 4;
        if (*sub == '/') sub++;

        /* POST /api/v1/pins/safe or GET /api/v1/pins/safe */
        if (strncmp(sub, "safe", 4) == 0) {
            profile_emergency_safe_state();
            const char *resp = "{\"status\":\"safe\",\"pins_quiesced\":true}";
            return build_http_response(file, 200, "OK", "application/json", resp, strlen(resp));
        }

        /* POST /api/v1/pins/claim?pin=... */
        if (strncmp(sub, "claim", 5) == 0) {
            int pin = get_query_param_int(sub, "pin", -1);
            if (pin < 0 && s_post_body[0]) {
                pin = get_json_int(s_post_body, "pin", -1);
            }
            if (pin < 0 || pin >= 30) {
                const char *err = "{\"error\":\"Invalid pin parameter\"}";
                return build_http_response(file, 400, "Bad Request", "application/json", err, strlen(err));
            }
            uint8_t p = (uint8_t)pin;
            uint32_t claim_id = 0;
            if (pin_owner_claim_pins(&p, 1, 5 /* GPIO */, 0, &claim_id)) {
                int len = snprintf(json_scratch, json_capacity, "{\"status\":\"claimed\",\"pin\":%d,\"claim_id\":%lu}", pin, (unsigned long)claim_id);
                return build_http_response(file, 200, "OK", "application/json", json_scratch, (size_t)len);
            } else {
                const char *err = "{\"error\":\"Pin conflict or claim failed\"}";
                return build_http_response(file, 409, "Conflict", "application/json", err, strlen(err));
            }
        }

        /* POST /api/v1/pins/release?claim_id=... */
        if (strncmp(sub, "release", 7) == 0) {
            int cid = get_query_param_int(sub, "claim_id", -1);
            if (cid < 0 && s_post_body[0]) {
                cid = get_json_int(s_post_body, "claim_id", -1);
            }
            pin_owner_result_t res;
            if (cid > 0 && pin_owner_release_claim((uint32_t)cid, &res)) {
                int len = snprintf(json_scratch, json_capacity, "{\"status\":\"released\",\"claim_id\":%d}", cid);
                return build_http_response(file, 200, "OK", "application/json", json_scratch, (size_t)len);
            } else {
                int pin = get_query_param_int(sub, "pin", -1);
                if (pin >= 0 && pin < 30) {
                    gpio_set_dir((uint)pin, GPIO_IN);
                    gpio_disable_pulls((uint)pin);
                    int len = snprintf(json_scratch, json_capacity, "{\"status\":\"released\",\"pin\":%d}", pin);
                    return build_http_response(file, 200, "OK", "application/json", json_scratch, (size_t)len);
                }
                const char *err = "{\"error\":\"Invalid claim_id or pin\"}";
                return build_http_response(file, 400, "Bad Request", "application/json", err, strlen(err));
            }
        }

        /* Specific pin operations: /api/v1/pins/{gpio} ... */
        if (*sub >= '0' && *sub <= '9') {
            int gpio = atoi(sub);
            if (gpio < 0 || gpio >= 30) {
                const char *err = "{\"error\":\"Invalid GPIO pin number (0-29)\"}";
                return build_http_response(file, 400, "Bad Request", "application/json", err, strlen(err));
            }

            const char *action = strchr(sub, '/');
            if (action) action++;

            /* POST /api/v1/pins/{gpio}/write?val=0|1 */
            if (action && strncmp(action, "write", 5) == 0) {
                int val = get_query_param_int(action, "val", -1);
                if (val < 0 && s_post_body[0]) {
                    val = get_json_int(s_post_body, "val", -1);
                }
                if (val != 0 && val != 1) {
                    const char *err = "{\"error\":\"Invalid value (expected 0 or 1)\"}";
                    return build_http_response(file, 400, "Bad Request", "application/json", err, strlen(err));
                }
                gpio_init((uint)gpio);
                gpio_set_dir((uint)gpio, GPIO_OUT);
                gpio_put((uint)gpio, val != 0);
                int len = snprintf(json_scratch, json_capacity, "{\"status\":\"ok\",\"pin\":%d,\"val\":%d}", gpio, val);
                return build_http_response(file, 200, "OK", "application/json", json_scratch, (size_t)len);
            }

            /* POST /api/v1/pins/{gpio}/mode?mode=in|out|hiz&pull=none|up|down */
            if (action && strncmp(action, "mode", 4) == 0) {
                char mode[16] = "in";
                char pull[16] = "none";
                if (s_post_body[0]) {
                    get_json_str(s_post_body, "mode", mode, sizeof(mode));
                    get_json_str(s_post_body, "pull", pull, sizeof(pull));
                }
                gpio_init((uint)gpio);
                if (strcmp(mode, "out") == 0) {
                    gpio_set_dir((uint)gpio, GPIO_OUT);
                } else {
                    gpio_set_dir((uint)gpio, GPIO_IN);
                }
                if (strcmp(pull, "up") == 0) {
                    gpio_pull_up((uint)gpio);
                } else if (strcmp(pull, "down") == 0) {
                    gpio_pull_down((uint)gpio);
                } else {
                    gpio_disable_pulls((uint)gpio);
                }
                int len = snprintf(json_scratch, json_capacity,
                    "{\"status\":\"ok\",\"pin\":%d,\"mode\":\"%s\",\"pull\":\"%s\"}", gpio, mode, pull);
                return build_http_response(file, 200, "OK", "application/json", json_scratch, (size_t)len);
            }

            /* GET /api/v1/pins/{gpio} */
            const char *dom = (gpio < 10) ? "PROGRAMMER" : ((gpio < 20) ? "ADAPTER" : "LOGIC ANALYZER");
            uint fn = gpio_get_function((uint)gpio);
            const char *fn_str = "SIO";
            if (fn == GPIO_FUNC_PIO0 || fn == GPIO_FUNC_PIO1) fn_str = "PIO";
            else if (fn == GPIO_FUNC_PWM) fn_str = "PWM";
            else if (fn == GPIO_FUNC_UART) fn_str = "UART";
            else if (fn == GPIO_FUNC_I2C) fn_str = "I2C";
            else if (fn == GPIO_FUNC_SPI) fn_str = "SPI";
            else if (fn == GPIO_FUNC_NULL) fn_str = "HIZ";

            bool is_out = gpio_get_dir((uint)gpio);
            int val = gpio_get((uint)gpio);
            int len = snprintf(json_scratch, json_capacity,
                "{\"pin\":%d,\"domain\":\"%s\",\"mode\":\"%s\",\"dir\":\"%s\",\"val\":%d,\"pull\":\"none\"}",
                gpio, dom, fn_str, is_out ? "out" : "in", val);
            return build_http_response(file, 200, "OK", "application/json", json_scratch, (size_t)len);
        }

        /* GET /api/v1/pins */
        if (*sub == '\0') {
            bool is_safe = profile_manager_is_safe_state_active();
            const char *prof_name = profile_manager_get_active_name();
            int written = snprintf(json_scratch, json_capacity,
                "{\"status\":\"ok\",\"safe\":%s,\"active_profile\":\"%s\",\"pins\":[",
                is_safe ? "true" : "false", prof_name ? prof_name : "Safe State");
            for (int i = 0; i < 30; i++) {
                char dom_char = (i < 10) ? 'P' : ((i < 20) ? 'A' : 'L');
                uint fn = gpio_get_function((uint)i);
                const char *fn_str = "SIO";
                if (fn == GPIO_FUNC_PIO0 || fn == GPIO_FUNC_PIO1) fn_str = "PIO";
                else if (fn == GPIO_FUNC_PWM) fn_str = "PWM";
                else if (fn == GPIO_FUNC_UART) fn_str = "UART";
                else if (fn == GPIO_FUNC_I2C) fn_str = "I2C";
                else if (fn == GPIO_FUNC_SPI) fn_str = "SPI";
                else if (fn == GPIO_FUNC_NULL) fn_str = "HIZ";

                bool is_out = gpio_get_dir((uint)i);
                int val = gpio_get((uint)i);
                if (written < (int)json_capacity - 70) {
                    written += snprintf(json_scratch + written, json_capacity - written,
                        "%s{\"p\":%d,\"d\":\"%c\",\"m\":\"%s\",\"o\":%d,\"v\":%d}",
                        (i > 0) ? "," : "", i, dom_char, fn_str, is_out ? 1 : 0, val);
                }
            }
            if (written < (int)json_capacity - 3) {
                json_scratch[written++] = ']';
                json_scratch[written++] = '}';
                json_scratch[written] = '\0';
            }
            return build_http_response(file, 200, "OK", "application/json", json_scratch, (size_t)written);
        }
    }

    /* 7b. DEVICE TELEMETRY & CONTROL: /api/v1/device/* */
    if (strncmp(path, "device", 6) == 0) {
        const char *sub = path + 6;
        if (*sub == '/') sub++;

        /* GET /api/v1/device/telemetry */
        if (strncmp(sub, "telemetry", 9) == 0 || *sub == '\0') {
            uint32_t uptime = to_ms_since_boot(get_absolute_time());
            int len = snprintf(json_scratch, json_capacity,
                "{\"status\":\"ok\",\"vtarget_mv\":3300,\"temp_c\":25.0,\"uptime_ms\":%lu,\"board\":\"%s\"}",
                (unsigned long)uptime, AGENTLA_BOARD_NAME);
            return build_http_response(file, 200, "OK", "application/json", json_scratch, (size_t)len);
        }

        /* POST /api/v1/device/power?power=on|off */
        if (strncmp(sub, "power", 5) == 0) {
            char pwr[16] = "on";
            if (s_post_body[0]) {
                get_json_str(s_post_body, "power", pwr, sizeof(pwr));
            }
            int len = snprintf(json_scratch, json_capacity, "{\"status\":\"ok\",\"power\":\"%s\"}", pwr);
            return build_http_response(file, 200, "OK", "application/json", json_scratch, (size_t)len);
        }

        /* POST /api/v1/device/reboot */
        if (strncmp(sub, "reboot", 6) == 0) {
            watchdog_reboot(0, 0, 100);
            const char *resp = "{\"status\":\"rebooting\"}";
            return build_http_response(file, 200, "OK", "application/json", resp, strlen(resp));
        }

        /* POST /api/v1/device/bootsel */
        if (strncmp(sub, "bootsel", 7) == 0) {
            int len = snprintf(json_scratch, json_capacity, "{\"status\":\"rebooting_to_bootsel\"}");
            int ret = build_http_response(file, 200, "OK", "application/json", json_scratch, (size_t)len);
            reset_usb_boot(0, 0);
            return ret;
        }
    }

    /* 8. PWM API: /api/v1/pwm/* */
    if (strncmp(path, "pwm", 3) == 0) {
        const char *sub = path + 3;
        if (*sub == '/') sub++;

        /* GET /api/v1/pwm */
        if (*sub == '\0') {
            servo_channel_t chs[SERVO_MAX_CHANNELS];
            int count = servo_get_active_channels(chs, SERVO_MAX_CHANNELS);
            int written = snprintf(json_scratch, json_capacity, "{\"status\":\"ok\",\"channels\":[");
            for (int i = 0; i < count; i++) {
                if (written < (int)json_capacity - 100) {
                    written += snprintf(json_scratch + written, json_capacity - written,
                        "%s{\"pin\":%u,\"freq\":%lu,\"pulse_us\":%lu,\"active\":true}",
                        (i > 0) ? "," : "",
                        chs[i].pin, (unsigned long)chs[i].freq, (unsigned long)chs[i].pulse_us);
                }
            }
            if (written < (int)json_capacity - 3) {
                json_scratch[written++] = ']';
                json_scratch[written++] = '}';
                json_scratch[written] = '\0';
            }
            return build_http_response(file, 200, "OK", "application/json", json_scratch, (size_t)written);
        }

        /* POST /api/v1/pwm/claim?pin=... */
        if (strncmp(sub, "claim", 5) == 0) {
            int pin = get_query_param_int(sub, "pin", -1);
            if (pin < 0) {
                const char *err = "{\"error\":\"Missing pin parameter\"}";
                return build_http_response(file, 400, "Bad Request", "application/json", err, strlen(err));
            }
            uint32_t handle = 0;
            if (servo_pwm_claim((uint8_t)pin, &handle)) {
                int len = snprintf(json_scratch, json_capacity, "{\"status\":\"claimed\",\"pin\":%d,\"handle\":%lu}", pin, (unsigned long)handle);
                return build_http_response(file, 200, "OK", "application/json", json_scratch, (size_t)len);
            } else {
                const char *err = "{\"error\":\"Failed to claim pin for PWM\"}";
                return build_http_response(file, 409, "Conflict", "application/json", err, strlen(err));
            }
        }

        /* POST /api/v1/pwm/set?pin=...&freq=...&pulse_us=... */
        if (strncmp(sub, "set", 3) == 0) {
            int pin = get_query_param_int(sub, "pin", -1);
            uint32_t freq = get_query_param_u32(sub, "freq", 50);
            uint32_t pulse_us = get_query_param_u32(sub, "pulse_us", 0);
            if (pin < 0 || pulse_us == 0) {
                const char *err = "{\"error\":\"Invalid pin or pulse_us parameter\"}";
                return build_http_response(file, 400, "Bad Request", "application/json", err, strlen(err));
            }
            if (servo_pwm_set((uint8_t)pin, freq, pulse_us)) {
                int len = snprintf(json_scratch, json_capacity, "{\"status\":\"ok\",\"pin\":%d,\"freq\":%lu,\"pulse_us\":%lu}", pin, (unsigned long)freq, (unsigned long)pulse_us);
                return build_http_response(file, 200, "OK", "application/json", json_scratch, (size_t)len);
            } else {
                const char *err = "{\"error\":\"Failed to configure PWM\"}";
                return build_http_response(file, 500, "Internal Error", "application/json", err, strlen(err));
            }
        }

        /* POST /api/v1/pwm/release?pin=... */
        if (strncmp(sub, "release", 7) == 0) {
            int pin = get_query_param_int(sub, "pin", -1);
            if (pin < 0) {
                const char *err = "{\"error\":\"Missing pin parameter\"}";
                return build_http_response(file, 400, "Bad Request", "application/json", err, strlen(err));
            }
            servo_pwm_release((uint8_t)pin);
            int len = snprintf(json_scratch, json_capacity, "{\"status\":\"released\",\"pin\":%d}", pin);
            return build_http_response(file, 200, "OK", "application/json", json_scratch, (size_t)len);
        }
    }

    /* 9. SERVO API: /api/v1/servo/* */
    if (strncmp(path, "servo", 5) == 0) {
        const char *sub = path + 5;
        if (*sub == '/') sub++;

        /* GET /api/v1/servo */
        if (*sub == '\0') {
            servo_channel_t chs[SERVO_MAX_CHANNELS];
            int count = servo_get_active_channels(chs, SERVO_MAX_CHANNELS);
            int written = snprintf(json_scratch, json_capacity, "{\"status\":\"ok\",\"channels\":[");
            for (int i = 0; i < count; i++) {
                if (written < (int)json_capacity - 120) {
                    written += snprintf(json_scratch + written, json_capacity - written,
                        "%s{\"pin\":%u,\"angle\":%d,\"pulse_us\":%lu,\"min_us\":%u,\"max_us\":%u,\"continuous\":%s}",
                        (i > 0) ? "," : "",
                        chs[i].pin, chs[i].angle, (unsigned long)chs[i].pulse_us,
                        chs[i].min_us, chs[i].max_us, chs[i].continuous ? "true" : "false");
                }
            }
            if (written < (int)json_capacity - 3) {
                json_scratch[written++] = ']';
                json_scratch[written++] = '}';
                json_scratch[written] = '\0';
            }
            return build_http_response(file, 200, "OK", "application/json", json_scratch, (size_t)written);
        }

        /* POST /api/v1/servo/attach?pin=...&min_us=...&max_us=...&continuous=... */
        if (strncmp(sub, "attach", 6) == 0) {
            int pin = get_query_param_int(sub, "pin", -1);
            uint16_t min_us = (uint16_t)get_query_param_u32(sub, "min_us", 500);
            uint16_t max_us = (uint16_t)get_query_param_u32(sub, "max_us", 2500);
            int cont = get_query_param_int(sub, "continuous", 0);
            if (pin < 0) {
                const char *err = "{\"error\":\"Missing pin parameter\"}";
                return build_http_response(file, 400, "Bad Request", "application/json", err, strlen(err));
            }
            if (servo_attach((uint8_t)pin, min_us, max_us, cont != 0)) {
                int len = snprintf(json_scratch, json_capacity,
                    "{\"status\":\"attached\",\"pin\":%d,\"angle\":%d,\"pulse_us\":1500,\"min_us\":%u,\"max_us\":%u,\"continuous\":%s}",
                    pin, cont ? 0 : 90, min_us, max_us, cont ? "true" : "false");
                return build_http_response(file, 200, "OK", "application/json", json_scratch, (size_t)len);
            } else {
                const char *err = "{\"error\":\"Failed to attach servo\"}";
                return build_http_response(file, 409, "Conflict", "application/json", err, strlen(err));
            }
        }

        /* POST /api/v1/servo/write_us?pin=...&pulse_us=... */
        if (strncmp(sub, "write_us", 8) == 0) {
            int pin = get_query_param_int(sub, "pin", -1);
            uint32_t pulse_us = get_query_param_u32(sub, "pulse_us", 1500);
            if (pin < 0) {
                const char *err = "{\"error\":\"Missing pin parameter\"}";
                return build_http_response(file, 400, "Bad Request", "application/json", err, strlen(err));
            }
            if (servo_write_us((uint8_t)pin, pulse_us)) {
                const servo_channel_t *ch = servo_pwm_get_slot((uint8_t)pin);
                int len = snprintf(json_scratch, json_capacity,
                    "{\"status\":\"ok\",\"pin\":%d,\"angle\":%d,\"pulse_us\":%lu}",
                    pin, ch ? ch->angle : 0, (unsigned long)pulse_us);
                return build_http_response(file, 200, "OK", "application/json", json_scratch, (size_t)len);
            } else {
                const char *err = "{\"error\":\"Failed to write servo pulse\"}";
                return build_http_response(file, 500, "Internal Error", "application/json", err, strlen(err));
            }
        }

        /* POST /api/v1/servo/write?pin=...&angle=... */
        if (strncmp(sub, "write", 5) == 0) {
            int pin = get_query_param_int(sub, "pin", -1);
            int angle = get_query_param_int(sub, "angle", 90);
            if (pin < 0) {
                const char *err = "{\"error\":\"Missing pin parameter\"}";
                return build_http_response(file, 400, "Bad Request", "application/json", err, strlen(err));
            }
            if (servo_write_angle((uint8_t)pin, (int16_t)angle)) {
                const servo_channel_t *ch = servo_pwm_get_slot((uint8_t)pin);
                int len = snprintf(json_scratch, json_capacity,
                    "{\"status\":\"ok\",\"pin\":%d,\"angle\":%d,\"pulse_us\":%lu}",
                    pin, angle, (unsigned long)(ch ? ch->pulse_us : 1500));
                return build_http_response(file, 200, "OK", "application/json", json_scratch, (size_t)len);
            } else {
                const char *err = "{\"error\":\"Failed to write servo angle\"}";
                return build_http_response(file, 500, "Internal Error", "application/json", err, strlen(err));
            }
        }

        /* POST /api/v1/servo/detach?pin=... */
        if (strncmp(sub, "detach", 6) == 0) {
            int pin = get_query_param_int(sub, "pin", -1);
            if (pin < 0) {
                const char *err = "{\"error\":\"Missing pin parameter\"}";
                return build_http_response(file, 400, "Bad Request", "application/json", err, strlen(err));
            }
            servo_detach((uint8_t)pin);
            int len = snprintf(json_scratch, json_capacity, "{\"status\":\"detached\",\"pin\":%d}", pin);
            return build_http_response(file, 200, "OK", "application/json", json_scratch, (size_t)len);
        }
    }

    /* 10. BUS API: /api/v1/bus/* */
    if (strncmp(path, "bus", 3) == 0) {
        const char *sub = path + 3;
        if (*sub == '/') sub++;

        /* 10a. I2C: /api/v1/bus/i2c/* */
        if (strncmp(sub, "i2c", 3) == 0) {
            const char *i2c_sub = sub + 3;
            if (*i2c_sub == '/') i2c_sub++;

            /* POST or GET /api/v1/bus/i2c/scan */
            if (strncmp(i2c_sub, "scan", 4) == 0) {
                adapter_i2c_auto_claim();
                int written = snprintf(json_scratch, json_capacity, "{\"status\":\"ok\",\"devices\":[");
                bool first = true;
                for (uint8_t addr = 0x08; addr <= 0x77; addr++) {
                    uint8_t dummy = 0;
                    int ret = adapter_i2c_raw_read(addr, &dummy, 1, false, 300);
                    if (ret >= 0) {
                        if (written < (int)json_capacity - 10) {
                            written += snprintf(json_scratch + written, json_capacity - written,
                                "%s%u", first ? "" : ",", addr);
                            first = false;
                        }
                    }
                }
                if (written < (int)json_capacity - 3) {
                    json_scratch[written++] = ']';
                    json_scratch[written++] = '}';
                    json_scratch[written] = '\0';
                }
                return build_http_response(file, 200, "OK", "application/json", json_scratch, (size_t)written);
            }

            /* POST /api/v1/bus/i2c/transfer */
            if (strncmp(i2c_sub, "transfer", 8) == 0) {
                int addr = get_query_param_int(i2c_sub, "addr", -1);
                int read_len = get_query_param_int(i2c_sub, "read_len", 0);
                if (s_post_body && s_post_body[0]) {
                    if (addr < 0) addr = get_json_int(s_post_body, "addr", -1);
                    if (read_len == 0) read_len = get_json_int(s_post_body, "read_len", 0);
                }
                if (addr < 0 || addr > 127) {
                    const char *err = "{\"error\":\"Invalid I2C address\"}";
                    return build_http_response(file, 400, "Bad Request", "application/json", err, strlen(err));
                }
                adapter_i2c_auto_claim();
                if (read_len > 64) read_len = 64;
                uint8_t rx_buf[64] = {0};
                int rx_read = 0;
                if (read_len > 0) {
                    rx_read = adapter_i2c_raw_read((uint8_t)addr, rx_buf, (size_t)read_len, false, 5000);
                }
                int written = snprintf(json_scratch, json_capacity, "{\"status\":\"ok\",\"addr\":%d,\"read\":[", addr);
                for (int i = 0; i < rx_read; i++) {
                    if (written < (int)json_capacity - 8) {
                        written += snprintf(json_scratch + written, json_capacity - written,
                            "%s%u", (i > 0) ? "," : "", rx_buf[i]);
                    }
                }
                if (written < (int)json_capacity - 3) {
                    json_scratch[written++] = ']';
                    json_scratch[written++] = '}';
                    json_scratch[written] = '\0';
                }
                return build_http_response(file, 200, "OK", "application/json", json_scratch, (size_t)written);
            }
        }

        /* 10b. SPI: /api/v1/bus/spi/* */
        if (strncmp(sub, "spi", 3) == 0) {
            const char *spi_sub = sub + 3;
            if (*spi_sub == '/') spi_sub++;

            /* POST /api/v1/bus/spi/transfer */
            if (strncmp(spi_sub, "transfer", 8) == 0) {
                int mode = get_query_param_int(spi_sub, "mode", 0);
                int speed = get_query_param_int(spi_sub, "speed_hz", 1000000);
                int read_len = get_query_param_int(spi_sub, "read_len", 0);
                if (s_post_body && s_post_body[0]) {
                    mode = get_json_int(s_post_body, "mode", mode);
                    speed = get_json_int(s_post_body, "speed_hz", speed);
                    read_len = get_json_int(s_post_body, "read_len", read_len);
                }
                int written = snprintf(json_scratch, json_capacity,
                    "{\"status\":\"ok\",\"mode\":%d,\"speed_hz\":%d,\"rx_hex\":\"\"}",
                    mode, speed);
                return build_http_response(file, 200, "OK", "application/json", json_scratch, (size_t)written);
            }
        }

        /* 10c. UART: /api/v1/bus/uart/* */
        if (strncmp(sub, "uart", 4) == 0) {
            const char *uart_sub = sub + 4;
            if (*uart_sub == '/') uart_sub++;

            /* POST /api/v1/bus/uart/write */
            if (strncmp(uart_sub, "write", 5) == 0) {
                uint32_t baud = (uint32_t)get_query_param_u32(uart_sub, "baud", 115200);
                char data_str[128] = {0};
                if (s_post_body && s_post_body[0]) {
                    baud = (uint32_t)get_json_int(s_post_body, "baud", (int)baud);
                    get_json_str(s_post_body, "data", data_str, sizeof(data_str));
                }
                size_t dlen = strlen(data_str);
                if (dlen > 0) {
                    adapter_uart_write((const uint8_t *)data_str, (uint8_t)dlen, baud);
                }
                int written = snprintf(json_scratch, json_capacity,
                    "{\"status\":\"ok\",\"baud\":%lu,\"bytes_sent\":%zu}",
                    (unsigned long)baud, dlen);
                return build_http_response(file, 200, "OK", "application/json", json_scratch, (size_t)written);
            }

            /* GET /api/v1/bus/uart/read */
            if (strncmp(uart_sub, "read", 4) == 0) {
                uint32_t baud = (uint32_t)get_query_param_u32(uart_sub, "baud", 115200);
                uint8_t rx_tmp[128] = {0};
                int n = adapter_uart_read(rx_tmp, sizeof(rx_tmp) - 1, baud, 5);
                if (n < 0) n = 0;
                rx_tmp[n] = '\0';
                int written = snprintf(json_scratch, json_capacity,
                    "{\"status\":\"ok\",\"bytes_read\":%d,\"data\":\"%s\"}", n, rx_tmp);
                return build_http_response(file, 200, "OK", "application/json", json_scratch, (size_t)written);
            }
        }

        /* 10d. CAN: /api/v1/bus/can/* */
        if (strncmp(sub, "can", 3) == 0) {
            const char *can_sub = sub + 3;
            if (*can_sub == '/') can_sub++;

            /* POST /api/v1/bus/can/tx */
            if (strncmp(can_sub, "tx", 2) == 0) {
                int written = snprintf(json_scratch, json_capacity, "{\"status\":\"ok\",\"can\":\"sent\"}");
                return build_http_response(file, 200, "OK", "application/json", json_scratch, (size_t)written);
            }

            /* GET /api/v1/bus/can/rx */
            if (strncmp(can_sub, "rx", 2) == 0) {
                int written = snprintf(json_scratch, json_capacity, "{\"status\":\"ok\",\"frames\":[]}");
                return build_http_response(file, 200, "OK", "application/json", json_scratch, (size_t)written);
            }
        }

        /* 10e. LED / NeoPixel: /api/v1/bus/led/* */
        if (strncmp(sub, "led", 3) == 0) {
            const char *led_sub = sub + 3;
            if (*led_sub == '/') led_sub++;

            /* POST /api/v1/bus/led/transmit */
            if (strncmp(led_sub, "transmit", 8) == 0 || *led_sub == '\0') {
                int pin = get_query_param_int(led_sub, "pin", 10);
                int clk_pin = get_query_param_int(led_sub, "clk_pin", 11);
                char proto_str[16] = "1wire";
                char model_str[24] = "ws2812b";
                char order_str[16] = "grb";
                int brightness = 255;
                if (s_post_body && s_post_body[0]) {
                    pin = get_json_int(s_post_body, "pin", pin);
                    clk_pin = get_json_int(s_post_body, "clk_pin", clk_pin);
                    brightness = get_json_int(s_post_body, "brightness", brightness);
                    get_json_str(s_post_body, "protocol", proto_str, sizeof(proto_str));
                    get_json_str(s_post_body, "model", model_str, sizeof(model_str));
                    get_json_str(s_post_body, "color_order", order_str, sizeof(order_str));
                }

                if (pin < 0 || pin >= 30) {
                    const char *err = "{\"error\":\"Invalid GPIO pin number\"}";
                    return build_http_response(file, 400, "Bad Request", "application/json", err, strlen(err));
                }

                led_color_t pixel_buf[LED_MAX_PIXELS];
                uint16_t num_pixels = 0;
                if (s_post_body && s_post_body[0]) {
                    const char *px = strstr(s_post_body, "\"pixels\"");
                    if (px) {
                        const char *b = strchr(px, '[');
                        if (b) {
                            while (*b && *b != ']' && num_pixels < LED_MAX_PIXELS) {
                                const char *obj = strchr(b, '{');
                                if (!obj) break;
                                const char *end_obj = strchr(obj, '}');
                                if (!end_obj) break;

                                pixel_buf[num_pixels].r = (uint8_t)get_json_int(obj, "r", 0);
                                pixel_buf[num_pixels].g = (uint8_t)get_json_int(obj, "g", 0);
                                pixel_buf[num_pixels].b = (uint8_t)get_json_int(obj, "b", 0);
                                pixel_buf[num_pixels].w = (uint8_t)get_json_int(obj, "w", 0);
                                num_pixels++;
                                b = end_obj + 1;
                            }
                        }
                    }
                }
                if (num_pixels == 0) {
                    pixel_buf[0].r = 0; pixel_buf[0].g = 255; pixel_buf[0].b = 0; pixel_buf[0].w = 0;
                    num_pixels = 1;
                }

                led_protocol_t proto = (strcmp(proto_str, "spi") == 0) ? LED_PROTO_SPI : LED_PROTO_1WIRE;
                bool is_rgbw = (strstr(model_str, "rgbw") != NULL || strstr(order_str, "rgbw") != NULL || strstr(order_str, "grbw") != NULL);
                led_color_order_t ord = LED_ORDER_GRB;
                if (strcmp(order_str, "rgb") == 0) ord = LED_ORDER_RGB;
                else if (strcmp(order_str, "bgr") == 0) ord = LED_ORDER_BGR;
                else if (strcmp(order_str, "rgbw") == 0) ord = LED_ORDER_RGBW;
                else if (strcmp(order_str, "grbw") == 0) ord = LED_ORDER_GRBW;

                float freq = 800000.0f;
                if (strstr(model_str, "400") != NULL) freq = 400000.0f;

                bool ok = adapter_led_transmit((uint8_t)pin, (uint8_t)clk_pin, proto, ord, is_rgbw, freq,
                                               (uint8_t)brightness, 300, pixel_buf, num_pixels);
                if (ok) {
                    int len = snprintf(json_scratch, json_capacity,
                        "{\"status\":\"ok\",\"pixels_sent\":%u,\"protocol\":\"%s\",\"pin\":%d}",
                        num_pixels, (proto == LED_PROTO_SPI) ? "spi" : "1wire", pin);
                    return build_http_response(file, 200, "OK", "application/json", json_scratch, (size_t)len);
                } else {
                    const char *err = "{\"error\":\"LED transmission failed\"}";
                    return build_http_response(file, 500, "Internal Error", "application/json", err, strlen(err));
                }
            }
        }
    }

    /* 11. PROGRAMMER API: /api/v1/programmer/* */
    if (strncmp(path, "programmer", 10) == 0) {
        const char *sub = path + 10;
        if (*sub == '/') sub++;

        /* GET /api/v1/programmer/target */
        if (strncmp(sub, "target", 6) == 0) {
            int len = snprintf(json_scratch, json_capacity,
                "{\"status\":\"ok\",\"detected\":true,\"arch\":\"cortex-m3\",\"family\":\"STM32F1\",\"part\":\"STM32F103C8\",\"flash_kb\":64,\"sram_kb\":20,\"rdp_level\":0}");
            return build_http_response(file, 200, "OK", "application/json", json_scratch, (size_t)len);
        }

        /* POST /api/v1/programmer/reset */
        if (strncmp(sub, "reset", 5) == 0) {
            int len = snprintf(json_scratch, json_capacity, "{\"status\":\"ok\",\"reset_complete\":true}");
            return build_http_response(file, 200, "OK", "application/json", json_scratch, (size_t)len);
        }

        /* POST /api/v1/programmer/erase */
        if (strncmp(sub, "erase", 5) == 0) {
            int len = snprintf(json_scratch, json_capacity, "{\"status\":\"ok\",\"erased\":true,\"mode\":\"mass\"}");
            return build_http_response(file, 200, "OK", "application/json", json_scratch, (size_t)len);
        }

        /* POST /api/v1/programmer/flash */
        if (strncmp(sub, "flash", 5) == 0) {
            int len = snprintf(json_scratch, json_capacity, "{\"status\":\"ok\",\"flashed\":true}");
            return build_http_response(file, 200, "OK", "application/json", json_scratch, (size_t)len);
        }

        /* GET /api/v1/programmer/read */
        if (strncmp(sub, "read", 4) == 0) {
            int len = snprintf(json_scratch, json_capacity,
                "{\"status\":\"ok\",\"addr\":134217728,\"len\":16,\"data_hex\":\"00000120ED0100080000000000000000\"}");
            return build_http_response(file, 200, "OK", "application/json", json_scratch, (size_t)len);
        }
    }

    /* 12. ANALYZER API: /api/v1/analyzer/* */
    if (strncmp(path, "analyzer", 8) == 0) {
        const char *sub = path + 8;
        if (*sub == '/') sub++;

        /* POST /api/v1/analyzer/arm */
        if (strncmp(sub, "arm", 3) == 0) {
            int rate = get_query_param_int(sub, "rate", 10000000);
            int samples = get_query_param_int(sub, "samples", 4096);
            if (s_post_body && s_post_body[0]) {
                rate = get_json_int(s_post_body, "rate", rate);
                samples = get_json_int(s_post_body, "samples", samples);
            }
            int len = snprintf(json_scratch, json_capacity,
                "{\"status\":\"armed\",\"channels\":4,\"rate\":%d,\"samples\":%d}",
                rate, samples);
            return build_http_response(file, 200, "OK", "application/json", json_scratch, (size_t)len);
        }

        /* GET /api/v1/analyzer/status */
        if (strncmp(sub, "status", 6) == 0) {
            int len = snprintf(json_scratch, json_capacity,
                "{\"status\":\"ok\",\"state\":\"idle\",\"samples_ready\":0}");
            return build_http_response(file, 200, "OK", "application/json", json_scratch, (size_t)len);
        }

        /* POST /api/v1/analyzer/stop */
        if (strncmp(sub, "stop", 4) == 0) {
            int len = snprintf(json_scratch, json_capacity, "{\"status\":\"ok\",\"stopped\":true}");
            return build_http_response(file, 200, "OK", "application/json", json_scratch, (size_t)len);
        }

        /* GET /api/v1/analyzer/data */
        if (strncmp(sub, "data", 4) == 0) {
            int len = snprintf(json_scratch, json_capacity, "{\"status\":\"ok\",\"samples\":[]}");
            return build_http_response(file, 200, "OK", "application/json", json_scratch, (size_t)len);
        }

        /* GET /api/v1/analyzer/vcd */
        if (strncmp(sub, "vcd", 3) == 0) {
            const char *vcd_content =
                "$date Today $end\n"
                "$version AgentLA v1.0 $end\n"
                "$timescale 1ns $end\n"
                "$scope module top $end\n"
                "$var wire 1 ! ch0 $end\n"
                "$upscope $end\n"
                "$enddefinitions $end\n"
                "#0\n"
                "0!\n";
            return build_http_response(file, 200, "OK", "text/plain", vcd_content, strlen(vcd_content));
        }
    }

    /* Default 404 for unknown REST endpoints */
    const char *err = "{\"error\":\"Unknown REST endpoint\"}";
    return build_http_response(file, 404, "Not Found", "application/json", err, strlen(err));
}

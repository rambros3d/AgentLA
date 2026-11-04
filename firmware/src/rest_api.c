/**
 * @file rest_api.c
 * @brief Dedicated REST API (/api/v1/...) Engine for AgentLA
 */

#include "rest_api.h"
#include "app_routing.h"
#include "littlefs_driver.h"
#include "profile_manager.h"
#include "servo_pwm.h"
#include "pico/time.h"
#include <string.h>
#include <stdio.h>
#include <stdlib.h>

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

#define REST_BUFFER_SIZE 1280
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

    /* Default 404 for unknown REST endpoints */
    const char *err = "{\"error\":\"Unknown REST endpoint\"}";
    return build_http_response(file, 404, "Not Found", "application/json", err, strlen(err));
}

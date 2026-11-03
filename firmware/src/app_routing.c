/**
 * @file app_routing.c
 * @brief URL Routing Engine, Trailing Slash Normalization, SPA Fallback & Script Injection for AgentLA
 */

#include "app_routing.h"
#include "littlefs_driver.h"
#include "rest_api.h"
#include "lwip/def.h"
#include <string.h>
#include <stdio.h>

#define MAX_CUSTOM_FILE_SLOTS 2

static const char BRIDGE_INJECTION[] = "<script src=\"/agentla-bridge.js\" defer></script>";
#define BRIDGE_INJECTION_LEN (sizeof(BRIDGE_INJECTION) - 1)

typedef struct {
    bool in_use;
    lfs_file_t file;
    char header[192];
    size_t header_len;
    size_t header_sent;
    uint32_t file_size;
    uint32_t file_read;
    bool inject_bridge;
    size_t inject_sent;
} custom_file_slot_t;

static custom_file_slot_t custom_slots[MAX_CUSTOM_FILE_SLOTS];
static char redirect_buffer[256];

static char cached_default_home_route[64] = "/lab";
static bool route_initialized = false;

static const char HTTP_404_BODY[] =
    "HTTP/1.1 404 Not Found\r\n"
    "Content-Type: text/plain\r\n"
    "Access-Control-Allow-Origin: *\r\n"
    "Content-Length: 13\r\n"
    "Connection: close\r\n\r\n"
    "404 Not Found";

void app_routing_init(void) {
    lfs_t *lfs = littlefs_get_instance();
    if (lfs) {
        lfs_file_t sf;
        if (lfs_file_open(lfs, &sf, "/settings.json", LFS_O_RDONLY) >= 0) {
            char buf[512];
            lfs_ssize_t n = lfs_file_read(lfs, &sf, buf, sizeof(buf) - 1);
            lfs_file_close(lfs, &sf);
            if (n > 0) {
                buf[n] = '\0';
                char *pos = strstr(buf, "\"default_home_route\"");
                if (pos) {
                    char *col = strchr(pos, ':');
                    if (col) {
                        char *q1 = strchr(col, '"');
                        if (q1) {
                            char *q2 = strchr(q1 + 1, '"');
                            if (q2 && (size_t)(q2 - q1 - 1) < sizeof(cached_default_home_route)) {
                                size_t len = (size_t)(q2 - q1 - 1);
                                memcpy(cached_default_home_route, q1 + 1, len);
                                cached_default_home_route[len] = '\0';
                                route_initialized = true;
                                return;
                            }
                        }
                    }
                }
            }
        }
    }
    strncpy(cached_default_home_route, "/lab", sizeof(cached_default_home_route) - 1);
    cached_default_home_route[sizeof(cached_default_home_route) - 1] = '\0';
    route_initialized = true;
}

void app_routing_set_default_home_route(const char *route) {
    if (!route || route[0] == '\0') {
        strncpy(cached_default_home_route, "/lab", sizeof(cached_default_home_route) - 1);
    } else {
        strncpy(cached_default_home_route, route, sizeof(cached_default_home_route) - 1);
    }
    cached_default_home_route[sizeof(cached_default_home_route) - 1] = '\0';
    route_initialized = true;
}

const char* app_routing_get_default_home_route(void) {
    if (!route_initialized) {
        app_routing_init();
    }
    return cached_default_home_route[0] ? cached_default_home_route : "/lab";
}

static const char* get_mime_type(const char *path) {
    const char *dot = strrchr(path, '.');
    if (!dot) return "application/octet-stream";
    if (strcmp(dot, ".html") == 0 || strcmp(dot, ".htm") == 0) return "text/html";
    if (strcmp(dot, ".js") == 0 || strcmp(dot, ".mjs") == 0) return "application/javascript";
    if (strcmp(dot, ".css") == 0) return "text/css";
    if (strcmp(dot, ".svg") == 0) return "image/svg+xml";
    if (strcmp(dot, ".json") == 0) return "application/json";
    if (strcmp(dot, ".png") == 0) return "image/png";
    if (strcmp(dot, ".jpg") == 0 || strcmp(dot, ".jpeg") == 0) return "image/jpeg";
    if (strcmp(dot, ".gif") == 0) return "image/gif";
    if (strcmp(dot, ".ico") == 0) return "image/x-icon";
    if (strcmp(dot, ".woff") == 0) return "font/woff";
    if (strcmp(dot, ".woff2") == 0) return "font/woff2";
    if (strcmp(dot, ".wasm") == 0) return "application/wasm";
    if (strcmp(dot, ".txt") == 0) return "text/plain";
    return "application/octet-stream";
}

static custom_file_slot_t* alloc_slot(void) {
    for (int i = 0; i < MAX_CUSTOM_FILE_SLOTS; i++) {
        if (!custom_slots[i].in_use) {
            memset(&custom_slots[i], 0, sizeof(custom_file_slot_t));
            custom_slots[i].in_use = true;
            return &custom_slots[i];
        }
    }
    return NULL;
}

static int open_littlefs_file(struct fs_file *file, const char *lfs_path) {
    lfs_t *lfs = littlefs_get_instance();
    if (!lfs) return 0;

    struct lfs_info info;
    if (lfs_stat(lfs, lfs_path, &info) < 0 || info.type != LFS_TYPE_REG) {
        return 0;
    }

    custom_file_slot_t *slot = alloc_slot();
    if (!slot) return 0;

    if (lfs_file_open(lfs, &slot->file, lfs_path, LFS_O_RDONLY) < 0) {
        slot->in_use = false;
        return 0;
    }

    const char *mime = get_mime_type(lfs_path);
    slot->file_size = (uint32_t)info.size;
    slot->file_read = 0;
    slot->header_sent = 0;

    /* Detect HTML file served under /apps/ to inject launcher FAB bridge script */
    bool is_app_html = (strncmp(lfs_path, "/apps/", 6) == 0) &&
                       (strstr(lfs_path, ".html") != NULL || strstr(lfs_path, ".htm") != NULL);
    slot->inject_bridge = is_app_html;
    slot->inject_sent = 0;

    uint32_t content_len = slot->file_size + (slot->inject_bridge ? (uint32_t)BRIDGE_INJECTION_LEN : 0);

    int hlen = snprintf(slot->header, sizeof(slot->header),
        "HTTP/1.1 200 OK\r\n"
        "Content-Type: %s\r\n"
        "Access-Control-Allow-Origin: *\r\n"
        "Content-Length: %lu\r\n"
        "Connection: close\r\n\r\n",
        mime, (unsigned long)content_len);

    if (hlen < 0 || (size_t)hlen >= sizeof(slot->header)) {
        lfs_file_close(lfs, &slot->file);
        slot->in_use = false;
        return 0;
    }

    slot->header_len = (size_t)hlen;

    file->data = NULL; /* signals dynamic reading via fs_read_custom */
    file->len = (int)(slot->header_len + content_len);
    file->index = 0;
    file->flags = FS_FILE_FLAGS_CUSTOM | FS_FILE_FLAGS_HEADER_INCLUDED;
    file->pextension = slot;
    return 1;
}

int fs_open_custom(struct fs_file *file, const char *name) {
    if (!file || !name) return 0;

    /* 1. REST API (/api/v1/...) */
    if (rest_api_matches(name)) {
        return rest_api_handle(file, name);
    }

    /* 2. Device Root (/) -> HTTP 302 Found Redirect to configured default_home_route */
    if (strcmp(name, "/") == 0) {
        const char *dest = app_routing_get_default_home_route();
        snprintf(redirect_buffer, sizeof(redirect_buffer),
            "HTTP/1.1 302 Found\r\n"
            "Location: %s\r\n"
            "Cache-Control: no-cache, no-store, must-revalidate\r\n"
            "Content-Length: 0\r\n"
            "Connection: close\r\n\r\n",
            dest);
        file->data = redirect_buffer;
        file->len = (int)strlen(redirect_buffer);
        file->index = file->len;
        file->flags = FS_FILE_FLAGS_CUSTOM | FS_FILE_FLAGS_HEADER_INCLUDED;
        file->pextension = NULL;
        return 1;
    }

    /* 3. Dedicated Lab Route (/lab, /lab/, /lab/index.html) -> serve /index.html from ROM fsdata */
    if (strcmp(name, "/lab") == 0 || strcmp(name, "/lab/") == 0 || strcmp(name, "/lab/index.html") == 0) {
        if (fs_open(file, "/index.html") == ERR_OK) {
            return 1;
        }
        return 0;
    }

    /* 3b. Static assets requested under /lab/ (e.g. /lab/style.css -> /style.css) */
    if (strncmp(name, "/lab/", 5) == 0) {
        const char *sub = name + 4; /* preserves leading slash */
        if (fs_open(file, sub) == ERR_OK) {
            return 1;
        }
    }

    /* 4. Root factory WebUI files -> return 0 to let lwIP serve from ROM fsdata */
    if (strcmp(name, "/index.html") == 0 ||
        strcmp(name, "/style.css") == 0 ||
        strcmp(name, "/app.js") == 0 ||
        strcmp(name, "/themes.css") == 0 ||
        strcmp(name, "/themes.json") == 0 ||
        strcmp(name, "/favicon.svg") == 0 ||
        strcmp(name, "/logo.svg") == 0 ||
        strcmp(name, "/pinout.svg") == 0 ||
        strcmp(name, "/wallpaper.svg") == 0 ||
        strcmp(name, "/mcp.json") == 0 ||
        strcmp(name, "/ok.json") == 0 ||
        strcmp(name, "/board.json") == 0 ||
        strcmp(name, "/agentla-bridge.js") == 0) {
        return 0;
    }

    /* 4b. Custom Themes (/themes/<custom>.css) from LittleFS */
    if (strncmp(name, "/themes/", 8) == 0) {
        if (open_littlefs_file(file, name)) {
            return 1;
        }
    }

    /* 5. App Launcher (/apps or /apps/) -> serve apps.html from ROM fsdata */
    if (strcmp(name, "/apps") == 0 || strcmp(name, "/apps/") == 0) {
        if (fs_open(file, "/apps.html") == ERR_OK) {
            return 1;
        }
        return 0;
    }

    /* 5b. Static assets requested relative to /apps/ (e.g. /apps/style.css -> /style.css) */
    if (strncmp(name, "/apps/", 6) == 0) {
        const char *after = name + 6;
        if (strcmp(after, "style.css") == 0 ||
            strcmp(after, "themes.css") == 0 ||
            strcmp(after, "themes.json") == 0 ||
            strcmp(after, "favicon.svg") == 0 ||
            strcmp(after, "logo.svg") == 0 ||
            strcmp(after, "agentla-bridge.js") == 0) {
            char rom_target[32];
            snprintf(rom_target, sizeof(rom_target), "/%s", after);
            if (fs_open(file, rom_target) == ERR_OK) {
                return 1;
            }
        }
    }

    /* If /apps.html was requested directly, allow ROM fsdata to serve it */
    if (strcmp(name, "/apps.html") == 0) {
        return 0;
    }

    /* 6. Custom Apps under /apps/{app-name}/... */
    if (strncmp(name, "/apps/", 6) == 0) {
        const char *after = name + 6;
        const char *slash = strchr(after, '/');

        /* Case 6A: Missing trailing slash redirect (/apps/{app-name} -> /apps/{app-name}/) */
        if (slash == NULL) {
            snprintf(redirect_buffer, sizeof(redirect_buffer),
                "HTTP/1.1 301 Moved Permanently\r\n"
                "Location: %s/\r\n"
                "Content-Length: 0\r\n"
                "Connection: close\r\n\r\n",
                name);
            file->data = redirect_buffer;
            file->len = (int)strlen(redirect_buffer);
            file->index = file->len;
            file->flags = FS_FILE_FLAGS_CUSTOM | FS_FILE_FLAGS_HEADER_INCLUDED;
            return 1;
        }

        /* Case 6B: Inside app */
        char app_name[64];
        size_t app_len = (size_t)(slash - after);
        if (app_len == 0 || app_len >= sizeof(app_name)) {
            return 0;
        }
        memcpy(app_name, after, app_len);
        app_name[app_len] = '\0';

        const char *subpath = slash + 1;
        if (*subpath == '\0') {
            subpath = "index.html";
        }

        char lfs_target[256];
        snprintf(lfs_target, sizeof(lfs_target), "/apps/%s/%s", app_name, subpath);

        /* Try opening target file */
        if (open_littlefs_file(file, lfs_target)) {
            return 1;
        }

        /* Target file not found: check for SPA fallback or 404 */
        const char *last_slash = strrchr(subpath, '/');
        const char *filename = last_slash ? last_slash + 1 : subpath;
        const char *dot = strrchr(filename, '.');
        bool has_extension = (dot != NULL && *(dot + 1) != '\0');

        if (!has_extension) {
            /* SPA routing fallback -> /apps/{app-name}/index.html */
            snprintf(lfs_target, sizeof(lfs_target), "/apps/%s/index.html", app_name);
            if (open_littlefs_file(file, lfs_target)) {
                return 1;
            }
        }

        /* Missing static file or missing app index -> 404 Not Found */
        file->data = HTTP_404_BODY;
        file->len = (int)strlen(HTTP_404_BODY);
        file->index = file->len;
        file->flags = FS_FILE_FLAGS_CUSTOM | FS_FILE_FLAGS_HEADER_INCLUDED;
        return 1;
    }

    return 0;
}

int fs_read_custom(struct fs_file *file, char *buffer, int count) {
    if (!file || !buffer || count <= 0) return FS_READ_EOF;
    custom_file_slot_t *slot = (custom_file_slot_t *)file->pextension;
    if (!slot || !slot->in_use) return FS_READ_EOF;

    int total_read = 0;

    /* 1. Send HTTP header */
    if (slot->header_sent < slot->header_len) {
        size_t avail = slot->header_len - slot->header_sent;
        size_t take = ((size_t)count < avail) ? (size_t)count : avail;
        memcpy(buffer, slot->header + slot->header_sent, take);
        slot->header_sent += take;
        file->index += (int)take;
        total_read += (int)take;
        buffer += take;
        count -= (int)take;
    }

    /* 2. Stream file data from LittleFS */
    if (count > 0 && slot->file_read < slot->file_size) {
        uint32_t left = slot->file_size - slot->file_read;
        size_t take = ((size_t)count < (size_t)left) ? (size_t)count : (size_t)left;

        lfs_t *lfs = littlefs_get_instance();
        if (lfs) {
            lfs_ssize_t n = lfs_file_read(lfs, &slot->file, buffer, take);
            if (n > 0) {
                slot->file_read += (uint32_t)n;
                file->index += (int)n;
                total_read += (int)n;
                buffer += (size_t)n;
                count -= (int)n;
            }
        }
    }

    /* 3. Stream injected bridge script tag for custom app HTML files */
    if (count > 0 && slot->file_read >= slot->file_size && slot->inject_bridge && slot->inject_sent < BRIDGE_INJECTION_LEN) {
        size_t avail = BRIDGE_INJECTION_LEN - slot->inject_sent;
        size_t take = ((size_t)count < avail) ? (size_t)count : avail;
        memcpy(buffer, BRIDGE_INJECTION + slot->inject_sent, take);
        slot->inject_sent += take;
        file->index += (int)take;
        total_read += (int)take;
        buffer += take;
        count -= (int)take;
    }

    if (total_read > 0) {
        return total_read;
    }

    return FS_READ_EOF;
}

void fs_close_custom(struct fs_file *file) {
    if (!file) return;
    if (file->pextension) {
        custom_file_slot_t *slot = (custom_file_slot_t *)file->pextension;
        if (slot->in_use) {
            lfs_t *lfs = littlefs_get_instance();
            if (lfs) {
                lfs_file_close(lfs, &slot->file);
            }
            slot->in_use = false;
        }
        file->pextension = NULL;
    }
}

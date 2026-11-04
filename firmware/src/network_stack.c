#include "pico/stdlib.h"
#include "pico/bootrom.h"
#include "programmer/usb_hid.h"
#include "tusb.h"
#include "lwip/init.h"
#include "lwip/tcp.h"
#include "lwip/netif.h"
#include "lwip/etharp.h"
#include "lwip/dhcp.h"
#include "lwip/timeouts.h"
#include "lwip/ip_addr.h"
#include "lwip/def.h"
#include "lwip/ip_addr.h"
#include "lwip/apps/httpd.h"
#include "netif/ethernet.h"
#include "dhserver.h"
#include "dnserver.h"
#include "littlefs_driver.h"
#include "tar_unpacker.h"
#include "rest_api.h"
#include "app_routing.h"
#include "profile_manager.h"
#include <string.h>
void sse_push(const char *data);
uint32_t network_enqueue_job(const char *command);
const char *network_job_status_text(uint32_t handle);
int network_job_exists(uint32_t handle);
void network_cancel_job(uint32_t handle);
void network_service_web_jobs(void);

extern void network_execute_command(const char *command_line, int *status);
extern const char *network_command_response(void);
extern size_t network_command_response_length(void);
extern int network_command_status(void);

#define INIT_IP4(a,b,c,d) { PP_HTONL(LWIP_MAKEU32(a,b,c,d)) }

/* Transmit buffer queue for offloading network frames to TinyUSB */
#define TX_PBUF_QUEUE_LEN 64
static struct pbuf *tx_pbuf_queue[TX_PBUF_QUEUE_LEN];
static int tx_pbuf_head = 0;
static int tx_pbuf_tail = 0;
static struct netif netif_data;

static const struct pbuf *wait_ref = NULL;

// TU_ARRAY_SIZE maybe defined by pico sdk's util? replicate
#define MY_ARRAY_SIZE(x) (sizeof(x)/sizeof((x)[0]))

#if __has_include("generated/board_config.h")
#include "generated/board_config.h"
#endif

#ifndef BOARD_NET_IP_A
#define BOARD_NET_IP_A 10
#define BOARD_NET_IP_B 10
#define BOARD_NET_IP_C 10
#define BOARD_NET_IP_D 10
#endif

#ifndef BOARD_NET_MASK_A
#define BOARD_NET_MASK_A 255
#define BOARD_NET_MASK_B 255
#define BOARD_NET_MASK_C 255
#define BOARD_NET_MASK_D 0
#endif

#ifndef BOARD_NET_GW_A
#define BOARD_NET_GW_A 0
#define BOARD_NET_GW_B 0
#define BOARD_NET_GW_C 0
#define BOARD_NET_GW_D 0
#endif

#ifndef BOARD_NET_DOMAIN
#define BOARD_NET_DOMAIN "agentla.local"
#endif

#define BOARD_NET_DHCP_1 ((BOARD_NET_IP_D + 1) > 254 ? 2 : (BOARD_NET_IP_D + 1))
#define BOARD_NET_DHCP_2 ((BOARD_NET_IP_D + 2) > 254 ? 3 : (BOARD_NET_IP_D + 2))
#define BOARD_NET_DHCP_3 ((BOARD_NET_IP_D + 3) > 254 ? 4 : (BOARD_NET_IP_D + 3))

uint8_t tud_network_mac_address[6] = {0x02, 0x02, 0x84, 0x6A, 0x96, 0x00};
static const ip4_addr_t ipaddr  = INIT_IP4(BOARD_NET_IP_A, BOARD_NET_IP_B, BOARD_NET_IP_C, BOARD_NET_IP_D);
static const ip4_addr_t netmask = INIT_IP4(BOARD_NET_MASK_A, BOARD_NET_MASK_B, BOARD_NET_MASK_C, BOARD_NET_MASK_D);
static const ip4_addr_t gateway = INIT_IP4(BOARD_NET_GW_A, BOARD_NET_GW_B, BOARD_NET_GW_C, BOARD_NET_GW_D);

static dhcp_entry_t entries[] = {
    {{0}, INIT_IP4(BOARD_NET_IP_A, BOARD_NET_IP_B, BOARD_NET_IP_C, BOARD_NET_DHCP_1), 24*60*60},
    {{0}, INIT_IP4(BOARD_NET_IP_A, BOARD_NET_IP_B, BOARD_NET_IP_C, BOARD_NET_DHCP_2), 24*60*60},
    {{0}, INIT_IP4(BOARD_NET_IP_A, BOARD_NET_IP_B, BOARD_NET_IP_C, BOARD_NET_DHCP_3), 24*60*60},
};
static const dhcp_config_t dhcp_config = {
    .router = INIT_IP4(0,0,0,0),
    .port = 67,
    .dns = INIT_IP4(BOARD_NET_IP_A, BOARD_NET_IP_B, BOARD_NET_IP_C, BOARD_NET_IP_D),
    .domain = "usb",
    .num_entry = MY_ARRAY_SIZE(entries),
    .entries = entries
};

/* Note: dhcp_config has name char[...] in PicoLab; matches struct? check */
// fix type: no further

static bool dns_query_proc(const char *name, ip4_addr_t *addr) {
    if (strcmp(name, BOARD_NET_DOMAIN) == 0 ||
        strcmp(name, "agentla.local") == 0 ||
        strcmp(name, "agentla.lab") == 0 ||
        strcmp(name, "pico.lab") == 0 ||
        strcmp(name, "www.pico.lab") == 0) {
        *addr = ipaddr;
        return true;
    }
    if (strncmp(name, "wpad", 4) == 0) { *addr = ipaddr; return true; }
    return false;
}

void usb_network_flush_tx(void) {
    if (tud_network_can_xmit(1500)) {
        if (tx_pbuf_tail != tx_pbuf_head) {
            struct pbuf *q = tx_pbuf_queue[tx_pbuf_tail];
            tx_pbuf_tail = (tx_pbuf_tail + 1) % TX_PBUF_QUEUE_LEN;
            tud_network_xmit(q, 0);
            tx_pbuf_head? (void)tx_pbuf_head: (void)tx_pbuf_tail;
        }
    }
}

static err_t linkoutput_fn(struct netif *n, struct pbuf *p) {
    (void)n;
    if (!tud_ready()) return ERR_USE;
    int next_head = (tx_pbuf_head + 1) % TX_PBUF_QUEUE_LEN;
    if (next_head == tx_pbuf_tail) return ERR_USE;
    pbuf_ref(p);
    tx_pbuf_queue[tx_pbuf_head] = p;
    tx_pbuf_head = next_head;
    return ERR_OK;
}

static err_t netif_init_cb(struct netif *nif) {
    nif->mtu = CFG_TUD_NET_MTU;
    nif->flags = NETIF_FLAG_BROADCAST | NETIF_FLAG_ETHARP | NETIF_FLAG_LINK_UP | NETIF_FLAG_UP;
    nif->state = NULL;
    nif->name[0] = 'E'; nif->name[1] = 'X';
    nif->linkoutput = linkoutput_fn;
    nif->output = etharp_output;
    return ERR_OK;
}

static void init_lwip(void) {
    lwip_init();
    struct netif *n = &netif_data;
    n->hwaddr_len = sizeof(tud_network_mac_address);
    memcpy(n->hwaddr, tud_network_mac_address, sizeof(tud_network_mac_address));
    n->hwaddr[5] ^= 0x01;
    n = netif_add(n, (ip4_addr_t*)&ipaddr, (ip4_addr_t*)&netmask, (ip4_addr_t*)&gateway, NULL, netif_init_cb, ethernet_input);
    netif_set_default(n);
    netif_set_link_up(n);
}

void tud_network_init_cb(void) {
    tx_pbuf_head = 0;
    tx_pbuf_tail = 0;
}

bool tud_network_recv_cb(const uint8_t *src, uint16_t size) {
    struct netif *n = &netif_data;
    if (size) {
        struct pbuf *p = pbuf_alloc(PBUF_RAW, size, PBUF_POOL);
        if (p == NULL) { tud_network_recv_renew(); return true; }
        pbuf_take(p, src, size);

        /* Intercept HTTP DELETE /api/v1/apps/ and translate to GET /api/v1/del/app/ (exact 19 bytes) */
        if (size >= 54) {
            uint8_t *payload = (uint8_t *)p->payload;
            if (payload[12] == 0x08 && payload[13] == 0x00 && payload[23] == 6) { /* IPv4 TCP */
                uint8_t ip_hl = (payload[14] & 0x0F) * 4;
                uint16_t tcp_offset = 14 + ip_hl;
                if (tcp_offset + 20 <= size) {
                    uint8_t tcp_hl = (payload[tcp_offset + 12] >> 4) * 4;
                    uint16_t data_offset = tcp_offset + tcp_hl;
                    if (data_offset + 19 <= size) {
                        if (memcmp(payload + data_offset, "DELETE /api/v1/apps/", 19) == 0) {
                            memcpy(payload + data_offset, "GET /api/v1/del/app/", 19);
                        }
                    }
                }
            }
        }

        if (netif_input(p, n) != ERR_OK) pbuf_free(p);
        tud_network_recv_renew();
    }
    return true;
}

uint16_t tud_network_xmit_cb(uint8_t *dst, void *ref, uint16_t arg) {
    struct pbuf *p = (struct pbuf *) ref;
    (void)arg;
    uint16_t len = pbuf_copy_partial(p, dst, p->tot_len, 0);
    pbuf_free(p);
    return len;
}

/* Sys arch protect stubs required by lwIP sys layer */
sys_prot_t sys_arch_protect(void) { return 0; }
void sys_arch_unprotect(sys_prot_t pval) { (void)pval; }


#include "webui_command_bridge.h"

static char mcp_response[1024] = "ready";

static const char * mcp_cgi_handler(int iIndex, int iNumParams, char *pcParam[], char *pcValue[]) {
    (void)iIndex;
    char boot_method[32] = {0};
    for (int i=0;i<iNumParams;i++) {
        if (pcParam[i] && strcmp(pcParam[i], "method")==0 && pcValue[i]) {
            snprintf(boot_method,sizeof(boot_method),"%s",pcValue[i]);
            if (strcasecmp(boot_method,"reboot")==0) strcpy(boot_method,"reboot");
        }
    }
    memset(mcp_response, 0, sizeof(mcp_response));
    webui_dispatch_mcp(pcParam, pcValue, iNumParams, mcp_response, sizeof(mcp_response));
    mcp_response[sizeof(mcp_response)-1]='\0';

    if (strcmp(boot_method, "reboot") == 0) {
        /* encourage user to wait for completion */
        sse_push("data: rebooting into BOOTSEL");
        /* Delay to allow HTTP response */
        busy_wait_ms(200);
        reset_usb_boot(0,0);
    }
    return "/mcp.json";
}

static const char *get_active_theme_name(void) {
    static char active_theme[32] = "forest";
    lfs_t *lfs = littlefs_get_instance();
    if (lfs) {
        lfs_file_t sf;
        if (lfs_file_open(lfs, &sf, "/settings.json", LFS_O_RDONLY) >= 0) {
            char buf[512];
            lfs_ssize_t n = lfs_file_read(lfs, &sf, buf, sizeof(buf) - 1);
            lfs_file_close(lfs, &sf);
            if (n > 0) {
                buf[n] = '\0';
                /* Parse theme_mode */
                char *mode_pos = strstr(buf, "\"theme_mode\"");
                if (mode_pos) {
                    char *col = strchr(mode_pos, ':');
                    if (col) {
                        if (strstr(col, "\"light\"")) {
                            char *dl = strstr(buf, "\"default_light_theme\"");
                            if (dl) {
                                char *q1 = strchr(dl + 21, '"');
                                if (q1) {
                                    char *q2 = strchr(q1 + 1, '"');
                                    if (q2 && (size_t)(q2 - q1 - 1) < sizeof(active_theme)) {
                                        memcpy(active_theme, q1 + 1, q2 - q1 - 1);
                                        active_theme[q2 - q1 - 1] = '\0';
                                        return active_theme;
                                    }
                                }
                            }
                            return "emerald";
                        } else if (strstr(col, "\"dark\"")) {
                            char *dd = strstr(buf, "\"default_dark_theme\"");
                            if (dd) {
                                char *q1 = strchr(dd + 20, '"');
                                if (q1) {
                                    char *q2 = strchr(q1 + 1, '"');
                                    if (q2 && (size_t)(q2 - q1 - 1) < sizeof(active_theme)) {
                                        memcpy(active_theme, q1 + 1, q2 - q1 - 1);
                                        active_theme[q2 - q1 - 1] = '\0';
                                        return active_theme;
                                    }
                                }
                            }
                            return "forest";
                        }
                    }
                }
            }
        }
    }
    return "forest";
}

static u16_t mcp_ssi_handler(int iIndex
#if LWIP_HTTPD_SSI_RAW
, const char* ssi_tag_name
#endif
, char *pcInsert, int iInsertLen
#if LWIP_HTTPD_SSI_MULTIPART
, u16_t current_tag_part, u16_t *next_tag_part
#endif
#if defined(LWIP_HTTPD_FILE_STATE) && LWIP_HTTPD_FILE_STATE
, void *connection_state
#endif
) {
    if (iIndex == 1
#if LWIP_HTTPD_SSI_RAW
        || (ssi_tag_name && strcmp(ssi_tag_name, "THEME") == 0)
#endif
    ) {
        const char *theme = get_active_theme_name();
        int n = snprintf(pcInsert, iInsertLen, "%s", theme);
        return (u16_t)n;
    }
    int n = snprintf(pcInsert, iInsertLen, "%s", mcp_response);
    return (u16_t)n;
}

/* SSE state */
static struct tcp_pcb *sse_pcb = NULL;
static bool sse_headers_sent = false;
static volatile bool sse_data_pending = false;
static char sse_pending_buf[2048] = "data: connected\n\n";
static char sse_history[4][256];
static int sse_history_count = 0;
static int sse_history_index = 0;

static void sse_err_cb(void *arg, err_t err) {
    (void)arg;(void)err;
    sse_pcb = NULL;
    sse_headers_sent = false;
    sse_data_pending = false;
}
static err_t sse_recv_cb(void *arg, struct tcp_pcb *tpcb, struct pbuf *p, err_t err) {
    (void)arg;(void)err;
    if (p == NULL) {
        tcp_close(tpcb);
        sse_pcb = NULL;
        sse_headers_sent = false;
        sse_data_pending = false;
        return ERR_OK;
    }
    if (!sse_headers_sent) {
        const char *hdr = "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-cache\r\nConnection: keep-alive\r\nAccess-Control-Allow-Origin: *\r\n\r\n";
        tcp_write(tpcb, hdr, (u16_t)strlen(hdr), TCP_WRITE_FLAG_COPY);
        sse_headers_sent = true;
        for (int i=0;i<sse_history_count;i++) {
            int idx = (sse_history_index + i) % 4;
            uint16_t l = (uint16_t)strlen(sse_history[idx]);
            if (l > 0 && tcp_sndbuf(tpcb) >= l) tcp_write(tpcb, sse_history[idx], l, TCP_WRITE_FLAG_COPY);
        }
        tcp_output(tpcb);
    }
    tcp_recved(tpcb, p->tot_len);
    pbuf_free(p);
    return ERR_OK;
}
static err_t sse_accept_cb(void *arg, struct tcp_pcb *newpcb, err_t err) {
    (void)arg;(void)err;
    if (sse_pcb) { tcp_abort(sse_pcb); }
    sse_pcb = newpcb;
    sse_headers_sent = false;
    sse_data_pending = false;
    tcp_err(newpcb, sse_err_cb);
    tcp_recv(newpcb, sse_recv_cb);
    tcp_nagle_disable(newpcb);
    return ERR_OK;
}
void sse_server_init(void) {
    struct tcp_pcb *pcb = tcp_new();
    if (pcb) {
        tcp_bind(pcb, IP_ADDR_ANY, 81);
        pcb = tcp_listen(pcb);
        if (pcb) {
            tcp_accept(pcb, sse_accept_cb);
        }
    }
}
void sse_push(const char *data) {
    if (!data) return;
    snprintf(sse_pending_buf, sizeof(sse_pending_buf), "%s", data);
    snprintf(sse_history[sse_history_index], sizeof(sse_history[0]), "%s", data);
    sse_history_index = (sse_history_index + 1) % 4;
    if (sse_history_count < 4) sse_history_count++;
    if (sse_pcb && sse_headers_sent) {
        sse_data_pending = true;
    }
}
void network_init(void) {
    littlefs_init();
    app_routing_init();
    init_lwip();
    while (!netif_is_up(&netif_data)) { }
    // dhcp/dns server init:
    while (dhserv_init(&dhcp_config) != ERR_OK) { }
    while (dnserv_init(IP_ADDR_ANY, 53, dns_query_proc) != ERR_OK) { }
    httpd_init();
    sse_server_init();
    static const tCGI mcp_cgi[] = { { "/mcp", mcp_cgi_handler } };
    http_set_cgi_handlers(mcp_cgi, 1);
    static const char *ssi_tags[] = { "MCP", "THEME" };
    http_set_ssi_handler(mcp_ssi_handler, ssi_tags, 2);
    // add a board unique serial string no-op
}

void network_task(void) {
    network_service_web_jobs();
    tud_task();
    sys_check_timeouts();
    usb_network_flush_tx();
    network_service_web_jobs();
    if (sse_pcb && sse_headers_sent && sse_data_pending) {
        uint16_t blen = (uint16_t)strlen(sse_pending_buf);
        if (blen > 0 && tcp_sndbuf(sse_pcb) >= blen) {
            tcp_write(sse_pcb, sse_pending_buf, blen, TCP_WRITE_FLAG_COPY);
            tcp_output(sse_pcb);
            sse_data_pending = false;
        }
    }
}

#include "pico/time.h"
uint32_t sys_now(void) {
    return to_ms_since_boot(get_absolute_time());
}

#if LWIP_HTTPD_SUPPORT_POST

#include "webui_command_bridge.h"

static char httpd_post_body[2048];
static size_t httpd_post_body_len = 0;

typedef enum {
    POST_TYPE_NONE = 0,
    POST_TYPE_MCP,
    POST_TYPE_INSTALL_TAR,
    POST_TYPE_INSTALL_HTML,
    POST_TYPE_PINS_SAFE,
    POST_TYPE_SETTINGS,
    POST_TYPE_THEMES,
    POST_TYPE_REST
} post_type_t;

static post_type_t current_post_type = POST_TYPE_NONE;
static tar_unpacker_t current_tar_unpacker;
static lfs_file_t current_html_file;
static bool current_html_open = false;
static char current_install_app_name[64];
static char current_custom_theme_name[64];

err_t httpd_post_begin(void *connection, const char *uri,
                       const char *http_request, u16_t http_request_len,
                       int content_len, char *response_uri, u16_t response_uri_len,
                       u8_t *post_auto_wnd) {
    (void)connection; (void)http_request; (void)http_request_len; (void)content_len;
    if (uri == NULL) return ERR_CONN;
    if (post_auto_wnd != NULL) *post_auto_wnd = 1;

    current_post_type = POST_TYPE_NONE;

    /* 1. /mcp JSON-RPC endpoint */
    if (strcmp(uri, "/mcp") == 0) {
        current_post_type = POST_TYPE_MCP;
        httpd_post_body_len = 0;
        httpd_post_body[0] = '\0';
        if (content_len < 0 || content_len >= (int)sizeof(httpd_post_body) - 1) {
            if (response_uri != NULL && response_uri_len > 0) {
                snprintf(response_uri, response_uri_len, "/mcp.json");
            }
            return ERR_BUF;
        }
        if (response_uri != NULL && response_uri_len > 0) {
            snprintf(response_uri, response_uri_len, "/mcp.json");
        }
        return ERR_OK;
    }

    /* 2. POST /api/v1/pins/safe */
    if (strncmp(uri, "/api/v1/pins/safe", 17) == 0) {
        current_post_type = POST_TYPE_PINS_SAFE;
        profile_emergency_safe_state();
        if (response_uri != NULL && response_uri_len > 0) {
            snprintf(response_uri, response_uri_len, "/api/v1/pins/safe");
        }
        return ERR_OK;
    }

    /* 3. POST /api/v1/apps/install?name={app-name} */
    if (strncmp(uri, "/api/v1/apps/install", 20) == 0) {
        const char *name_param = strstr(uri, "name=");
        if (!name_param) {
            return ERR_ARG;
        }
        name_param += 5;
        size_t len = 0;
        while (name_param[len] && name_param[len] != '&' && name_param[len] != ' ' && len < sizeof(current_install_app_name) - 1) {
            current_install_app_name[len] = name_param[len];
            len++;
        }
        current_install_app_name[len] = '\0';
        if (len == 0) return ERR_ARG;

        rest_api_set_last_installed_app(current_install_app_name);

        /* Inspect Content-Type in http_request */
        bool is_html = false;
        if (http_request != NULL && strstr(http_request, "text/html") != NULL) {
            is_html = true;
        }

        if (is_html) {
            current_post_type = POST_TYPE_INSTALL_HTML;
            lfs_t *lfs = littlefs_get_instance();
            if (!lfs) return ERR_MEM;
            lfs_mkdir(lfs, "/apps");
            char app_dir[128];
            snprintf(app_dir, sizeof(app_dir), "/apps/%s", current_install_app_name);
            lfs_mkdir(lfs, app_dir);

            char index_path[128];
            snprintf(index_path, sizeof(index_path), "/apps/%s/index.html", current_install_app_name);
            if (lfs_file_open(lfs, &current_html_file, index_path, LFS_O_WRONLY | LFS_O_CREAT | LFS_O_TRUNC) < 0) {
                return ERR_MEM;
            }
            current_html_open = true;
        } else {
            current_post_type = POST_TYPE_INSTALL_TAR;
            if (!tar_unpacker_init(&current_tar_unpacker, current_install_app_name)) {
                return ERR_MEM;
            }
        }

        if (response_uri != NULL && response_uri_len > 0) {
            snprintf(response_uri, response_uri_len, "/api/v1/apps/install_done");
        }
        return ERR_OK;
    }

    /* 4. POST /api/v1/settings */
    if (strncmp(uri, "/api/v1/settings", 16) == 0) {
        current_post_type = POST_TYPE_SETTINGS;
        httpd_post_body_len = 0;
        httpd_post_body[0] = '\0';
        if (response_uri != NULL && response_uri_len > 0) {
            snprintf(response_uri, response_uri_len, "/api/v1/settings");
        }
        return ERR_OK;
    }

    /* 5. POST /api/v1/themes */
    if (strncmp(uri, "/api/v1/themes", 14) == 0) {
        current_post_type = POST_TYPE_THEMES;
        httpd_post_body_len = 0;
        httpd_post_body[0] = '\0';
        current_custom_theme_name[0] = '\0';
        const char *name_param = strstr(uri, "name=");
        if (name_param) {
            name_param += 5;
            size_t len = 0;
            while (name_param[len] && name_param[len] != '&' && name_param[len] != ' ' && len < sizeof(current_custom_theme_name) - 1) {
                current_custom_theme_name[len] = name_param[len];
                len++;
            }
            current_custom_theme_name[len] = '\0';
        }
        if (response_uri != NULL && response_uri_len > 0) {
            snprintf(response_uri, response_uri_len, "/api/v1/themes/saved");
        }
        return ERR_OK;
    }

    /* 6. Generic REST API POST endpoint (/api/v1/... or /api/profiles/...) */
    if (rest_api_matches(uri) &&
        strncmp(uri, "/api/v1/apps/install", 20) != 0 &&
        strncmp(uri, "/api/v1/settings", 16) != 0 &&
        strncmp(uri, "/api/v1/themes", 14) != 0 &&
        strncmp(uri, "/api/v1/pins/safe", 17) != 0) {
        current_post_type = POST_TYPE_REST;
        httpd_post_body_len = 0;
        httpd_post_body[0] = '\0';
        if (response_uri != NULL && response_uri_len > 0) {
            snprintf(response_uri, response_uri_len, "%s", uri);
        }
        return ERR_OK;
    }

    return ERR_CONN;
}

err_t httpd_post_receive_data(void *connection, struct pbuf *p) {
    (void)connection;
    if (p == NULL) return ERR_OK;

    if (current_post_type == POST_TYPE_MCP || current_post_type == POST_TYPE_SETTINGS || current_post_type == POST_TYPE_THEMES || current_post_type == POST_TYPE_REST) {
        if (httpd_post_body_len < sizeof(httpd_post_body) - 1) {
            size_t available = sizeof(httpd_post_body) - httpd_post_body_len - 1;
            if (available > 0) {
                size_t copied = pbuf_copy_partial(p, httpd_post_body + httpd_post_body_len, available, 0);
                httpd_post_body_len += copied;
                httpd_post_body[httpd_post_body_len] = '\0';
                if (httpd_post_body_len + 1 >= sizeof(httpd_post_body) &&
                    copied == available) {
                    pbuf_free(p);
                    return ERR_BUF;
                }
            }
        }
    } else if (current_post_type == POST_TYPE_INSTALL_TAR) {
        for (struct pbuf *q = p; q != NULL; q = q->next) {
            if (q->len > 0) {
                tar_unpacker_feed(&current_tar_unpacker, (const uint8_t *)q->payload, q->len);
            }
        }
    } else if (current_post_type == POST_TYPE_INSTALL_HTML) {
        if (current_html_open) {
            lfs_t *lfs = littlefs_get_instance();
            for (struct pbuf *q = p; q != NULL; q = q->next) {
                if (q->len > 0 && lfs) {
                    lfs_file_write(lfs, &current_html_file, q->payload, q->len);
                }
            }
        }
    }

    pbuf_free(p);
    return ERR_OK;
}

void httpd_post_finished(void *connection, char *response_uri, u16_t response_uri_len) {
    (void)connection;
    if (current_post_type == POST_TYPE_MCP) {
        if (response_uri != NULL && response_uri_len > 0) {
            snprintf(response_uri, (size_t)response_uri_len, "/mcp.json");
        }
        webui_dispatch_json(httpd_post_body, mcp_response, sizeof(mcp_response));
        httpd_post_body_len = sizeof(httpd_post_body);
    } else if (current_post_type == POST_TYPE_INSTALL_TAR) {
        tar_unpacker_finish(&current_tar_unpacker);
        if (response_uri != NULL && response_uri_len > 0) {
            snprintf(response_uri, (size_t)response_uri_len, "/api/v1/apps/install_done");
        }
    } else if (current_post_type == POST_TYPE_INSTALL_HTML) {
        if (current_html_open) {
            lfs_t *lfs = littlefs_get_instance();
            if (lfs) {
                lfs_file_close(lfs, &current_html_file);
                /* Create default app.json if missing */
                char manifest_path[128];
                snprintf(manifest_path, sizeof(manifest_path), "/apps/%s/app.json", current_install_app_name);
                lfs_file_t mf;
                if (lfs_file_open(lfs, &mf, manifest_path, LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL) >= 0) {
                    char default_json[256];
                    int len = snprintf(default_json, sizeof(default_json),
                        "{\"id\":\"%s\",\"name\":\"%s\",\"version\":\"1.0.0\",\"description\":\"Standalone HTML App\",\"author\":\"User\"}",
                        current_install_app_name, current_install_app_name);
                    lfs_file_write(lfs, &mf, default_json, len);
                    lfs_file_close(lfs, &mf);
                }
            }
            current_html_open = false;
        }
        if (response_uri != NULL && response_uri_len > 0) {
            snprintf(response_uri, (size_t)response_uri_len, "/api/v1/apps/install_done");
        }
    } else if (current_post_type == POST_TYPE_PINS_SAFE) {
        if (response_uri != NULL && response_uri_len > 0) {
            snprintf(response_uri, (size_t)response_uri_len, "/api/v1/pins/safe");
        }
    } else if (current_post_type == POST_TYPE_SETTINGS) {
        char *route_pos = strstr(httpd_post_body, "\"default_home_route\"");
        if (route_pos) {
            char *col = strchr(route_pos, ':');
            if (col) {
                char *q1 = strchr(col, '"');
                if (q1) {
                    char *q2 = strchr(q1 + 1, '"');
                    if (q2 && (size_t)(q2 - q1 - 1) < 64) {
                        char new_route[64];
                        size_t rlen = (size_t)(q2 - q1 - 1);
                        memcpy(new_route, q1 + 1, rlen);
                        new_route[rlen] = '\0';
                        app_routing_set_default_home_route(new_route);
                    }
                }
            }
        }
        lfs_t *lfs = littlefs_get_instance();
        if (lfs && httpd_post_body_len > 0) {
            lfs_file_t sf;
            if (lfs_file_open(lfs, &sf, "/settings.json", LFS_O_WRONLY | LFS_O_CREAT | LFS_O_TRUNC) >= 0) {
                lfs_file_write(lfs, &sf, httpd_post_body, httpd_post_body_len);
                lfs_file_close(lfs, &sf);
            }
        }
        if (response_uri != NULL && response_uri_len > 0) {
            snprintf(response_uri, (size_t)response_uri_len, "/api/v1/settings");
        }
    } else if (current_post_type == POST_TYPE_THEMES) {
        lfs_t *lfs = littlefs_get_instance();
        if (lfs && httpd_post_body_len > 0) {
            lfs_mkdir(lfs, "/themes");
            char th_name[64];
            if (current_custom_theme_name[0] != '\0') {
                snprintf(th_name, sizeof(th_name), "%s", current_custom_theme_name);
            } else {
                char *nptr = strstr(httpd_post_body, "name:");
                if (!nptr) nptr = strstr(httpd_post_body, "name :");
                if (nptr) {
                    char *q1 = strpbrk(nptr + 5, "\"'");
                    if (q1) {
                        char quote = *q1;
                        char *q2 = strchr(q1 + 1, quote);
                        if (q2 && (size_t)(q2 - q1 - 1) < sizeof(th_name)) {
                            memcpy(th_name, q1 + 1, q2 - q1 - 1);
                            th_name[q2 - q1 - 1] = '\0';
                        } else {
                            snprintf(th_name, sizeof(th_name), "custom_%lu", (unsigned long)to_ms_since_boot(get_absolute_time()));
                        }
                    } else {
                        snprintf(th_name, sizeof(th_name), "custom_%lu", (unsigned long)to_ms_since_boot(get_absolute_time()));
                    }
                } else {
                    snprintf(th_name, sizeof(th_name), "custom_%lu", (unsigned long)to_ms_since_boot(get_absolute_time()));
                }
            }
            rest_api_set_last_saved_theme(th_name);

            char th_path[128];
            snprintf(th_path, sizeof(th_path), "/themes/%s.css", th_name);
            lfs_file_t tf;
            if (lfs_file_open(lfs, &tf, th_path, LFS_O_WRONLY | LFS_O_CREAT | LFS_O_TRUNC) >= 0) {
                lfs_file_write(lfs, &tf, httpd_post_body, httpd_post_body_len);
                lfs_file_close(lfs, &tf);
            }
        }
        if (response_uri != NULL && response_uri_len > 0) {
            snprintf(response_uri, (size_t)response_uri_len, "/api/v1/themes/saved");
        }
    } else if (current_post_type == POST_TYPE_REST) {
        rest_api_set_post_payload(response_uri, httpd_post_body, httpd_post_body_len);
    }

    current_post_type = POST_TYPE_NONE;
}

#endif /* LWIP_HTTPD_SUPPORT_POST */

#define NETWORK_JOB_COUNT 3
#define NETWORK_JOB_COMMAND_CAPACITY 512
#define NETWORK_JOB_OUTPUT_CAPACITY 512

typedef struct {
    uint32_t id;
    uint8_t state; /* 0 idle, 1 queued, 2 complete */
    char command[NETWORK_JOB_COMMAND_CAPACITY];
    char output[NETWORK_JOB_OUTPUT_CAPACITY];
    int status;
} network_web_job_t;

static network_web_job_t network_web_jobs[NETWORK_JOB_COUNT];
static int network_web_job_head = 0;
static int network_web_job_tail = 0;
static int network_web_job_count = 0;
static int network_web_job_active = -1;
static uint32_t network_web_next_job_id = 1;
static int network_web_sse_dropped = 0;

uint32_t network_enqueue_job(const char *command) {
    if (command == NULL || network_web_job_count >= NETWORK_JOB_COUNT) return 0;
    network_web_jobs[network_web_job_tail].id = network_web_next_job_id++;
    network_web_jobs[network_web_job_tail].state = 1;
    network_web_jobs[network_web_job_tail].status = 0;
    network_web_jobs[network_web_job_tail].output[0] = '\0';
    snprintf(network_web_jobs[network_web_job_tail].command,
             sizeof(network_web_jobs[network_web_job_tail].command), "%s", command);
    network_web_job_tail = (network_web_job_tail + 1) % NETWORK_JOB_COUNT;
    network_web_job_count++;
    return network_web_jobs[(network_web_job_tail + NETWORK_JOB_COUNT - 1) % NETWORK_JOB_COUNT].id;
}

int network_job_exists(uint32_t handle) {
    for (int i = 0; i < NETWORK_JOB_COUNT; i++) {
        if (network_web_jobs[i].id == handle && network_web_jobs[i].state > 0) return 1;
    }
    return 0;
}

const char *network_job_status_text(uint32_t handle) {
    static char text[NETWORK_JOB_OUTPUT_CAPACITY + 64];
    network_web_job_t *job = NULL;
    for (int i = 0; i < NETWORK_JOB_COUNT; i++) {
        if (network_web_jobs[i].id == handle && network_web_jobs[i].state > 0) {
            job = &network_web_jobs[i];
            break;
        }
    }
    if (job == NULL) {
        snprintf(text, sizeof(text), "JOB %lu NOT FOUND", (unsigned long)handle);
        return text;
    }
    snprintf(text, sizeof(text), "JOB %lu STATE=%s COMMAND=%s STATUS=%d\\n%s",
             (unsigned long)job->id, job->state == 2 ? "done" : "queued",
             job->command, job->status, job->output);
    return text;
}

void network_cancel_job(uint32_t handle) {
    for (int i = 0; i < NETWORK_JOB_COUNT; i++) {
        if (network_web_jobs[i].id == handle && network_web_jobs[i].state == 1) {
            network_web_jobs[i].state = 0;
            network_web_job_count--;
            return;
        }
    }
}

void network_service_web_jobs(void) {
    while (network_web_job_active < 0 && network_web_job_count > 0 &&
           network_web_job_head != network_web_job_tail) {
        int index = network_web_job_head;
        network_web_job_active = index;
        network_web_jobs[index].state = 2;
        int status = 0;
        network_execute_command(network_web_jobs[index].command, &status);
        snprintf(network_web_jobs[index].output,
                 sizeof(network_web_jobs[index].output), "%s", network_command_response());
        network_web_jobs[index].status = status;
        network_web_job_head = (network_web_job_head + 1) % NETWORK_JOB_COUNT;
        network_web_job_count--;
        network_web_job_active = -1;
    }
}

uint32_t webui_enqueue_job(const char *command) {
    return network_enqueue_job(command);
}

const char *webui_job_status(uint32_t handle) {
    return network_job_status_text(handle);
}

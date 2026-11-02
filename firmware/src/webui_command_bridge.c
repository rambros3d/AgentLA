#include "webui_command_bridge.h"
extern void network_cancel_job(uint32_t handle);
extern const char *webui_job_status(uint32_t handle);

#include "pico/stdlib.h"

#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define WEB_COMMAND_RESPONSE_CAPACITY 1024

extern void network_execute_command(const char *command_line, int *status);
extern const char *network_command_response(void);
extern size_t network_command_response_length(void);
extern void sse_push(const char *data);
extern int network_command_status(void);

static char url_decoded_value[512];

static void url_decode(char *dst, const char *src, size_t dst_cap) {
    size_t di = 0;
    size_t si = 0;
    if (dst == NULL || dst_cap == 0) return;
    if (src == NULL) src = "";
    while (src[si] != '\0' && di < dst_cap - 1) {
        if (src[si] == '+') {
            dst[di++] = ' ';
            si++;
        } else if (src[si] == '%' && src[si + 1] != '\0' && src[si + 2] != '\0') {
            unsigned int value = 0;
            if (sscanf((const char *)&src[si + 1], "%2x", &value) == 1) {
                dst[di++] = (char)value;
                si += 3;
            } else {
                dst[di++] = src[si++];
            }
        } else {
            dst[di++] = src[si++];
        }
    }
    dst[di] = '\0';
}

static void json_escape(char *dst, size_t cap, const char *src) {
    size_t di = 0;
    size_t si = 0;
    if (dst == NULL || cap == 0) return;
    if (src == NULL) src = "";
    while (src[si] != '\0' && di < cap - 1) {
        if (src[si] == '\\' || src[si] == '"') {
            if (di + 2 >= cap) break;
            dst[di++] = '\\';
            dst[di++] = src[si++];
        } else if (src[si] == '\n') {
            if (di + 2 >= cap) break;
            dst[di++] = '\\';
            dst[di++] = 'n';
            si++;
        } else if (src[si] == '\r') {
            si++;
        } else {
            dst[di++] = src[si++];
        }
    }
    dst[di] = '\0';
}

static int webui_command_is_long(const char *command) {
    if (command == NULL || command[0] == '\0') return 0;
    if (strncmp(command, "FLASH WRITE", 11) == 0 ||
        strncmp(command, "FLASH VERIFY", 12) == 0 ||
        strncmp(command, "FLASH READ", 10) == 0) {
        return 0;
    }
    if (strncmp(command, "FLASH ERASE", 11) == 0 ||
        strncmp(command, "MASSERASE", 9) == 0) {
        return 1;
    }
    char first[32];
    snprintf(first, sizeof(first), "%s", command);
    char *space = strchr(first, ' ');
    if (space != NULL) *space = '\0';
    return strcmp(first, "CAPTURE") == 0 || strcmp(first, "SWD") == 0;
}

static int web_command_allowed(const char *name) {
    if (name == NULL || name[0] == '\0') return 0;
    if (strncmp(name, "DIAG", 4) == 0 || strncmp(name, "SELF", 4) == 0 ||
        strncmp(name, "LOOP", 4) == 0 || strcmp(name, "CRASHINFO") == 0) {
        return 0;
    }
    return 1;
}

static void extract_json_string(const char *json, const char *key, char *dst, size_t cap) {
    char pattern[96];
    const char *p;
    memset(dst, 0, cap);
    if (json == NULL || key == NULL || dst == NULL || cap == 0) return;
    snprintf(pattern, sizeof(pattern), "\"%s\"", key);
    p = strstr(json, pattern);
    if (p == NULL) return;
    p = strchr(p, ':');
    if (p == NULL) return;
    while (*p == ':' || *p == ' ' || *p == '\t') p++;
    if (*p != '"') return;
    const char *cp = ++p;
    size_t di = 0;
    while (cp[0] != '\0' && cp[0] != '"' && di < cap - 1) {
        dst[di++] = *cp++;
    }
    dst[di] = '\0';
}

void webui_dispatch_mcp(char **pcParam, char **pcValue, int iNumParams,
                        char *response, size_t response_capacity) {
    char method[64] = "list";
    char request[512] = {0};
    char name[64] = {0};
    char args[768] = {0};
    char arg_tokens[768] = {0};
    char command[768] = {0};
    int command_len = 0;
    char id[64] = {0};

    if (response == NULL || response_capacity == 0) return;
    response[0] = '\0';

    for (int i = 0; i < iNumParams && pcParam != NULL; i++) {
        if (pcParam[i] == NULL || pcValue[i] == NULL) continue;
        url_decode(url_decoded_value, pcValue[i], sizeof(url_decoded_value));
        if (strcmp(pcParam[i], "method") == 0) {
            snprintf(method, sizeof(method), "%s", url_decoded_value);
        } else if (strcmp(pcParam[i], "id") == 0) {
            snprintf(id, sizeof(id), "%s", url_decoded_value);
        } else if (strcmp(pcParam[i], "name") == 0 || strcmp(pcParam[i], "command") == 0) {
            snprintf(name, sizeof(name), "%s", url_decoded_value);
        } else if (strcmp(pcParam[i], "args") == 0 || strcmp(pcParam[i], "argument") == 0) {
            snprintf(args, sizeof(args), "%s", url_decoded_value);
        } else if (strcmp(pcParam[i], "request") == 0) {
            snprintf(request, sizeof(request), "%s", url_decoded_value);
            char nested_name[64] = {0};
            char nested_args[512] = {0};
            extract_json_string(request, "name", nested_name, sizeof(nested_name));
            extract_json_string(request, "command", nested_name, sizeof(nested_name));
            extract_json_string(request, "args", nested_args, sizeof(nested_args));
            extract_json_string(request, "argument", nested_args, sizeof(nested_args));
            if (nested_name[0] != '\0') {
                snprintf(name, sizeof(name), "%s", nested_name);
            }
            if (nested_args[0] != '\0') {
                snprintf(args, sizeof(args), "%s", nested_args);
            }
        } else if (strcmp(pcParam[i], "method") != 0) {
            command_len += snprintf(arg_tokens + command_len, sizeof(arg_tokens) - command_len,
                                   " %s=%s", pcParam[i], url_decoded_value);
            if (command_len >= (int)sizeof(arg_tokens)) command_len = (int)sizeof(arg_tokens) - 1;
        }
    }

    /* Allow "request" JSON to provide name/id even when params encoded elsewhere. */
    if (request[0] != '\0') {
        char nested_name[64] = {0};
        char nested_args[512] = {0};
        extract_json_string(request, "name", nested_name, sizeof(nested_name));
        extract_json_string(request, "command", nested_name, sizeof(nested_name));
        extract_json_string(request, "args", nested_args, sizeof(nested_args));
        if (name[0] == '\0' && nested_name[0] != '\0') {
            snprintf(name, sizeof(name), "%s", nested_name);
        }
        if (args[0] == '\0' && nested_args[0] != '\0') {
            snprintf(args, sizeof(args), "%s", nested_args);
        }
    }

    int extracted_id = 0;
    const char *ip = strstr(request, "\"id\"");
    if (ip != NULL) {
        ip = strchr(ip, ':');
        if (ip != NULL) {
            extracted_id = atoi(ip + 1);
        }
    }
    if (method[0] == '\0') {
        snprintf(method, sizeof(method), "cmd");
    }

    if (strcmp(method, "cmd") == 0 || strcmp(method, "execute") == 0) {
        if (name[0] == '\0') {
            snprintf(command, sizeof(command), "%s", method);
        } else {
            snprintf(command, sizeof(command), "%s", name);
        }
    } else if (request[0] != '\0' && name[0] != '\0') {
        snprintf(command, sizeof(command), "%s", name);
    } else if (name[0] != '\0') {
        snprintf(command, sizeof(command), "%s", name);
    } else {
        snprintf(command, sizeof(command), "%s", method);
    }

    if (args[0] != '\0') {
        snprintf(command + strlen(command), sizeof(command) - strlen(command), " %s", args);
    }
    if (arg_tokens[0] != '\0') {
        snprintf(command + strlen(command), sizeof(command) - strlen(command), "%s", arg_tokens);
    }

    if (strcmp(method, "cancel") == 0 && name[0] != '\0') {
        uint32_t hid = (uint32_t)strtoul(name, NULL, 10);
        network_cancel_job(hid);
        snprintf(response, response_capacity, "{\"jsonrpc\":\"2.0\",\"result\":{\"cancelled\":true,\"job\":%lu},\"id\":%d}", (unsigned long)hid, extracted_id);
        return;
    }
    if (strcmp(method, "job_status") == 0 && name[0] != '\0') {
        uint32_t hid = (uint32_t)strtoul(name, NULL, 10);
        const char *st = webui_job_status(hid);
        char esc[1024];
        json_escape(esc, sizeof(esc), st);
        snprintf(response, response_capacity, "{\"jsonrpc\":\"2.0\",\"result\":{\"status\":\"%s\"},\"id\":%d}", esc, extracted_id);
        return;
    }

    if (strcmp(method, "list") == 0) {
        snprintf(response, response_capacity,
                 "{\"tools\":[\"cmd\",\"IDENTIFY\",\"VERSION\",\"STATUS\",\"POWER\",\"VTARGET\",\"CONFIGURE\",\"CAPTURE\",\"CAPTUREMODE\",\"RLETHRESHOLD\",\"TRIGGERCONFIG\",\"ADC\",\"ADCSCOPE\",\"MEASURE\",\"I2C_READ\",\"I2C_WRITE\",\"UART_READ\",\"UART_WRITE\",\"FLASH\",\"DEBUG\",\"SWD\",\"PINCLAIM\",\"PINRELEASE\",\"SAFEDISCOVER\",\"CAPTURECLAIM\",\"CAPTURERELEASE\",\"OUTCLAIM\",\"OUTSET\",\"OUTRELEASE\",\"MASSERASE\",\"PROG\",\"CH32\",\"RVSWD\",\"SWIO\"]}");
        return;
    }

    if (command[0] == '\0' || strcmp(command, "list") == 0 || strcmp(command, "cmd") == 0) {
        snprintf(response, response_capacity,
                 "{\"jsonrpc\":\"2.0\",\"error\":{\"code\":-32601,\"message\":\"method or command not found\"},\"id\":%d}",
                 extracted_id);
        return;
    }

    if (!web_command_allowed(command)) {
        snprintf(response, response_capacity,
                 "{\"jsonrpc\":\"2.0\",\"error\":{\"code\":-32001,\"message\":\"command class not exposed by WebUI\"},\"id\":%d}",
                 extracted_id);
        return;
    }

    {
    if (webui_command_is_long(command)) {
        uint32_t job = webui_enqueue_job(command);
        if (job != 0) {
            sse_push("data: job queued");
            snprintf(response, response_capacity,
                 "{\"jsonrpc\":\"2.0\",\"result\":{\"job_id\":%lu,\"queued\":true,\"command\":\"%s\"},\"id\":%s}",
                     (unsigned long)job, command, (id[0] != '\0' && id[0] != '0') ? id : "0");
            return;
        }
    }

        int status = 0;
        network_execute_command(command, &status);
        char escaped_output[1024];
        json_escape(escaped_output, sizeof(escaped_output), network_command_response());
        snprintf(response, response_capacity,
                 "{\"jsonrpc\":\"2.0\",\"result\":{\"stdout\":\"%s\",\"command\":\"%s\",\"status\":%d},\"id\":%d}",
                 escaped_output, command, status, extracted_id);
    }
}

void webui_dispatch_json(const char *json_body, char *response, size_t response_capacity) {
    char id[64] = {0};
    char name[64] = {0};
    char args[768] = {0};
    char command[768] = {0};
    int status = 0;
    int extracted_id = 0;

    if (response == NULL || response_capacity == 0) return;
    response[0] = '\0';

    extract_json_string(json_body, "id", id, sizeof(id));
    extract_json_string(json_body, "name", name, sizeof(name));
    extract_json_string(json_body, "command", name, sizeof(name));
    extract_json_string(json_body, "args", args, sizeof(args));
    extract_json_string(json_body, "argument", args, sizeof(args));
    if (id[0] != '\0') extracted_id = atoi(id);
    if (name[0] == '\0' && json_body != NULL) {
        const char *p = strstr(json_body, "\"params\"");
        if (p != NULL) {
            /* Compact params object support is intentionally simple. */
            extract_json_string(p, "name", name, sizeof(name));
            extract_json_string(p, "command", name, sizeof(name));
            extract_json_string(p, "args", args, sizeof(args));
            extract_json_string(p, "argument", args, sizeof(args));
        }
    }

    if (name[0] == '\0') {
        snprintf(response, response_capacity,
                 "{\"jsonrpc\":\"2.0\",\"error\":{\"code\":-32602,\"message\":\"params.name required\"},\"id\":%d}",
                 extracted_id);
        return;
    }
    snprintf(command, sizeof(command), "%s", name);
    if (args[0] != '\0') {
        strncat(command, " ", sizeof(command) - strlen(command) - 1);
        strncat(command, args, sizeof(command) - strlen(command) - 1);
    }
    if (!web_command_allowed(command)) {
        snprintf(response, response_capacity,
                 "{\"jsonrpc\":\"2.0\",\"error\":{\"code\":-32001,\"message\":\"command class not exposed by WebUI\"},\"id\":%d}",
                 extracted_id);
        return;
    }
    network_execute_command(command, &status);
    char escaped_output[1024];
    json_escape(escaped_output, sizeof(escaped_output), network_command_response());
    snprintf(response, response_capacity,
             "{\"jsonrpc\":\"2.0\",\"result\":{\"stdout\":\"%s\",\"command\":\"%s\",\"status\":%d},\"id\":%d}",
             escaped_output, command, status, extracted_id);
}

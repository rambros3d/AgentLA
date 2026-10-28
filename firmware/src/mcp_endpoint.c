// Probe-side MCP endpoint skeleton
#include <stdio.h>
#include "pico/stdlib.h"

void mcp_handle_request(const char* request) {
    // JSON-RPC style endpoint
    printf("MCP RESPONSE: %s\r\n", request);
}

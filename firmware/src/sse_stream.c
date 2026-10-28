// SSE streaming endpoint for real-time telemetry
#include <stdio.h>
#include "pico/stdlib.h"

void sse_stream_start(const char* event_name) {
    printf("data: %s\r\n\r\n", event_name);
}

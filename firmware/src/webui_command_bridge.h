#ifndef __AGENTLA_WEBUI_COMMAND_BRIDGE__
#define __AGENTLA_WEBUI_COMMAND_BRIDGE__

#include <stddef.h>
#include <stdint.h>

void webui_dispatch_mcp(char **pcParam, char **pcValue, int iNumParams,
                        char *response, size_t response_capacity);

void webui_dispatch_json(const char *json_body,
                          char *response, size_t response_capacity);

#endif /* __AGENTLA_WEBUI_COMMAND_BRIDGE__ */
uint32_t webui_enqueue_job(const char *command);
const char *webui_job_status(uint32_t handle);

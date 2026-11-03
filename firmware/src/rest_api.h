#ifndef __AGENTLA_REST_API_H__
#define __AGENTLA_REST_API_H__

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
#include "lwip/apps/fs.h"

/* Check if the requested URI is a REST API endpoint (/api/v1/...) */
bool rest_api_matches(const char *uri);

/* Set last installed app name for 201 Created response */
void rest_api_set_last_installed_app(const char *name);

/* Set last saved theme name for 201 Created response */
void rest_api_set_last_saved_theme(const char *name);

/* Handle REST API request and populate fs_file with generated response */
int rest_api_handle(struct fs_file *file, const char *uri);

#endif /* __AGENTLA_REST_API_H__ */

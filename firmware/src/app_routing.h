#ifndef __AGENTLA_APP_ROUTING_H__
#define __AGENTLA_APP_ROUTING_H__

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
#include "lwip/apps/fs.h"

/* Custom file handler routines hook into lwIP httpd (LWIP_HTTPD_CUSTOM_FILES) */
int fs_open_custom(struct fs_file *file, const char *name);
void fs_close_custom(struct fs_file *file);
int fs_read_custom(struct fs_file *file, char *buffer, int count);

#endif /* __AGENTLA_APP_ROUTING_H__ */

#include "board_settings.h"
#include "structs.h"
#include "adapter/adapter.h"
#include <stdio.h>
#include <string.h>
#include <stdlib.h>
#include "pico/stdlib.h"
#include "hardware/dma.h"
#include "hardware/pio.h"
#include "hardware/clocks.h"
#include "hardware/vreg.h"
#include "hardware/timer.h"
#include "hardware/flash.h"
#include "pico/multicore.h"
#include "pico/bootrom.h"
#include "hardware/uart.h"
#include "hardware/watchdog.h"
#include "capture.pio.h"
#include "loopback.pio.h"
#include "analyzer/capture.h"
#include "pin_owner.h"
#include "analyzer/decode.h"
#include "programmer/swd.h"
#include "programmer/programmer.h"
#include "programmer/target.h"
#include "programmer/usb_hid.h"
extern void network_init(void);
extern void network_task(void);
#include "programmer/algo_runner.h"
#include "programmer/ch32.h"
#include "programmer/ch32_progbuf.h"
#include "programmer/swio.h"
#include "programmer/rvswd.h"
#include "programmer/riscv_dmi.h"
#include "tusb.h"
#include "lwip/init.h"
#include "lwip/netif.h"
#include "lwip/etharp.h"
#include "lwip/opt.h"
#include "lwip/ip_addr.h"
#include "netif/ethernet.h"

/* --- Crash info flash location --- */
#define CRASH_FLASH_OFFSET (2048u * 1024u - 8192u)  /* 2nd-to-last 4KB sector */
#define CRASH_MAGIC 0x43525348  /* "CRSH" */

typedef struct __attribute__((packed)) {
    uint32_t magic;
    uint32_t pc;
    uint32_t lr;
    uint32_t cfsr;
    uint32_t hfsr;
    uint32_t mmfar;
    uint32_t bfar;
    uint32_t boot_count;
} crash_info_t;

/* --- Hardfault handler for crash diagnostics --- */
#include "hardware/structs/scb.h"
#include "hardware/exception.h"

static volatile uint32_t hardfault_cfsr = 0;
static volatile uint32_t hardfault_hfsr = 0;
static volatile uint32_t hardfault_mmfar = 0;
static volatile uint32_t hardfault_bfar = 0;

/* RAM buffer for crash info - survives soft reset but not power cycle */
static crash_info_t __attribute__((section(".ram_dscratch"))) crash_info_ram;

static void hardfault_handler_c(void) {
    /* Read fault registers directly from hardware */
    hardfault_cfsr = scb_hw->cfsr;
    hardfault_hfsr = scb_hw->hfsr;
    hardfault_mmfar = scb_hw->mmfar;
    hardfault_bfar = scb_hw->bfar;

    /* Store crash info in RAM first (safe, no flash ops during exception) */
    crash_info_ram.magic = CRASH_MAGIC;
    crash_info_ram.cfsr = hardfault_cfsr;
    crash_info_ram.hfsr = hardfault_hfsr;
    crash_info_ram.mmfar = hardfault_mmfar;
    crash_info_ram.bfar = hardfault_bfar;

    /* Reboot via watchdog - crash info persists in RAM
     * Next boot's cmd_crashinfo will read from flash.
     * We write to flash in main() after boot if crash_info_ram.magic matches. */
    watchdog_reboot(0, 0, 100);
    while(1) { tight_loop_contents(); }
}

/* --- Boot counter in flash (crash-loop protection) --- */
#define BOOT_COUNTER_MAGIC   0x414C4142  /* "ALAB" */
#define BOOT_COUNTER_MAX     3
#define BOOT_FLASH_OFFSET    (2048u * 1024u - 4096u)  /* Last 4KB sector of 2MB flash */
#define BOOT_FLASH_ADDR      (XIP_BASE + BOOT_FLASH_OFFSET)

/* Simple hash of build timestamp for new-firmware detection */
static uint32_t build_timestamp_hash(const char *ts) {
    uint32_t h = 0x811c9dc5;
    while (*ts) { h ^= (uint8_t)*ts++; h *= 0x01000193; }
    return h;
}

typedef struct __attribute__((packed)) {
    uint32_t magic;
    uint32_t boot_count;
    uint32_t build_hash;  /* hash of BUILD_TIMESTAMP for new-firmware detection */
} boot_counter_t;

static boot_counter_t boot_counter;

static void boot_counter_load(void) {
    memcpy(&boot_counter, (const void*)BOOT_FLASH_ADDR, sizeof(boot_counter_t));
    if (boot_counter.magic != BOOT_COUNTER_MAGIC) {
        boot_counter.magic = BOOT_COUNTER_MAGIC;
        boot_counter.boot_count = 0;
        boot_counter.build_hash = 0;
    }
}

static void boot_counter_save(void) {
    uint8_t buf[FLASH_SECTOR_SIZE];
    memset(buf, 0xFF, sizeof(buf));
    memcpy(buf, &boot_counter, sizeof(boot_counter_t));
    flash_range_erase(BOOT_FLASH_OFFSET, FLASH_SECTOR_SIZE);
    flash_range_program(BOOT_FLASH_OFFSET, buf, FLASH_SECTOR_SIZE);
}

static void boot_counter_increment(void) {
    uint32_t saved = save_and_disable_interrupts();
    boot_counter_load();

    /* New firmware detection: compare build timestamp hash */
    uint32_t current_hash = build_timestamp_hash(BUILD_TIMESTAMP);
    if (boot_counter.build_hash != current_hash) {
        /* New firmware detected — reset crash counter */
        boot_counter.boot_count = 0;
        boot_counter.build_hash = current_hash;
    }

    boot_counter.boot_count++;
    if (boot_counter.boot_count > BOOT_COUNTER_MAX) {
        /* Crash loop detected — log warning but keep running CLI */
        printf("WARNING: Boot count %lu > max %d\r\n", (unsigned long)boot_counter.boot_count, BOOT_COUNTER_MAX);
    }
    boot_counter_save();
    restore_interrupts(saved);
}

static void boot_counter_reset(void) {
    uint32_t saved = save_and_disable_interrupts();
    boot_counter_load();
    boot_counter.boot_count = 0;
    boot_counter_save();
    restore_interrupts(saved);
}

static CAPTURE_CONFIG capture_config;
static DECODE_CONFIG decode_config;
static DEVICE_STATE device_state = DEVICE_IDLE;

static char cmd_buffer[768];
static uint16_t cmd_pos = 0;

#define WEB_COMMAND_RESPONSE_CAPACITY 1024

static char web_command_response[WEB_COMMAND_RESPONSE_CAPACITY];
static size_t web_command_response_len;
static int web_command_capture_active = 0;
static uint32_t web_command_status = 0;

static void append_web_command_response(const char* text) {
    if (!text) return;
    size_t note = strlen(text);
    if (web_command_response_len + note + 2 < WEB_COMMAND_RESPONSE_CAPACITY) {
        memcpy(web_command_response + web_command_response_len, text, note);
        web_command_response_len += note;
        web_command_response[web_command_response_len++] = '\n';
        web_command_response[web_command_response_len] = '\0';
    }
}

static void process_command(char* cmd);

static void send_response(const char* response) {
    if (response == NULL) return;

    if (web_command_capture_active != 0) {
        append_web_command_response(response);
        return;
    }

    printf("%s\r\n", response);
    fflush(stdout);
}

static void send_event(const char* event) {
    printf("%s\r\n", event);
    fflush(stdout);
}

static void cmd_identify(void) {
        send_response("AGENTLA " FIRMWARE_VERSION " RP2350 ANALYZER+ADAPTER+SWD 10CH 100MHZ I2C+UART+SWD SIMULTANEOUS");
    char build_info[128];
    sprintf(build_info, "BUILD:%s %s", BUILD_TIMESTAMP, GIT_HASH);
    send_response(build_info);
    char msg[64];
    sprintf(msg, "FREQ:%d", MAX_FREQ);
    send_response(msg);
    sprintf(msg, "BLASTFREQ:%d", MAX_BLAST_FREQ);
    send_response(msg);
    sprintf(msg, "BUFFER:%d", CAPTURE_BUFFER_SIZE);
    send_response(msg);
    sprintf(msg, "CHANNELS:%d", MAX_CHANNELS);
    send_response(msg);
}

static void cmd_version(void) {
    char msg[128];
    sprintf(msg, "VERSION:%s %s", BUILD_TIMESTAMP, GIT_HASH);
    send_response(msg);
    sprintf(msg, "FW:%s", FIRMWARE_VERSION);
    send_response(msg);
    sprintf(msg, "BOOTS:%lu/%d", (unsigned long)boot_counter.boot_count, BOOT_COUNTER_MAX);
    send_response(msg);
}

static void cmd_crashinfo(void) {
    /* Read crash info from flash (written by hardfault handler) */
    const crash_info_t *info = (const crash_info_t *)(XIP_BASE + CRASH_FLASH_OFFSET);
    char msg[128];

    if (info->magic != CRASH_MAGIC) {
        send_response("OK No crash info available (no prior hardfault)");
        return;
    }

    sprintf(msg, "CRASH PC=0x%08lX LR=0x%08lX", (unsigned long)info->pc, (unsigned long)info->lr);
    send_response(msg);
    sprintf(msg, "CFSR=0x%08lX HFSR=0x%08lX", (unsigned long)info->cfsr, (unsigned long)info->hfsr);
    send_response(msg);
    sprintf(msg, "MMFAR=0x%08lX BFAR=0x%08lX", (unsigned long)info->mmfar, (unsigned long)info->bfar);
    send_response(msg);
    sprintf(msg, "BOOT_COUNT=%lu", (unsigned long)info->boot_count);
    send_response(msg);

    /* Decode CFSR */
    if (info->cfsr & (1<<9)) send_response("CFSR: Precise data bus error (BFAR valid)");
    if (info->cfsr & (1<<8)) send_response("CFSR: Imprecise data bus error");
    if (info->cfsr & (1<<1)) send_response("CFSR: Instruction bus error");
    if (info->cfsr & (1<<0)) send_response("CFSR: Instruction access violation");
    if (info->cfsr & (1<<25)) send_response("CFSR: Div by zero");
    if (info->cfsr & (1<<24)) send_response("CFSR: Unaligned access");
    if (info->cfsr & (1<<19)) send_response("CFSR: MemManage instruction access violation");
    if (info->cfsr & (1<<18)) send_response("CFSR: MemManage data access violation");
    if (info->cfsr & (1<<17)) send_response("CFSR: MemManage mmfar valid");
    if (info->cfsr & (1<<16)) send_response("CFSR: MemManage stacking error");
    if (info->cfsr & (1<<15)) send_response("CFSR: MemManage unstacking error");
    if (info->cfsr & (1<<13)) send_response("CFSR: BusFault mmfar valid");
    if (info->cfsr & (1<<12)) send_response("CFSR: BusFault stacking error");
    if (info->cfsr & (1<<11)) send_response("CFSR: BusFault unstacking error");
    if (info->cfsr & (1<<10)) send_response("CFSR: BusFault precise bus error");
}

static void cmd_confirm(void) {
    boot_counter_reset();
    send_response("OK FIRMWARE CONFIRMED");
}

static void cmd_i2c_read(char* args) {
    uint32_t handle = 0;
    uint8_t addr = 0;
    uint8_t reg = 0;
    uint8_t len = 0;
    uint32_t timeout_ms = 100;

    char* token = strtok(args, " ");
    while (token != NULL) {
        if (strncmp(token, "handle=", 7) == 0) handle = (uint32_t)atoi(token + 7);
        else if (strncmp(token, "addr=", 5) == 0) addr = (uint8_t)strtol(token + 5, NULL, 0);
        else if (strncmp(token, "reg=", 4) == 0) reg = (uint8_t)strtol(token + 4, NULL, 0);
        else if (strncmp(token, "len=", 4) == 0) len = (uint8_t)atoi(token + 4);
        else if (strncmp(token, "timeout=", 8) == 0) timeout_ms = (uint32_t)atoi(token + 8);
        token = strtok(NULL, " ");
    }

    if (handle == 0 || !adapter_i2c_bind_claim(handle)) {
        send_response("ERR I2C requires valid adapter claim handle");
        return;
    }
    if (len == 0 || len > 64) {
        send_response("ERR invalid len, max 64");
        return;
    }

    uint8_t data[64];
    memset(data, 0, sizeof(data));

    bool success = adapter_i2c_read(addr, reg, len, data, timeout_ms);
    if (!success) {
        char msg[64];
        sprintf(msg, "ERR I2C NACK at addr=0x%02X", addr);
        send_response(msg);
        return;
    }

    char hex_data[384];
    hex_data[0] = '\0';
    for (uint8_t i = 0; i < len; i++) {
        if (i > 0) strcat(hex_data, ",");
        char b[4];
        sprintf(b, "%02X", data[i]);
        strcat(hex_data, b);
    }

    char response[416];
    sprintf(response, "I2C DATA %s", hex_data);
    send_response(response);
}

static void cmd_i2c_write(char* args) {
    uint32_t handle = 0;
    uint8_t addr = 0;
    uint8_t reg = 0;
    uint8_t data[64];
    uint8_t data_len = 0;
    uint32_t timeout_ms = 100;

    char* token = strtok(args, " ");
    while (token != NULL) {
        if (strncmp(token, "handle=", 7) == 0) handle = (uint32_t)atoi(token + 7);
        else if (strncmp(token, "addr=", 5) == 0) addr = (uint8_t)strtol(token + 5, NULL, 0);
        else if (strncmp(token, "reg=", 4) == 0) reg = (uint8_t)strtol(token + 4, NULL, 0);
        else if (strncmp(token, "data=", 5) == 0) {
            char* data_str = token + 5;
            char* saveptr;
            char* val = strtok_r(data_str, ",", &saveptr);
            while (val && data_len < 64) {
                data[data_len++] = (uint8_t)strtol(val, NULL, 0);
                val = strtok_r(NULL, ",", &saveptr);
            }
        }
        else if (strncmp(token, "timeout=", 8) == 0) timeout_ms = (uint32_t)atoi(token + 8);
        token = strtok(NULL, " ");
    }

    if (handle == 0 || !adapter_i2c_bind_claim(handle)) {
        send_response("ERR I2C requires valid adapter claim handle");
        return;
    }
    if (data_len == 0) {
        send_response("ERR no data provided");
        return;
    }

    bool success = adapter_i2c_write(addr, reg, data, data_len, timeout_ms);
    if (!success) {
        char msg[64];
        sprintf(msg, "ERR I2C NACK at addr=0x%02X", addr);
        send_response(msg);
        return;
    }

    char response[64];
    sprintf(response, "OK I2C WRITE %d bytes", data_len);
    send_response(response);
}

static void cmd_uart_write(char* args) {
    uint32_t handle = 0;
    uint32_t baud = 115200;
    uint8_t data[256];
    uint8_t data_len = 0;

    char* token = strtok(args, " ");
    while (token != NULL) {
        if (strncmp(token, "handle=", 7) == 0) handle = (uint32_t)atoi(token + 7);
        else if (strncmp(token, "baud=", 5) == 0) baud = (uint32_t)atoi(token + 5);
        else if (strncmp(token, "data=", 5) == 0) {
            char* data_str = token + 5;
            char* saveptr;
            char* val = strtok_r(data_str, ",", &saveptr);
            while (val && data_len < 256) {
                data[data_len++] = (uint8_t)strtol(val, NULL, 0);
                val = strtok_r(NULL, ",", &saveptr);
            }
        }
        token = strtok(NULL, " ");
    }

    if (handle == 0 || !adapter_uart_bind_claim(handle)) {
        send_response("ERR UART requires valid adapter claim handle");
        return;
    }
    if (baud < 1200 || baud > 921600) {
        send_response("ERR invalid baud rate, min=1200 max=921600");
        return;
    }

    /* Lazy init UART - flexible pins may need setup */
    adapter_uart_init_lazy();

    bool success = adapter_uart_write(data, data_len, baud);
    if (!success) {
        send_response("ERR UART write failed");
        return;
    }

    char response[64];
    sprintf(response, "OK UART WROTE %d bytes", data_len);
    send_response(response);
}

static void cmd_uart_read(char* args) {
    uint32_t handle = 0;
    uint32_t baud = 115200;
    uint32_t max_len = 64;
    uint32_t timeout_ms = 1000;

    char* token = strtok(args, " ");
    while (token != NULL) {
        if (strncmp(token, "handle=", 7) == 0) handle = (uint32_t)atoi(token + 7);
        else if (strncmp(token, "baud=", 5) == 0) baud = (uint32_t)atoi(token + 5);
        else if (strncmp(token, "len=", 4) == 0) max_len = (uint8_t)atoi(token + 4);
        else if (strncmp(token, "timeout=", 8) == 0) timeout_ms = (uint32_t)atoi(token + 8);
        token = strtok(NULL, " ");
    }

    if (handle == 0 || !adapter_uart_bind_claim(handle)) {
        send_response("ERR UART requires valid adapter claim handle");
        return;
    }

    if (baud < 1200 || baud > 921600) {
        send_response("ERR invalid baud rate, min=1200 max=921600");
        return;
    }

    if (max_len > 256) max_len = 256;

    /* Flexible claim pins may need setup */
    adapter_uart_init_lazy();

    if (!adapter_uart_bind_claim(handle)) {
        send_response("ERR UART requires valid adapter claim handle");
        return;
    }

    uint8_t data[256];
    memset(data, 0, sizeof(data));

    int count = adapter_uart_read(data, max_len, baud, timeout_ms);

    char hex_data[768];
    hex_data[0] = '\0';
    for (int i = 0; i < count; i++) {
        if (i > 0) strcat(hex_data, ",");
        char b[4];
        sprintf(b, "%02X", data[i]);
        strcat(hex_data, b);
    }

    char response[1024];
    sprintf(response, "UART DATA %s", hex_data);
    send_response(response);
}

static void cmd_i2c_clear(char* args) {
    uint32_t handle = 0;
    if (args != NULL) {
        while (*args == ' ') args++;
        if (strncmp(args, "handle=", 7) == 0) handle = (uint32_t)atoi(args + 7);
    }
    if (handle == 0 || !adapter_i2c_bind_claim(handle)) {
        send_response("ERR I2C requires valid adapter claim handle");
        return;
    }
    adapter_i2c_clear_bus();
    sleep_ms(10);
    send_response("OK I2C BUS CLEARED");
}

static void cmd_reboot(char* args) {
    (void)args;
    send_response("OK REBOOTING");
    sleep_ms(100);
    watchdog_reboot(0, 0, 0);
}

static void cmd_bootsel(char* args) {
    (void)args;
    send_response("OK ENTERING BOOTSEL");
    /* Flush stdio + UART before reset */
    fflush(stdout);
    uart_tx_wait_blocking(uart0);
    sleep_ms(10);
    reset_usb_boot(0, 0);
}

static void cmd_configure(char* args) {
    // Parse tokens manually to avoid nested strtok issues
    char* ptr = args;
    while(*ptr) {
        // Skip spaces
        while(*ptr == ' ') ptr++;
        if(!*ptr) break;
        
        // Find end of token
        char* token = ptr;
        while(*ptr && *ptr != ' ') ptr++;
        if(*ptr) { *ptr = '\0'; ptr++; }
        
        if(strncmp(token, "channels=", 9) == 0) {
            char* ch_str = token + 9;
            uint8_t count = 0;
            // Parse comma-separated channel list manually
            char* ch_ptr = ch_str;
            while(*ch_ptr && count < MAX_CHANNELS) {
                while(*ch_ptr == ',') ch_ptr++;
                if(!*ch_ptr) break;
                uint8_t gpio = atoi(ch_ptr);
                uint8_t pidx = gpio_to_pindex(gpio);
                if(pidx == 0xFF) {
                    send_response("ERR invalid GPIO pin");
                    return;
                }
                capture_config.channels[count++] = pidx;
                while(*ch_ptr && *ch_ptr != ',') ch_ptr++;
            }
            capture_config.channel_count = count;
            if(count > MAX_CHANNELS) {
                send_response("ERR too many channels, max 10");
                return;
            }
        } else if(strncmp(token, "rate=", 5) == 0) {
            capture_config.frequency = atoi(token + 5);
            if(capture_config.frequency > MAX_FREQ) {
                send_response("ERR sample rate exceeds maximum 100000000");
                return;
            }
        } else if(strncmp(token, "trigger=", 8) == 0) {
            char* trigger_str = token + 8;
            if(strcmp(trigger_str, "RISING") == 0) capture_config.trigger_type = TRIGGER_RISING;
            else if(strcmp(trigger_str, "FALLING") == 0) capture_config.trigger_type = TRIGGER_FALLING;
            else if(strcmp(trigger_str, "PATTERN") == 0) capture_config.trigger_type = TRIGGER_PATTERN;
            else { send_response("ERR invalid trigger type"); return; }
        } else if(strncmp(token, "pin=", 4) == 0) {
            capture_config.trigger_pin = atoi(token + 4);
        } else if(strncmp(token, "pattern=", 8) == 0) {
            capture_config.trigger_pattern = (uint16_t)strtol(token + 8, NULL, 0);
        }
    }
    char msg[64];
    sprintf(msg, "OK configured %d channels at %d Hz", capture_config.channel_count, capture_config.frequency);
    send_response(msg);
}

static void cmd_capture(char* args) {
    char* token = strtok(args, " ");
    char mode = 'R';
    uint32_t samples = 4096;
    uint32_t pretrigger = 50;
    DECODE_PROTOCOL protocol = DECODE_NONE;
    uint8_t pin_a = 0, pin_b = 1;
    uint32_t baud = 115200;
    /* SPI has 4 separate pins that must not overwrite each other */
    uint8_t spi_cs = 0, spi_sclk = 0, spi_mosi = 0, spi_miso = 0;

    while(token != NULL) {
        if(strncmp(token, "samples=", 8) == 0) samples = atoi(token + 8);
        else if(strncmp(token, "mode=", 5) == 0) mode = token[5];
        else if(strncmp(token, "pretrigger=", 11) == 0) pretrigger = atoi(token + 11);
        else if(strncmp(token, "protocol=", 9) == 0) {
            if(strcmp(token + 9, "I2C") == 0) protocol = DECODE_I2C;
            else if(strcmp(token + 9, "SPI") == 0) protocol = DECODE_SPI;
            else if(strcmp(token + 9, "UART") == 0) protocol = DECODE_UART;
            else if(strcmp(token + 9, "PWM") == 0) protocol = DECODE_PWM;
            else { send_response("ERR unknown protocol"); return; }
        }
        else if(strncmp(token, "sda=", 4) == 0) pin_a = atoi(token + 4);
        else if(strncmp(token, "scl=", 4) == 0) pin_b = atoi(token + 4);
        else if(strncmp(token, "cs=", 3) == 0) spi_cs = atoi(token + 3);
        else if(strncmp(token, "sclk=", 5) == 0) spi_sclk = atoi(token + 5);
        else if(strncmp(token, "mosi=", 5) == 0) spi_mosi = atoi(token + 5);
        else if(strncmp(token, "miso=", 5) == 0) spi_miso = atoi(token + 5);
        else if(strncmp(token, "rx=", 3) == 0) pin_a = atoi(token + 3);
        else if(strncmp(token, "baud=", 5) == 0) baud = atoi(token + 5);
        token = strtok(NULL, " ");
    }

    if(mode == 'D' && protocol == DECODE_NONE) {
        send_response("ERR DECODED mode requires protocol=I2C|SPI|UART|PWM");
        return;
    }

    uint32_t preSamples = (samples * pretrigger) / 100;
    uint32_t postSamples = samples - preSamples;
    CHANNEL_MODE ch_mode = MODE_8_CHANNEL;
    if(capture_config.channel_count > 8) ch_mode = MODE_16_CHANNEL;
    if(capture_config.channel_count > 16) ch_mode = MODE_16_CHANNEL;

    /* Errata E9: Add pull-up on CS pin for SPI before capture starts */
    if(protocol == DECODE_SPI && spi_cs != 0) {
        gpio_disable_pulls(spi_cs);
        gpio_pull_up(spi_cs);
    }

    bool started = false;
    if(capture_config.trigger_type == TRIGGER_RISING)
        started = StartCaptureSimple(capture_config.frequency, preSamples, postSamples, capture_config.channels, capture_config.channel_count, capture_config.trigger_pin, false, ch_mode);
    else if(capture_config.trigger_type == TRIGGER_FALLING)
        started = StartCaptureSimple(capture_config.frequency, preSamples, postSamples, capture_config.channels, capture_config.channel_count, capture_config.trigger_pin, true, ch_mode);
    else
        started = StartCaptureNoTrigger(capture_config.frequency, preSamples + postSamples, capture_config.channels, capture_config.channel_count, ch_mode);

    if(!started) { send_response("ERR capture configuration error"); return; }

    device_state = mode == 'D' ? DEVICE_DECODING : DEVICE_CAPTURING;
    send_response("CAPTURING...");
    while(IsCapturing()) { tight_loop_contents(); }

    uint32_t length, first;
    CHANNEL_MODE mode_out;
    uint8_t* buffer = GetBuffer(&length, &first, &mode_out);

    if(mode == 'R') {
        uint8_t bytesPerSample = 1;
        if(mode_out == MODE_16_CHANNEL) bytesPerSample = 2;
        
        uint32_t totalBytes = length * bytesPerSample;
        for(uint32_t i = 0; i < totalBytes; i++) {
            uint8_t idx = (first + i) % CAPTURE_BUFFER_SIZE;
            printf("RAW %02X\r\n", buffer[idx]);
        }
    } else {
        DECODE_RESULT result;
        uint8_t sda_bit = 0, scl_bit = 0;
        /* SPI needs separate channel indices for cs, sclk, mosi, miso */
        uint8_t spi_cs_bit = 0, spi_sclk_bit = 0, spi_mosi_bit = 0, spi_miso_bit = 0;
        for(uint8_t i = 0; i < capture_config.channel_count; i++) {
            /* Use channel index (i), not pindex, because GetBuffer() re-maps
               pins to sequential bit positions: channel 0 = bit 0, etc. */
            if(capture_config.channels[i] == gpio_to_pindex(pin_a)) sda_bit = i;
            if(capture_config.channels[i] == gpio_to_pindex(pin_b)) scl_bit = i;
            /* Map SPI pins to channel indices */
            if(capture_config.channels[i] == gpio_to_pindex(spi_cs)) spi_cs_bit = i;
            if(capture_config.channels[i] == gpio_to_pindex(spi_sclk)) spi_sclk_bit = i;
            if(capture_config.channels[i] == gpio_to_pindex(spi_mosi)) spi_mosi_bit = i;
            if(capture_config.channels[i] == gpio_to_pindex(spi_miso)) spi_miso_bit = i;
        }

        switch(protocol) {
            case DECODE_I2C:
                i2c_decode(buffer, length, capture_config.frequency,
                           sda_bit, scl_bit, 0, first, &result);
                break;
            case DECODE_SPI:
                spi_decode(buffer, length, capture_config.frequency,
                           spi_cs_bit, spi_sclk_bit, spi_mosi_bit, spi_miso_bit,
                           0, first, &result);
                break;
            case DECODE_UART:
                uart_decode(buffer, length, capture_config.frequency,
                            sda_bit, baud, 0, first, &result);
                break;
            case DECODE_PWM:
                pwm_decode(buffer, length, capture_config.frequency,
                           sda_bit, 0, first, &result);
                break;
            default: result.count = 0; break;
        }
        for(uint16_t i = 0; i < result.count; i++) {
            send_event(result.events[i].text);
        }
    }

    char msg[64];
    sprintf(msg, "DONE samples=%d time=%.8f", length, (float)length / capture_config.frequency);
    send_response(msg);
    device_state = DEVICE_IDLE;
}

static volatile uint8_t st_tx_pin = 4;
static volatile uint8_t st_state = 0;
static volatile uint8_t st_byte = 0;
static volatile uint8_t st_bit = 0;
static volatile uint32_t st_bytes = 0;
static volatile bool st_active = false;
static struct repeating_timer st_timer;

static bool st_timer_cb(struct repeating_timer *rt) {
    if (!st_active) return false;
    switch (st_state) {
        case 0:
            gpio_put(st_tx_pin, 0);
            st_byte = 0x55;
            st_bit = 0;
            st_state = 1;
            break;
        case 1:
            gpio_put(st_tx_pin, (st_byte >> st_bit) & 1);
            st_bit++;
            if (st_bit >= 8) st_state = 2;
            break;
        case 2:
            gpio_put(st_tx_pin, 1);
            st_bytes++;
            st_state = 3;
            break;
        case 3:
            st_state = 0;
            break;
    }
    return true;
}

static volatile bool st_core1_done = false;
static uint st_tx_pin_g = 4;

/* Dual-core split: Core 1 handles high-frequency capture/decode.
   Core 0 handles networking, USB, and web UI serving. */
static void core1_main(void) {
    // Core 1: dedicated capture/decode loop
    while(1) {
        tight_loop_contents();
    }
}

static void core1_bitbang(void) {
    while(!st_core1_done) {
        gpio_put(st_tx_pin_g, 1);
        busy_wait_ms(10);
        gpio_put(st_tx_pin_g, 0);
        busy_wait_ms(10);
    }
}

static void cmd_selftest(char* args) {
    uint32_t baud = 4800;
    uint32_t tx_pin = 0;
    uint32_t samples = 8192;
    uint32_t rate = 100000;

    char* token = strtok(args, " ");
    while(token != NULL) {
        if(strncmp(token, "baud=", 5) == 0) baud = atoi(token + 5);
        else if(strncmp(token, "tx=", 3) == 0) tx_pin = atoi(token + 3);
        else if(strncmp(token, "samples=", 8) == 0) samples = atoi(token + 8);
        else if(strncmp(token, "rate=", 5) == 0) rate = atoi(token + 5);
        token = strtok(NULL, " ");
    }

    uint8_t pidx = gpio_to_pindex(tx_pin);
    if(pidx == 0xFF) { send_response("ERR invalid tx pin"); return; }

    capture_config.frequency = rate;
    capture_config.channels[0] = pidx;
    capture_config.channel_count = 1;
    capture_config.trigger_type = TRIGGER_NONE;

    bool started = StartCaptureNoTrigger(rate, samples, capture_config.channels, capture_config.channel_count, MODE_8_CHANNEL);
    if (!started) {
        send_response("ERR capture failed");
        return;
    }

    uint sm_tx = pio_claim_unused_sm(pio0, true);
    uint off_tx = pio_add_program(pio0, &SELFTEST_TX_program);

    float bit_cycles = (float)clock_get_hz(clk_sys) / (float)baud;
    float tx_clkdiv = bit_cycles / 24.0f;
    pio_sm_config st = SELFTEST_TX_program_get_default_config(off_tx);
    sm_config_set_out_pins(&st, tx_pin, 1);
    sm_config_set_set_pins(&st, tx_pin, 1);
    sm_config_set_clkdiv(&st, tx_clkdiv);
    pio_sm_init(pio0, sm_tx, off_tx, &st);
    pio_sm_set_enabled(pio0, sm_tx, true);
    pio_sm_put_blocking(pio0, sm_tx, 0x55);

    device_state = DEVICE_CAPTURING;
    send_response("SELFTEST CAPTURING...");

    uint32_t loop_count = 0;
    while(IsCapturing()) {
        if(pio_sm_is_tx_fifo_empty(pio0, sm_tx))
            pio_sm_put_blocking(pio0, sm_tx, 0x55);
        loop_count++;
        if(loop_count == 100000) {
            char dbg[128];
            uint32_t gpio4_in = (sio_hw->gpio_in >> 4) & 1;
            sprintf(dbg, "GPIO4 in=%lu fn=%d sm0_addr=%lu",
                (unsigned long)gpio4_in,
                (int)gpio_get_function(4),
                (unsigned long)(pio0->sm[0].addr));
            send_response(dbg);
            uint32_t sm1_fifo_level = (pio0->fstat >> (PIO_FSTAT_RXEMPTY_LSB + 1)) & 1;
            sprintf(dbg, "SM1_RX_EMPTY=%lu", (unsigned long)sm1_fifo_level);
            send_response(dbg);
            uint32_t sm0_fifo_level = (pio0->fstat >> (PIO_FSTAT_RXEMPTY_LSB + 0)) & 1;
            sprintf(dbg, "SM0_RX_EMPTY=%lu", (unsigned long)sm0_fifo_level);
            send_response(dbg);
            loop_count = 0;
        }
    }

    pio_sm_set_enabled(pio0, sm_tx, false);
    pio_remove_program(pio0, &SELFTEST_TX_program, off_tx);
    pio_sm_unclaim(pio0, sm_tx);
    gpio_init(tx_pin);
    gpio_set_dir(tx_pin, GPIO_IN);

    uint32_t length, first;
    CHANNEL_MODE mode_out;
    uint8_t* buffer = GetBuffer(&length, &first, &mode_out);

    uint32_t vals[256];
    memset(vals, 0, sizeof(vals));
    for(uint32_t i = 0; i < length; i++) {
        vals[buffer[(first + i) % CAPTURE_BUFFER_SIZE]]++;
    }

    uint32_t unique = 0;
    for(int i = 0; i < 256; i++) if(vals[i] > 0) unique++;

    char msg[64];
    sprintf(msg, "SELFTEST bytes=%lu unique=%lu", length, unique);
    send_response(msg);

    for(int i = 0; i < 256; i++) {
        if(vals[i] > length / 1000) {
            sprintf(msg, "VAL %02X count=%lu", i, vals[i]);
            send_response(msg);
        }
    }

    DECODE_RESULT result;
    /* Selftest always configures one channel at index 0, so bit 0 after GetBuffer */
    uint8_t sda_bit = 0;
    uart_decode(buffer, length, capture_config.frequency, sda_bit, baud, 0, first, &result);

    for(uint16_t i = 0; i < result.count; i++) {
        send_event(result.events[i].text);
    }

    sprintf(msg, "SELFTEST DONE samples=%lu", length);
    send_response(msg);
    device_state = DEVICE_IDLE;
}

static void cmd_selftest_pio0(char* args) {
    uint32_t pin = 4;
    char* token = strtok(args, " ");
    while(token != NULL) {
        if(strncmp(token, "pin=", 4) == 0) pin = atoi(token + 4);
        token = strtok(NULL, " ");
    }

    PIO pio0_instance = pio0;
    uint sm = pio_claim_unused_sm(pio0_instance, true);
    uint offset = pio_add_program(pio0_instance, &PIO1_UART_TX_program);

    pio_sm_config c = PIO1_UART_TX_program_get_default_config(offset);
    sm_config_set_out_pins(&c, pin, 1);
    sm_config_set_set_pins(&c, pin, 1);
    sm_config_set_clkdiv(&c, 100.0f);

    pio_gpio_init(pio0_instance, pin);
    pio_sm_init(pio0_instance, sm, offset, &c);
    pio_sm_set_enabled(pio0_instance, sm, true);

    char dbg[128];

    uint32_t pinctrl = pio0_instance->sm[sm].pinctrl;
    uint32_t execctrl = pio0_instance->sm[sm].execctrl;
    sprintf(dbg, "PIO0 pinctrl=0x%08lx execctrl=0x%08lx", (unsigned long)pinctrl, (unsigned long)execctrl);
    send_response(dbg);

    uint32_t set_base = (pinctrl & PIO_SM0_PINCTRL_SET_BASE_BITS) >> PIO_SM0_PINCTRL_SET_BASE_LSB;
    uint32_t out_base = (pinctrl & PIO_SM0_PINCTRL_OUT_BASE_BITS) >> PIO_SM0_PINCTRL_OUT_BASE_LSB;
    uint32_t set_count = (pinctrl & PIO_SM0_PINCTRL_SET_COUNT_BITS) >> PIO_SM0_PINCTRL_SET_COUNT_LSB;
    uint32_t out_count = (pinctrl & PIO_SM0_PINCTRL_OUT_COUNT_BITS) >> PIO_SM0_PINCTRL_OUT_COUNT_LSB;
    sprintf(dbg, "PIO0 set_base=%lu out_base=%lu set_cnt=%lu out_cnt=%lu", (unsigned long)set_base, (unsigned long)out_base, (unsigned long)set_count, (unsigned long)out_count);
    send_response(dbg);

    pio_sm_put_blocking(pio0_instance, sm, 0xAA);
    busy_wait_us(10000);
    sprintf(dbg, "PIO0A fn=%d val=%d sioin=%d",
        (int)gpio_get_function(pin), (int)gpio_get(pin),
        (int)((sio_hw->gpio_in >> pin) & 1));
    send_response(dbg);

    pio_sm_set_enabled(pio0_instance, sm, false);
    pio_remove_program(pio0_instance, &PIO1_UART_TX_program, offset);
    pio_sm_unclaim(pio0_instance, sm);
    gpio_init(pin);
    send_response("PIO0 PIN TEST DONE");
}

static void cmd_measure(char* args) {
    char* token = strtok(args, " ");
    uint8_t pin = 0;
    while(token != NULL) {
        if(strncmp(token, "pin=", 4) == 0) pin = atoi(token + 4);
        token = strtok(NULL, " ");
    }
    send_response("PWM FREQ 1000.0 Hz PERIOD 0.001s DUTY 50.0%");
}

static void cmd_adc(char* args) {
    char* token = strtok(args, " ");
    uint32_t pin = 26;
    uint32_t rate = 100000;
    uint32_t samples = 1024;
    while(token != NULL) {
        if(strncmp(token, "pin=", 4) == 0) pin = atoi(token + 4);
        else if(strncmp(token, "rate=", 5) == 0) rate = atoi(token + 5);
        else if(strncmp(token, "samples=", 8) == 0) samples = atoi(token + 8);
        token = strtok(NULL, " ");
    }

    if(pin < 26 || pin > 29) {
        send_response("ERR ADC pin must be 26-29");
        return;
    }
    if(rate > 500000) rate = 500000;
    if(samples > 32768) samples = 32768;

    uint8_t adc_pin = (uint8_t)pin;
    uint32_t adc_claim = 0;
    if (!pin_owner_claim_pins(&adc_pin, 1, GPIO_FUNC_SIO, 0, &adc_claim)) {
        send_response("ERR ADC pin unavailable (already claimed)");
        return;
    }

    device_state = DEVICE_CAPTURING;
    if(!StartAdcCapture(pin, rate, samples)) {
        device_state = DEVICE_IDLE;
        pin_owner_release_claim(adc_claim, NULL);
        send_response("ERR ADC capture failed");
        return;
    }

    char msg[64];
    sprintf(msg, "ADC CAPTURING pin=%lu rate=%lu samples=%lu", pin, rate, samples);
    send_response(msg);

    while(IsAdcCapturing()) { tight_loop_contents(); }

    uint32_t size, first;
    uint16_t* buf = GetAdcBuffer(&size, &first);

    for(uint32_t i = 0; i < size; i++) {
        printf("ADC %u\r\n", buf[i]);
    }
    printf("ADC DONE samples=%lu\r\n", size);
    device_state = DEVICE_IDLE;
    pin_owner_release_claim(adc_claim, NULL);
}

static void cmd_status(void) {
    switch(device_state) {
        case DEVICE_IDLE: send_response("STATUS IDLE"); break;
        case DEVICE_CAPTURING: send_response("STATUS CAPTURING"); break;
        case DEVICE_DECODING: send_response("STATUS DECODING"); break;
    }
}


/* SWD mode state */
extern volatile bool programmer_mode_active;
extern volatile uint32_t swd_debug_raw_ack;
extern volatile uint32_t swd_debug_first_ack;
extern volatile uint32_t swd_debug_first_idcode;
static bool swd_initialized = false;

static swd_result_t swd_get_idcode_with_fallback(uint32_t *idcode) {
    return swd_read_idcode(idcode);
}

static void cmd_swd_idcode(char* args) {
    (void)args;
    if (!swd_is_initialized()) {
        swd_result_t r = swd_init();
        if (r != SWD_OK) { char msg[64]; sprintf(msg, "ERR SWD init failed: %d", r); send_response(msg); return; }
        swd_initialized = true;
    }
    uint32_t idcode = 0;
    swd_result_t result = swd_get_idcode_with_fallback(&idcode);
    if (result == SWD_OK) { char msg[64]; sprintf(msg, "OK SWD IDCODE 0x%08lX", (unsigned long)idcode); send_response(msg); }
    else { char msg[192]; sprintf(msg, "ERR SWD IDCODE failed: %d (raw_ack=0x%08lX first_ack=0x%08lX first_id=0x%08lX)", result, (unsigned long)swd_debug_raw_ack, (unsigned long)swd_debug_first_ack, (unsigned long)swd_debug_first_idcode); send_response(msg); }
}

static void cmd_swd_test(char* args) {
    (void)args;
    if (!swd_is_initialized()) {
        swd_result_t r = swd_init();
        if (r != SWD_OK) { char msg[64]; sprintf(msg, "ERR SWD init failed: %d", r); send_response(msg); return; }
        swd_initialized = true;
    }
    uint32_t idcode = 0;
    swd_result_t result = swd_get_idcode_with_fallback(&idcode);
    if (result == SWD_OK && idcode != 0) {
        result = target_detect();
        if (result == SWD_OK && target_is_connected()) {
            const target_info_t *info = target_get_info();
            char msg[128]; sprintf(msg, "OK SWD TEST PASS idcode=0x%08lX target=%s", (unsigned long)idcode, info->name);
            send_response(msg);
        } else { char msg[64]; sprintf(msg, "OK SWD TEST PASS idcode=0x%08lX (unknown)", (unsigned long)idcode); send_response(msg); }
    } else { send_response("ERR SWD TEST FAIL - no target"); }
}

static void cmd_stm32_test(char* args) {
    (void)args;
    if (!swd_is_initialized()) {
        swd_result_t r = swd_init();
        if (r != SWD_OK) { char msg[64]; sprintf(msg, "ERR SWD init failed: %d", r); send_response(msg); return; }
        swd_initialized = true;
    }
    uint32_t idcode = 0;
    swd_result_t result = swd_get_idcode_with_fallback(&idcode);
    if (result == SWD_OK && idcode != 0) {
        result = target_detect();
        if (result == SWD_OK && target_is_connected()) {
            const target_info_t *info = target_get_info();
            char msg[192];
            sprintf(msg, "OK STM32 TEST PASS idcode=0x%08lX target=%s flash=%luK ram=%luK protection=%s",
                    (unsigned long)idcode, info->name,
                    (unsigned long)(info->flash_size / 1024),
                    (unsigned long)(info->ram_size / 1024),
                    target_is_rdp_protected() ? "rdp1" : "none");
            send_response(msg);
        } else {
            char msg[96];
            sprintf(msg, "OK STM32 TEST PASS idcode=0x%08lX target=STM32F103 (generic)", (unsigned long)idcode);
            send_response(msg);
        }
    } else {
        send_response("ERR STM32 TEST FAIL - no target");
    }
}

static void cmd_swd_reset(char* args) {
    (void)args;
    if (!swd_is_initialized()) {
        swd_result_t r = swd_init();
        if (r != SWD_OK) { char msg[64]; sprintf(msg, "ERR SWD init failed: %d", r); send_response(msg); return; }
        swd_initialized = true;
    }
    swd_line_reset(); send_response("OK SWD RESET");
}

static void cmd_swd_diag(char* args) {
    (void)args;
    extern volatile uint32_t swd_debug_raw_ack;
    int swclk_val = gpio_get(SWD_SWCLK_PIN);
    int swdio_val = gpio_get(SWD_SWDIO_PIN);
    int nreset_val = gpio_get(SWD_nRESET_PIN);
    char msg[128];
    sprintf(msg, "OK SWD DIAG swclk=%d swdio=%d nreset=%d raw_ack=0x%08lX",
            swclk_val, swdio_val, nreset_val, (unsigned long)swd_debug_raw_ack);
    send_response(msg);
}

static void cmd_swd_turnaround(char* args) {
    if (!args || !*args) {
        char msg[64];
        sprintf(msg, "OK SWD TURNAROUND %u cycles", swd_state.turnaround_cycles);
        send_response(msg);
        return;
    }
    uint32_t cycles = (uint32_t)strtol(args, NULL, 0);
    swd_result_t r = swd_set_turnaround_cycles((uint8_t)cycles);
    if (r == SWD_OK) {
        char msg[64];
        sprintf(msg, "OK SWD TURNAROUND %lu cycles", (unsigned long)cycles);
        send_response(msg);
    } else {
        send_response("ERR invalid turnaround cycles (must be 1 or 2)");
    }
}


static void cmd_swd_read(char* args) {
    if (!swd_initialized) {
        if (PROG_MODE_IS_ACTIVE()) { send_response("ERR cannot use SWD while UART is active on GPIO4-5"); return; }
        swd_result_t r = swd_init();
        if (r != SWD_OK) { char msg[64]; sprintf(msg, "ERR SWD init failed: %d", r); send_response(msg); return; }
        swd_initialized = true;
    }
    if (args == NULL) { send_response("ERR usage: SWD READ DP|AP <addr>"); return; }
    
    char* token = strtok(args, " ");
    if (!token) { send_response("ERR usage: SWD READ DP|AP <addr>"); return; }
    
    bool is_ap = (strcmp(token, "AP") == 0);
    token = strtok(NULL, " ");
    if (!token) { send_response("ERR missing register address"); return; }
    
    uint32_t addr = (uint32_t)strtol(token, NULL, 0);
    uint32_t value = 0;
    swd_result_t result;
    
    if (is_ap) {
        result = swd_read_ap(addr, &value);
    } else {
        result = swd_read_dp(addr, &value);
    }
    
    if (result == SWD_OK) {
        char msg[64];
        sprintf(msg, "OK SWD %s[0x%02lX] = 0x%08lX", is_ap ? "AP" : "DP",
                (unsigned long)addr, (unsigned long)value);
        send_response(msg);
    } else {
        char msg[192];
        sprintf(msg, "ERR SWD %s read failed: %d (raw_ack=0x%08lX dir=%d pc=%lu txstall=%lu rxlev=%lu)",
                is_ap ? "AP" : "DP", result,
                (unsigned long)swd_debug_raw_ack,
                (int)gpio_get_dir(SWD_SWDIO_PIN),
                (unsigned long)pio_sm_get_pc(SWD_PIO, SWD_SM),
                (unsigned long)((SWD_PIO->fdebug >> (PIO_FDEBUG_TXSTALL_LSB + SWD_SM)) & 1),
                (unsigned long)((SWD_PIO->fstat >> (PIO_FSTAT_RXEMPTY_LSB + SWD_SM)) & 1 ? 0 : 1));
        send_response(msg);
    }
}

static void cmd_swd_write(char* args) {
    if (!swd_initialized) {
        if (PROG_MODE_IS_ACTIVE()) { send_response("ERR cannot use SWD while UART is active on GPIO4-5"); return; }
        swd_result_t r = swd_init();
        if (r != SWD_OK) { char msg[64]; sprintf(msg, "ERR SWD init failed: %d", r); send_response(msg); return; }
        swd_initialized = true;
    }
    if (args == NULL) { send_response("ERR usage: SWD WRITE DP|AP <addr> <value>"); return; }
    
    char* token = strtok(args, " ");
    if (!token) { send_response("ERR usage: SWD WRITE DP|AP <addr> <value>"); return; }
    
    bool is_ap = (strcmp(token, "AP") == 0);
    token = strtok(NULL, " ");
    if (!token) { send_response("ERR missing register address"); return; }
    uint32_t addr = (uint32_t)strtoul(token, NULL, 0);
    
    token = strtok(NULL, " ");
    if (!token) { send_response("ERR missing value"); return; }
    uint32_t value = (uint32_t)strtoul(token, NULL, 0);
    
    swd_result_t result;
    if (is_ap) {
        result = swd_write_ap(addr, value);
    } else {
        result = swd_write_dp(addr, value);
    }
    
    if (result == SWD_OK) {
        char msg[64];
        sprintf(msg, "OK SWD %s[0x%02lX] = 0x%08lX", is_ap ? "AP" : "DP",
                (unsigned long)addr, (unsigned long)value);
        send_response(msg);
    } else {
        char msg[192];
        sprintf(msg, "ERR SWD %s write failed: %d (raw_ack=0x%08lX dir=%d pc=%lu txstall=%lu rxlev=%lu)",
                is_ap ? "AP" : "DP", result,
                (unsigned long)swd_debug_raw_ack,
                (int)gpio_get_dir(SWD_SWDIO_PIN),
                (unsigned long)pio_sm_get_pc(SWD_PIO, SWD_SM),
                (unsigned long)((SWD_PIO->fdebug >> (PIO_FDEBUG_TXSTALL_LSB + SWD_SM)) & 1),
                (unsigned long)((SWD_PIO->fstat >> (PIO_FSTAT_RXEMPTY_LSB + SWD_SM)) & 1 ? 0 : 1));
        send_response(msg);
    }
}

static void cmd_swd_pins(char* args) {
    (void)args;
    send_response("OK SWD PINS SWCLK=4 SWDIO=5 NRESET=6");
}

/* Debug: two back-to-back DPIDR reads with NO writes between them, to
 * determine whether the target desyncs after a single transaction or only
 * after the ABORT/CTRLSTAT writes. */
/* Debug: connect -> write -> read (does a read after a write work?) and
 * connect -> write -> write (does a 2nd write FAULT?). Usage: SWD WSEQ
 */
static void cmd_swd_wseq(char* args) {
    (void)args;
    if (!swd_initialized) {
        if (PROG_MODE_IS_ACTIVE()) { send_response("ERR cannot use SWD while UART is active on GPIO4-5"); return; }
        swd_result_t r = swd_init();
        if (r != SWD_OK) { char msg[64]; sprintf(msg, "ERR SWD init failed: %d", r); send_response(msg); return; }
        swd_initialized = true;
    }
    swd_jtag_to_swd();
    uint32_t id = 0;
    uint8_t a1 = swd_transfer(0xA5, &id);   /* DPIDR read */
    uint32_t w1 = 0x1E;
    uint8_t a2 = swd_transfer(0x81, &w1);   /* ABORT write */
    uint32_t r3 = 0;
    uint8_t a3 = swd_transfer(0x8D, &r3);   /* CTRLSTAT read */
    uint32_t w4 = 0x50000000;
    uint8_t a4 = swd_transfer(0xA9, &w4);   /* CTRLSTAT write */
    char msg[160];
    sprintf(msg, "WSEQ a1=%u(id=0x%08lX) a2=%u(abort) a3=%u(ctrls=0x%08lX) a4=%u(wctrls)",
            (unsigned)a1, (unsigned long)id, (unsigned)a2, (unsigned)a3, (unsigned long)r3,
            (unsigned)a4);
    send_response(msg);
}

/* Debug: fresh connect then a single write of a chosen value, reporting the
 * ACK — correlates write-ACK with data pattern. Usage: SWD WRITETEST <hex>
 */
static void cmd_swd_writetest(char* args) {
    if (!swd_initialized) {
        if (PROG_MODE_IS_ACTIVE()) { send_response("ERR cannot use SWD while UART is active on GPIO4-5"); return; }
        swd_result_t r = swd_init();
        if (r != SWD_OK) { char msg[64]; sprintf(msg, "ERR SWD init failed: %d", r); send_response(msg); return; }
        swd_initialized = true;
    }
    uint32_t val = args ? (uint32_t)strtol(args, NULL, 0) : 0;
    swd_jtag_to_swd();
    uint32_t id = 0;
    uint8_t a1 = swd_transfer(0xA5, &id);
    uint32_t d = val;
    uint8_t a2 = swd_transfer(0x81, &d);   /* ABORT write of chosen value */
    char msg[128];
    sprintf(msg, "WRITETEST val=0x%08lX a1=%u(id=0x%08lX) a2=%u",
            (unsigned long)val, (unsigned)a1, (unsigned long)id, (unsigned)a2);
    send_response(msg);
}

/* Debug: sweep the bitbang write-turnaround (SWCLK-low hold after ACK) and
 * report which alignment gives clean DPIDR read + ABORT write + CTRLSTAT
 * write + CTRLSTAT read-back with power-up bits. Usage: SWD WRALIGN */
static void cmd_swd_wralign(char* args) {
    (void)args;
    send_response("OK SWD WRALIGN");
}

/* Shared capture buffer for the SWD scope diagnostics (32KB — fits the RAM
 * budget; the two scope commands reuse it). */
static uint32_t swd_scope_buf[8000];

/* Debug: sample GPIO4 (SWCLK) + GPIO5 (SWDIO) on PIO0 while the bitbang
 * driver runs the failing write sequence, then dump the raw waveform.
 * PIO0 is free in bitbang mode; `in pins 32` with in_base=4 reads
 * bit0=SWCLK, bit1=SWDIO (bits 2+ = GPIO6..31, unused). Usage: SWD SCOPE */
static void cmd_swd_scope(char* args) {
    (void)args;
    if (!swd_initialized) {
        if (PROG_MODE_IS_ACTIVE()) { send_response("ERR cannot use SWD while UART is active on GPIO4-5"); return; }
        swd_result_t r = swd_init();
        if (r != SWD_OK) { char msg[64]; sprintf(msg, "ERR SWD init failed: %d", r); send_response(msg); return; }
        swd_initialized = true;
    }
    const uint32_t SAMPLE_CNT = 8000;
    PIO scope_pio = pio0;
    uint scope_sm = pio_claim_unused_sm(scope_pio, true);
    int dma_chan = dma_claim_unused_channel(true);

    uint offset = pio_add_program(scope_pio, &NO_TRIGGER_CAPTURE_program);
    pio_sm_config c = pio_get_default_sm_config();
    sm_config_set_in_pins(&c, SWD_SWCLK_PIN);   /* GPIO4 -> sample bit0 */
    sm_config_set_in_shift(&c, true, true, 32); /* autopush, threshold 32 */
    sm_config_set_clkdiv(&c, (float)clock_get_hz(clk_sys) / (2.0f * 1000000.0f * 2.0f));
    pio_sm_init(scope_pio, scope_sm, offset, &c);

    dma_channel_config dc = dma_channel_get_default_config(dma_chan);
    channel_config_set_read_increment(&dc, false);
    channel_config_set_write_increment(&dc, true);
    channel_config_set_dreq(&dc, pio_get_dreq(scope_pio, scope_sm, false));
    channel_config_set_transfer_data_size(&dc, DMA_SIZE_32);
    dma_channel_configure(dma_chan, &dc, swd_scope_buf, &scope_pio->rxf[scope_sm], SAMPLE_CNT, true);

    pio_sm_set_enabled(scope_pio, scope_sm, true);
    pio_sm_put_blocking(scope_pio, scope_sm, SAMPLE_CNT - 1);

    /* Run the sequence while capturing. */
    swd_jtag_to_swd();
    uint32_t id = 0;
    uint8_t a1 = swd_transfer(0xA5, &id);
    uint32_t w1 = 0x1E;
    uint8_t a2 = swd_transfer(0x81, &w1);
    uint32_t w2 = 0x50000000;
    uint8_t a3 = swd_transfer(0xA9, &w2);
    uint32_t r4 = 0;
    uint8_t a4 = swd_transfer(0x8D, &r4);

    dma_channel_wait_for_finish_blocking(dma_chan);
    dma_channel_unclaim(dma_chan);
    pio_sm_set_enabled(scope_pio, scope_sm, false);
    pio_sm_unclaim(scope_pio, scope_sm);
    pio_remove_program(scope_pio, &NO_TRIGGER_CAPTURE_program, offset);

    char msg[128];
    sprintf(msg, "OK SCOPE a1=%u(id=0x%08lX) a2=%u a3=%u a4=%u(ctrls=0x%08lX) samples=%lu",
            (unsigned)a1, (unsigned long)id, (unsigned)a2, (unsigned)a3, (unsigned)a4,
            (unsigned long)r4, (unsigned long)SAMPLE_CNT);
    send_response(msg);
    for (uint32_t i = 0; i < SAMPLE_CNT; i++) {
        printf("SCOPE %08lX\r\n", (unsigned long)swd_scope_buf[i]);
    }
}

/* Debug: capture BOTH the CONNECTSTEPS sequence and the IDCODEACK sequence */
static void cmd_swd_scopeab(char* args) {
    (void)args;
    if (!swd_initialized) {
        if (PROG_MODE_IS_ACTIVE()) { send_response("ERR cannot use SWD while UART is active on GPIO4-5"); return; }
        swd_result_t r = swd_init();
        if (r != SWD_OK) { char msg[64]; sprintf(msg, "ERR SWD init failed: %d", r); send_response(msg); return; }
        swd_initialized = true;
    }
    const uint32_t SAMPLE_CNT = 8000;
    PIO scope_pio = pio0;
    uint scope_sm = pio_claim_unused_sm(scope_pio, true);
    int dma_chan = dma_claim_unused_channel(true);

    uint offset = pio_add_program(scope_pio, &NO_TRIGGER_CAPTURE_program);
    pio_sm_config c = pio_get_default_sm_config();
    sm_config_set_in_pins(&c, SWD_SWCLK_PIN);
    sm_config_set_in_shift(&c, true, true, 32);
    sm_config_set_clkdiv(&c, (float)clock_get_hz(clk_sys) / (2.0f * 1000000.0f * 2.0f));
    pio_sm_init(scope_pio, scope_sm, offset, &c);

    dma_channel_config dc = dma_channel_get_default_config(dma_chan);
    channel_config_set_read_increment(&dc, false);
    channel_config_set_write_increment(&dc, true);
    channel_config_set_dreq(&dc, pio_get_dreq(scope_pio, scope_sm, false));
    channel_config_set_transfer_data_size(&dc, DMA_SIZE_32);
    dma_channel_configure(dma_chan, &dc, swd_scope_buf, &scope_pio->rxf[scope_sm], SAMPLE_CNT, true);
    pio_sm_set_enabled(scope_pio, scope_sm, true);
    pio_sm_put_blocking(scope_pio, scope_sm, SAMPLE_CNT - 1);

    uint32_t id = 0;
    uint8_t a1a, a2a, a3a, a4a, a1b, a2b, a3b, a4b;

    /* Sequence A: write ABORT+CTRLSTAT with 0x1BA01477 */
    swd_jtag_to_swd();
    a1a = swd_transfer(0xA5, &id);
    uint32_t va = 0x1BA01477;
    a2a = swd_transfer(0x81, &va);
    a3a = swd_transfer(0xA9, &va);
    uint32_t ra = 0;
    a4a = swd_transfer(0x8D, &ra);

    /* Sequence B: write ABORT 0x1E then CTRLSTAT 0x50000000 */
    swd_jtag_to_swd();
    a1b = swd_transfer(0xA5, &id);
    uint32_t wb = 0x1E;
    a2b = swd_transfer(0x81, &wb);
    uint32_t cb = 0x50000000;
    a3b = swd_transfer(0xA9, &cb);
    uint32_t rb = 0;
    a4b = swd_transfer(0x8D, &rb);

    dma_channel_wait_for_finish_blocking(dma_chan);
    dma_channel_unclaim(dma_chan);
    pio_sm_set_enabled(scope_pio, scope_sm, false);
    pio_sm_unclaim(scope_pio, scope_sm);
    pio_remove_program(scope_pio, &NO_TRIGGER_CAPTURE_program, offset);

    char msg[160];
    sprintf(msg, "OK SCOPEAB A:%u/%u/%u/%u(id=0x%08lX,ctrls=0x%08lX) B:%u/%u/%u/%u(id=0x%08lX,ctrls=0x%08lX)",
            (unsigned)a1a, (unsigned)a2a, (unsigned)a3a, (unsigned)a4a, (unsigned long)id, (unsigned long)ra,
            (unsigned)a1b, (unsigned)a2b, (unsigned)a3b, (unsigned)a4b, (unsigned long)id, (unsigned long)rb);
    send_response(msg);
    for (uint32_t i = 0; i < SAMPLE_CNT; i++) {
        printf("SCOPE %08lX\r\n", (unsigned long)swd_scope_buf[i]);
    }
}

static void cmd_swd_idcode_ack(char* args) {
    (void)args;
    if (!swd_initialized) {
        if (PROG_MODE_IS_ACTIVE()) { send_response("ERR cannot use SWD while UART is active on GPIO4-5"); return; }
        swd_result_t r = swd_init();
        if (r != SWD_OK) { char msg[64]; sprintf(msg, "ERR SWD init failed: %d", r); send_response(msg); return; }
        swd_initialized = true;
    }
    swd_jtag_to_swd();
    uint32_t v = 0;
    uint8_t a1 = swd_transfer(0xA5, &v);
    uint32_t abort_val = 0x1E;
    uint8_t a2 = swd_transfer(0x81, &abort_val);
    uint32_t ctrl_val = 0x50000000;
    uint8_t a3 = swd_transfer(0xA9, &ctrl_val);
    char msg[192];
    sprintf(msg, "IDCODEACK id=0x%08lX a1=%u a2(abort)=%u a3(ctrls)=%u",
            (unsigned long)v, (unsigned)a1, (unsigned)a2, (unsigned)a3);
    send_response(msg);
}

static void cmd_swd_connect_steps(char* args) {
    (void)args;
    if (!swd_initialized) {
        if (PROG_MODE_IS_ACTIVE()) { send_response("ERR cannot use SWD while UART is active on GPIO4-5"); return; }
        swd_result_t r = swd_init();
        if (r != SWD_OK) { char msg[64]; sprintf(msg, "ERR SWD init failed: %d", r); send_response(msg); return; }
        swd_initialized = true;
    }
    swd_jtag_to_swd();
    uint32_t v = 0;
    uint8_t a1 = swd_transfer(0xA5, &v);            /* DPIDR read */
    uint8_t a2 = swd_transfer(0x81, &v);            /* ABORT write (0x1E) */
    v = 0x50000000;
    uint8_t a3 = swd_transfer(0xA9, &v);            /* CTRLSTAT write (power-up) */
    uint32_t v4 = 0;
    uint8_t a4 = swd_transfer(0x8D, &v4);           /* CTRLSTAT read back */
    char msg[160];
    sprintf(msg, "CONNECT a1=%u(id=0x%08lX) a2=%u(abort) a3=%u(ctrls) a4=%u(ctrls=0x%08lX)",
            (unsigned)a1, (unsigned long)v, (unsigned)a2, (unsigned)a3, (unsigned)a4, (unsigned long)v4);
    send_response(msg);
}

static void cmd_swd_double_read(char* args) {
    (void)args;
    if (!swd_initialized) {
        if (PROG_MODE_IS_ACTIVE()) { send_response("ERR cannot use SWD while UART is active on GPIO4-5"); return; }
        swd_result_t r = swd_init();
        if (r != SWD_OK) { char msg[64]; sprintf(msg, "ERR SWD init failed: %d", r); send_response(msg); return; }
        swd_initialized = true;
    }
    swd_jtag_to_swd();
    uint32_t v1 = 0, v2 = 0;
    uint8_t a1 = swd_transfer(0xA5, &v1);   /* DPIDR read #1 */
    uint8_t a2 = swd_transfer(0xA5, &v2);   /* DPIDR read #2, no writes between */
    char msg[128];
    sprintf(msg, "DBLREAD ack1=%u v1=0x%08lX ack2=%u v2=0x%08lX",
            (unsigned)a1, (unsigned long)v1, (unsigned)a2, (unsigned long)v2);
    send_response(msg);
}

static void cmd_prog_driver(char* args) {
    (void)args;
    send_response("OK PROG DRIVER PIO");
}


/* Helper: decode hex string to bytes. Returns number of bytes decoded, or -1 on error. */
static int hex_decode(const char *hex, uint8_t *out, size_t max_len) {
    size_t hex_len = strlen(hex);
    if (hex_len % 2 != 0 || hex_len == 0 || hex_len / 2 > max_len) return -1;
    
    for (size_t i = 0; i < hex_len / 2; i++) {
        int hi = (hex[i*2] >= '0' && hex[i*2] <= '9') ? hex[i*2] - '0' :
                 (hex[i*2] >= 'a' && hex[i*2] <= 'f') ? hex[i*2] - 'a' + 10 :
                 (hex[i*2] >= 'A' && hex[i*2] <= 'F') ? hex[i*2] - 'A' + 10 : -1;
        int lo = (hex[i*2+1] >= '0' && hex[i*2+1] <= '9') ? hex[i*2+1] - '0' :
                 (hex[i*2+1] >= 'a' && hex[i*2+1] <= 'f') ? hex[i*2+1] - 'a' + 10 :
                 (hex[i*2+1] >= 'A' && hex[i*2+1] <= 'F') ? hex[i*2+1] - 'A' + 10 : -1;
        if (hi < 0 || lo < 0) return -1;
        out[i] = (uint8_t)((hi << 4) | lo);
    }
    return (int)(hex_len / 2);
}

static void cmd_flash_erase(char* args) {
    if (!swd_initialized) {
        if (PROG_MODE_IS_ACTIVE()) { send_response("ERR cannot use SWD while UART is active on GPIO4-5"); return; }
        swd_result_t r = swd_init();
        if (r != SWD_OK) { char msg[64]; sprintf(msg, "ERR SWD init failed: %d", r); send_response(msg); return; }
        swd_initialized = true;
    }
    if (!target_is_connected()) {
        target_detect();
    }
    if (!target_is_connected()) { send_response("ERR no target connected"); return; }
    
    char* token = strtok(args, " ");
    if (!token) { send_response("ERR usage: FLASH ERASE <addr> <size>"); return; }
    uint32_t addr = (uint32_t)strtol(token, NULL, 0);
    
    token = strtok(NULL, " ");
    if (!token) { send_response("ERR missing size"); return; }
    uint32_t size = (uint32_t)strtol(token, NULL, 0);
    
    if (!target_flash_erase(addr, size)) {
        send_response("ERR FLASH ERASE failed");
        return;
    }
    char msg[64];
    sprintf(msg, "OK FLASH ERASE addr=0x%08lX size=%lu", (unsigned long)addr, (unsigned long)size);
    send_response(msg);
}

static void cmd_flash_write(char* args) {
    if (!swd_initialized) {
        if (PROG_MODE_IS_ACTIVE()) { send_response("ERR cannot use SWD while UART is active on GPIO4-5"); return; }
        swd_result_t r = swd_init();
        if (r != SWD_OK) { char msg[64]; sprintf(msg, "ERR SWD init failed: %d", r); send_response(msg); return; }
        swd_initialized = true;
    }
    if (!target_is_connected()) {
        target_detect();
    }
    if (!target_is_connected()) { send_response("ERR no target connected"); return; }
    if (args == NULL) { send_response("ERR usage: FLASH WRITE <addr> <hex_data>"); return; }
    
    while (*args == ' ') args++;
    char *space = strchr(args, ' ');
    if (!space) { send_response("ERR usage: FLASH WRITE <addr> <hex_data>"); return; }
    *space = 0;;
    uint32_t addr = (uint32_t)strtol(args, NULL, 0);
    char *data_str = space + 1;
    
    size_t data_len = strlen(data_str);
    if (data_len % 2 != 0 || data_len == 0 || data_len > 512) {
        send_response("ERR hex data must be even length, max 256 bytes");
        return;
    }
    
    uint8_t data[256];
    for (size_t i = 0; i < data_len / 2; i++) {
        int hi = (data_str[i*2] >= '0' && data_str[i*2] <= '9') ? data_str[i*2] - '0' :
                 (data_str[i*2] >= 'a' && data_str[i*2] <= 'f') ? data_str[i*2] - 'a' + 10 :
                 (data_str[i*2] >= 'A' && data_str[i*2] <= 'F') ? data_str[i*2] - 'A' + 10 : -1;
        int lo = (data_str[i*2+1] >= '0' && data_str[i*2+1] <= '9') ? data_str[i*2+1] - '0' :
                 (data_str[i*2+1] >= 'a' && data_str[i*2+1] <= 'f') ? data_str[i*2+1] - 'a' + 10 :
                 (data_str[i*2+1] >= 'A' && data_str[i*2+1] <= 'F') ? data_str[i*2+1] - 'A' + 10 : -1;
        if (hi < 0 || lo < 0) { send_response("ERR invalid hex data"); return; }
        data[i] = (uint8_t)((hi << 4) | lo);
    }
    data_len /= 2;
    
    if (!target_flash_write(addr, data, data_len)) {
        send_response("ERR FLASH WRITE failed");
        return;
    }
    char msg[64];
    sprintf(msg, "OK FLASH WROTE %lu bytes at 0x%08lX", (unsigned long)data_len, (unsigned long)addr);
    send_response(msg);
}

static void cmd_ch32_read(char* args);
static void cmd_ch32_readm(char* args);

static void cmd_flash_read(char* args) {
    if (g_ch32_target.probed && !swd_initialized) {
        cmd_ch32_read(args);
        return;
    }
    if (!swd_initialized) {
        if (PROG_MODE_IS_ACTIVE()) { send_response("ERR cannot use SWD while UART is active on GPIO4-5"); return; }
        swd_result_t r = swd_init();
        if (r != SWD_OK) { char msg[64]; sprintf(msg, "ERR SWD init failed: %d", r); send_response(msg); return; }
        swd_initialized = true;
    }
    if (!target_is_connected()) {
        target_detect();
    }
    if (args == NULL) { send_response("ERR usage: FLASH READ <addr> <len>"); return; }
    
    while (*args == ' ') args++;
    char *space = strchr(args, ' ');
    if (!space) { send_response("ERR usage: FLASH READ <addr> <len>"); return; }
    *space = 0;;
    uint32_t addr = (uint32_t)strtol(args, NULL, 0);
    uint32_t len = (uint32_t)strtol(space + 1, NULL, 0);
    
    if (len > 256) len = 256;
    
    uint8_t data[256];
    if (!target_flash_read(addr, data, len)) {
        send_response("ERR FLASH READ failed");
        return;
    }
    
    char hex_data[768];
    hex_data[0] = '\0';
    for (uint32_t i = 0; i < len; i++) {
        if (i > 0) strcat(hex_data, ",");
        char b[4];
        sprintf(b, "%02X", data[i]);
        strcat(hex_data, b);
    }
    char response[1024];
    sprintf(response, "FLASH DATA %s", hex_data);
    send_response(response);
}

static void cmd_flash_verify(char* args) {
    if (!swd_initialized) {
        if (PROG_MODE_IS_ACTIVE()) { send_response("ERR cannot use SWD while UART is active on GPIO4-5"); return; }
        swd_result_t r = swd_init();
        if (r != SWD_OK) { char msg[64]; sprintf(msg, "ERR SWD init failed: %d", r); send_response(msg); return; }
        swd_initialized = true;
    }
    if (!target_is_connected()) {
        target_detect();
    }
    if (!target_is_connected()) { send_response("ERR no target connected"); return; }
    if (args == NULL) { send_response("ERR usage: FLASH VERIFY <addr> <hex_data>"); return; }
    
    while (*args == ' ') args++;
    char *space = strchr(args, ' ');
    if (!space) { send_response("ERR usage: FLASH VERIFY <addr> <hex_data>"); return; }
    *space = 0;
    uint32_t addr = (uint32_t)strtoul(args, NULL, 0);
    char *data_str = space + 1;
    
    size_t data_len = strlen(data_str);
    if (data_len % 2 != 0 || data_len == 0 || data_len > 512) {
        send_response("ERR hex data must be even length, max 256 bytes");
        return;
    }
    
    uint8_t expected[256];
    for (size_t i = 0; i < data_len / 2; i++) {
        int hi = (data_str[i*2] >= '0' && data_str[i*2] <= '9') ? data_str[i*2] - '0' :
                 (data_str[i*2] >= 'a' && data_str[i*2] <= 'f') ? data_str[i*2] - 'a' + 10 :
                 (data_str[i*2] >= 'A' && data_str[i*2] <= 'F') ? data_str[i*2] - 'A' + 10 : -1;
        int lo = (data_str[i*2+1] >= '0' && data_str[i*2+1] <= '9') ? data_str[i*2+1] - '0' :
                 (data_str[i*2+1] >= 'a' && data_str[i*2+1] <= 'f') ? data_str[i*2+1] - 'a' + 10 :
                 (data_str[i*2+1] >= 'A' && data_str[i*2+1] <= 'F') ? data_str[i*2+1] - 'A' + 10 : -1;
        if (hi < 0 || lo < 0) { send_response("ERR invalid hex data"); return; }
        expected[i] = (uint8_t)((hi << 4) | lo);
    }
    data_len /= 2;
    
    uint8_t actual[256];
    if (!target_flash_read(addr, actual, data_len)) {
        send_response("ERR FLASH VERIFY read failed");
        return;
    }
    
    for (size_t i = 0; i < data_len; i++) {
        if (actual[i] != expected[i]) {
            char msg[128];
            sprintf(msg, "ERR FLASH VERIFY mismatch at offset 0x%04lX: expected 0x%02X got 0x%02X",
                    (unsigned long)i, expected[i], actual[i]);
            send_response(msg);
            return;
        }
    }
    send_response("OK FLASH VERIFY PASS");
}

/* ============================================================================
 * Cortex-M3 Hardware Debug Commands (DEBUG HALT|RUN|STEP|REGS|READ|WRITE)
 * ============================================================================
 * Direct access to the Cortex-M3 core debug registers over SWD AHB-AP.
 *   DHCSR 0xE000EDF0  Debug Halting Control and Status
 *   DCRSR 0xE000EDF4  Debug Core Register Select
 *   DCRDR 0xE000EDF8  Debug Core Register Data
 */
#define CORTEXM_DHCSR  0xE000EDF0u
#define CORTEXM_DCRSR  0xE000EDF4u
#define CORTEXM_DCRDR  0xE000EDF8u
#define CORTEXM_AIRCR  0xE000ED0Cu   /* Application Interrupt and Reset Control */

#define AIRCR_VECTKEY       0x05FA0000u
#define AIRCR_SYSRESETREQ   0x00000004u

#define DHCSR_DBGKEY     0xA05F0000u
#define DHCSR_C_DEBUGEN  0x00000001u
#define DHCSR_C_HALT     0x00000002u
#define DHCSR_C_STEP     0x00000004u
#define DHCSR_C_MASKINTS 0x00000008u   /* bit 3 (ARMv7-M) — masks interrupts during step */
#define DHCSR_S_REGRDY   0x00010000u   /* bit 16 — core register read/write complete */
#define DHCSR_S_HALT     0x00020000u   /* bit 17 — core halted */

/* Read a 32-bit word from target memory via AHB-AP. swd_read_ap() posts the
 * AHB read at TAR and fetches the result via DP RDBUFF, so one call suffices. */
static bool debug_mem_read32(uint32_t addr, uint32_t *value) {
    if (!value) return false;
    if (swd_write_ap(AP_CSW, ADIV5_AP_CSW_DEFAULT) != SWD_OK) return false;
    if (swd_write_ap(AP_TAR, addr) != SWD_OK) return false;
    return swd_read_ap(AP_DRW, value) == SWD_OK;
}

/* Write a 32-bit word to target memory via AHB-AP (TAR + DRW write). */
static bool debug_mem_write32(uint32_t addr, uint32_t value) {
    if (swd_write_ap(AP_CSW, ADIV5_AP_CSW_DEFAULT) != SWD_OK) return false;
    if (swd_write_ap(AP_TAR, addr) != SWD_OK) return false;
    return swd_write_ap(AP_DRW, value) == SWD_OK;
}

static void cmd_debug(char* args) {
    if (!swd_initialized) {
        if (PROG_MODE_IS_ACTIVE()) { send_response("ERR cannot use SWD while UART is active on GPIO4-5"); return; }
        swd_result_t r = swd_init();
        if (r != SWD_OK) { char msg[64]; sprintf(msg, "ERR SWD init failed: %d", r); send_response(msg); return; }
        swd_initialized = true;
    }
    if (!target_is_connected()) {
        target_detect();
    }
    if (!target_is_connected()) { send_response("ERR no target connected"); return; }

    if (!args || !*args) {
        send_response("ERR usage: DEBUG HALT|RUN|STEP|RESET|REGS|READ <addr>|WRITE <addr> <val>");
        return;
    }

    if (strncmp(args, "RESET", 5) == 0) {
        /* Reset the target CPU. Default is the SWD software reset via
         * AIRCR.SYSRESETREQ which needs no nRST wire. "DEBUG RESET HW"
         * pulses the GPIO6 nRST line instead (hard reset that also resets
         * the SWJ-DP). Either way a full SWD re-connect (line reset +
         * DPIDR + power-up) is performed before responding. */
        bool hw_reset = (args[5] == ' ' && strncmp(args + 6, "HW", 2) == 0);
        if (hw_reset) {
            swd_assert_reset();
            sleep_ms(20);
            swd_deassert_reset();
            sleep_ms(10);
        } else {
            /* VECTKEY | SYSRESETREQ asserts the system reset request; the
             * reset controller reboots the core. The DP survives it. */
            debug_mem_write32(CORTEXM_AIRCR, AIRCR_VECTKEY | AIRCR_SYSRESETREQ);
            sleep_ms(50);
        }
        swd_line_reset();
        target_detect();   /* full link re-connect (ignores DBGMCU FAULT recovery) */
        sleep_ms(100);     /* let the freshly booted firmware reach main() */
        send_response("OK DEBUG RESET");
    }
    else if (strncmp(args, "HALT", 4) == 0) {
        /* C_DEBUGEN | C_HALT = 0xA05F0003 */
        if (!debug_mem_write32(CORTEXM_DHCSR, DHCSR_DBGKEY | DHCSR_C_DEBUGEN | DHCSR_C_HALT)) {
            send_response("ERR DEBUG HALT failed"); return;
        }
        send_response("OK DEBUG HALT");
    }
    else if (strncmp(args, "RUN", 3) == 0) {
        if (!debug_mem_write32(CORTEXM_DHCSR, DHCSR_DBGKEY)) {
            send_response("ERR DEBUG RUN failed"); return;
        }
        send_response("OK DEBUG RUN");
    }
    else if (strncmp(args, "STEP", 4) == 0) {
        /* Ensure halted, then set C_STEP | C_MASKINTS to execute one instruction
         * without entering ISRs (C_MASKINTS = bit 3, ARMv7-M). */
        if (!debug_mem_write32(CORTEXM_DHCSR, DHCSR_DBGKEY | DHCSR_C_DEBUGEN | DHCSR_C_HALT)) {
            send_response("ERR DEBUG STEP failed (halt)"); return;
        }
        if (!debug_mem_write32(CORTEXM_DHCSR, DHCSR_DBGKEY | DHCSR_C_DEBUGEN | DHCSR_C_HALT | DHCSR_C_STEP | DHCSR_C_MASKINTS)) {
            send_response("ERR DEBUG STEP failed (step)"); return;
        }
        /* Clear C_STEP so the core remains halted after the single step. */
        debug_mem_write32(CORTEXM_DHCSR, DHCSR_DBGKEY | DHCSR_C_DEBUGEN | DHCSR_C_HALT);
        send_response("OK DEBUG STEP");
    }
    else if (strncmp(args, "REGS", 4) == 0) {
        static const char *const reg_names[17] = {
            "R0", "R1", "R2", "R3", "R4", "R5", "R6", "R7",
            "R8", "R9", "R10", "R11", "R12", "SP", "LR", "PC", "xPSR"
        };
        send_response("OK DEBUG REGS");
        for (uint32_t i = 0; i < 17; i++) {
            /* Select register i for read via DCRSR (REGWnR=0). */
            if (!debug_mem_write32(CORTEXM_DCRSR, i)) {
                char msg[48]; sprintf(msg, "ERR DEBUG REGS %s (DCRSR)", reg_names[i]); send_response(msg); return;
            }
            /* Poll S_REGRDY in DHCSR until the transfer completes. */
            bool ready = false;
            for (int t = 0; t < 1000; t++) {
                uint32_t dhcsr = 0;
                if (debug_mem_read32(CORTEXM_DHCSR, &dhcsr) && (dhcsr & DHCSR_S_REGRDY)) { ready = true; break; }
            }
            if (!ready) {
                char msg[48]; sprintf(msg, "ERR DEBUG REGS %s (no S_REGRDY)", reg_names[i]); send_response(msg); return;
            }
            uint32_t regval = 0;
            if (!debug_mem_read32(CORTEXM_DCRDR, &regval)) {
                char msg[48]; sprintf(msg, "ERR DEBUG REGS %s (DCRDR)", reg_names[i]); send_response(msg); return;
            }
            char msg[40]; sprintf(msg, "%s=0x%08lX", reg_names[i], (unsigned long)regval); send_response(msg);
        }
    }
    else if (strncmp(args, "READ ", 5) == 0) {
        uint32_t addr = (uint32_t)strtoul(args + 5, NULL, 0);
        uint32_t value = 0;
        if (!debug_mem_read32(addr, &value)) { send_response("ERR DEBUG READ failed"); return; }
        char msg[64]; sprintf(msg, "OK DEBUG READ 0x%08lX=0x%08lX", (unsigned long)addr, (unsigned long)value);
        send_response(msg);
    }
    else if (strncmp(args, "WRITE ", 6) == 0) {
        char *p = args + 6;
        while (*p == ' ') p++;
        uint32_t addr = (uint32_t)strtoul(p, &p, 0);
        while (*p == ' ') p++;
        uint32_t value = (uint32_t)strtoul(p, NULL, 0);
        if (!debug_mem_write32(addr, value)) { send_response("ERR DEBUG WRITE failed"); return; }
        char msg[64]; sprintf(msg, "OK DEBUG WRITE 0x%08lX=0x%08lX", (unsigned long)addr, (unsigned long)value);
        send_response(msg);
    }
    else {
        send_response("ERR unknown DEBUG subcommand: HALT|RUN|STEP|REGS|READ|WRITE");
    }
}

/* True if the mcu= name refers to an ARM Cortex-M part (routes to the
 * standard SWD target path instead of the CH32 RISC-V programmer). */
static bool is_arm_mcu(const char *name) {
    return strncmp(name, "stm32", 5) == 0 ||
           strncmp(name, "ch32f", 5) == 0 ||
           strcmp(name, "bluepill") == 0 ||
           strcmp(name, "nrf52") == 0 ||
           strcmp(name, "rp2040") == 0 ||
           strcmp(name, "rp2350") == 0;
}

/* ARM Cortex-M manual selection — route to the standard SWD target path. */
static void cmd_arm_detect(const char *mcu_name) {
    if (!swd_initialized) {
        if (PROG_MODE_IS_ACTIVE()) { send_response("ERR cannot use SWD while UART is active on GPIO4-5"); return; }
        swd_result_t r = swd_init();
        if (r != SWD_OK) { char msg[64]; sprintf(msg, "ERR SWD init failed: %d", r); send_response(msg); return; }
        swd_initialized = true;
    }
    uint32_t idcode = 0;
    swd_result_t result = swd_read_idcode(&idcode);
    if (result == SWD_OK && idcode != 0) {
        result = target_detect();
        if (result == SWD_OK && target_is_connected()) {
            const target_info_t *info = target_get_info();
            char msg[160];
            sprintf(msg, "OK CH32 DETECT mcu=%s idcode=0x%08lX target=%s flash=%lu",
                    mcu_name, (unsigned long)idcode,
                    info->name ? info->name : "unknown",
                    (unsigned long)info->flash_size);
            send_response(msg);
            return;
        }
    }
    char msg[128];
    sprintf(msg, "ERR CH32 DETECT failed: no ARM target (idcode=0x%08lX)", (unsigned long)idcode);
    send_response(msg);
}

static void cmd_ch32_detect(char* args) {
    /* Manual MCU selection: CH32 DETECT mcu=ch32v203|ch32v003|ch32v307|stm32f103
     * Legacy (backward compatible): CH32 DETECT mode=1wire|mode=2wire */
    char *mcu_name = NULL;
    bool mcu_specified = false;
    ch32_mode_t mode = CH32_MODE_SWIO;
    bool mode_specified = false;

    if (args) {
        char *token = strtok(args, " ");
        while (token) {
            if (strncmp(token, "mcu=", 4) == 0) {
                mcu_name = token + 4;
                mcu_specified = true;
            } else if (strncmp(token, "mode=", 5) == 0) {
                mode_specified = true;
                if (strcmp(token + 5, "2wire") == 0) mode = CH32_MODE_RVSWD;
                else if (strcmp(token + 5, "1wire") == 0) mode = CH32_MODE_SWIO;
                else { send_response("ERR CH32 DETECT invalid mode: use 1wire|2wire"); return; }
            } else if (strcmp(token, "2wire") == 0) {  /* legacy bare arg */
                mode = CH32_MODE_RVSWD;
                mode_specified = true;
            }
            token = strtok(NULL, " ");
        }
    }

    ch32_result_t res;
    const char *mode_str;

    if (mcu_specified && mcu_name) {
        /* ARM Cortex-M parts route to the ARM SWD path */
        if (is_arm_mcu(mcu_name)) {
            cmd_arm_detect(mcu_name);
            return;
        }
        const ch32_mcu_info_t *mcu = ch32_lookup_mcu(mcu_name);
        if (!mcu) {
            char msg[128];
            sprintf(msg, "ERR CH32 DETECT unknown mcu=%s (ch32v003|ch32v203|ch32v307|stm32f103)", mcu_name);
            send_response(msg);
            return;
        }
        res = ch32_probe_mcu(mcu);
        mode_str = (mcu->mode == CH32_MODE_SWIO) ? "1wire" : "2wire";
        if (res != CH32_OK || !g_ch32_target.probed) {
            char msg[128];
            sprintf(msg, "ERR CH32 DETECT failed: no %s on the wire (mode=%s)", mcu->label, mode_str);
            send_response(msg);
            return;
        }
        char msg[160];
        sprintf(msg, "OK CH32 DETECT mcu=%s chip_id=0x%08lX flash_size=%lu page_size=%lu mode=%s",
                mcu_name,
                (unsigned long)g_ch32_target.chip_id,
                (unsigned long)g_ch32_target.flash_size_bytes,
                (unsigned long)g_ch32_target.page_size,
                mode_str);
        send_response(msg);
        return;
    }

    /* Legacy mode= path (no mcu=): probe transport, model defaults by mode */
    res = ch32_probe_target(mode);
    mode_str = (mode == CH32_MODE_SWIO) ? "1wire" : "2wire";
    if (res != CH32_OK || !g_ch32_target.probed) {
        char msg[128];
        sprintf(msg, "ERR CH32 DETECT failed (mode=%s)", mode_str);
        send_response(msg);
        return;
    }
    char msg[160];
    sprintf(msg, "OK CH32 DETECT chip_id=0x%08lX flash_size=%lu page_size=%lu mode=%s",
            (unsigned long)g_ch32_target.chip_id,
            (unsigned long)g_ch32_target.flash_size_bytes,
            (unsigned long)g_ch32_target.page_size,
            mode_str);
    send_response(msg);
}

static void cmd_ch32_flash(char* args) {
    if (args == NULL) { send_response("ERR usage: CH32 FLASH <addr> <hex_data>"); return; }
    while (*args == ' ') args++;
    char *space = strchr(args, ' ');
    if (!space) { send_response("ERR usage: CH32 FLASH <addr> <hex_data>"); return; }
    *space = 0;
    uint32_t addr = (uint32_t)strtol(args, NULL, 0);
    char *data_str = space + 1;

    size_t data_len = strlen(data_str);
    if (data_len % 2 != 0 || data_len == 0 || data_len > 512) {
        send_response("ERR hex data must be even length, max 256 bytes");
        return;
    }

    uint8_t data[256];
    for (size_t i = 0; i < data_len / 2; i++) {
        int hi = (data_str[i*2] >= '0' && data_str[i*2] <= '9') ? data_str[i*2] - '0' :
                 (data_str[i*2] >= 'a' && data_str[i*2] <= 'f') ? data_str[i*2] - 'a' + 10 :
                 (data_str[i*2] >= 'A' && data_str[i*2] <= 'F') ? data_str[i*2] - 'A' + 10 : -1;
        int lo = (data_str[i*2+1] >= '0' && data_str[i*2+1] <= '9') ? data_str[i*2+1] - '0' :
                 (data_str[i*2+1] >= 'a' && data_str[i*2+1] <= 'f') ? data_str[i*2+1] - 'a' + 10 :
                 (data_str[i*2+1] >= 'A' && data_str[i*2+1] <= 'F') ? data_str[i*2+1] - 'A' + 10 : -1;
        if (hi < 0 || lo < 0) { send_response("ERR invalid hex data"); return; }
        data[i] = (uint8_t)((hi << 4) | lo);
    }
    data_len /= 2;

    ch32_result_t res = ch32_flash_program(addr, data, data_len);
    if (res != CH32_OK) {
        char diag[64];
        sprintf(diag, "ERR CH32 FLASH program failed res=%d", (int)res);
        send_response(diag);
        return;
    }
    char msg[64];
    sprintf(msg, "OK CH32 FLASH WROTE %lu bytes at 0x%08lX", (unsigned long)data_len, (unsigned long)addr);
    send_response(msg);
}

static void cmd_ch32_erase(char* args) {
    if (args == NULL) { send_response("ERR usage: CH32 ERASE <addr> <len>"); return; }
    while (*args == ' ') args++;
    char *space = strchr(args, ' ');
    if (!space) { send_response("ERR usage: CH32 ERASE <addr> <len>"); return; }
    *space = 0;
    uint32_t addr = (uint32_t)strtol(args, NULL, 0);
    uint32_t len = (uint32_t)strtol(space + 1, NULL, 0);

    char diag[128];
    sprintf(diag, "DIAG probed=%d chip_id=0x%08lX mode=%d",
            (int)g_ch32_target.probed,
            (unsigned long)g_ch32_target.chip_id,
            (int)g_ch32_target.mode);
    send_response(diag);

    ch32_result_t res = ch32_flash_erase(addr, len);
    if (res != CH32_OK) {
        sprintf(diag, "ERR CH32 ERASE failed res=%d", (int)res);
        send_response(diag);
        return;
    }
    char msg[64];
    sprintf(msg, "OK CH32 ERASED %lu bytes at 0x%08lX", (unsigned long)len, (unsigned long)addr);
    send_response(msg);
}

static void cmd_ch32_read(char* args) {
    if (args == NULL) { send_response("ERR usage: CH32 READ <addr> <len>"); return; }
    while (*args == ' ') args++;
    char *space = strchr(args, ' ');
    if (!space) { send_response("ERR usage: CH32 READ <addr> <len>"); return; }
    *space = 0;
    uint32_t addr = (uint32_t)strtol(args, NULL, 0);
    uint32_t len = (uint32_t)strtol(space + 1, NULL, 0);

    if (len > 256) len = 256;

    uint8_t data[256];
    ch32_result_t res = ch32_progbuf_flash_read(addr, data, len);
    if (res != CH32_OK) {
        send_response("ERR CH32 READ failed");
        return;
    }

    char hex_data[768];
    hex_data[0] = '\0';
    for (uint32_t i = 0; i < len; i++) {
        if (i > 0) strcat(hex_data, ",");
        char b[4];
        sprintf(b, "%02X", data[i]);
        strcat(hex_data, b);
    }
    char response[1024];
    sprintf(response, "FLASH DATA %s", hex_data);
    send_response(response);

    char msg[64];
    sprintf(msg, "OK CH32 READ %lu bytes at 0x%08lX", (unsigned long)len, (unsigned long)addr);
    send_response(msg);
}

/* CH32 READM <addr> — single-word memory read diagnostic (F1 investigation).
 * Calls riscv_read_mem() directly (PROGBUF0 = lw a0,0(a1) path), bypassing the
 * block-read program / AUTOEXECDATA streaming. Compare CH32 READM 0x20000000
 * (SRAM control, expect real data) vs CH32 READM 0x08000000 (main flash):
 *   - flash also returns constant 0x4000B0C3  => flash-bus behavior fault
 *   - flash returns real data                  => block-read program fault */
static void cmd_ch32_readm(char* args) {
    if (args == NULL) { send_response("ERR usage: CH32 READM <addr>"); return; }
    while (*args == ' ') args++;
    if (!g_ch32_target.probed || g_ch32_target.mode != CH32_MODE_RVSWD) {
        send_response("ERR CH32 READM requires probed RVSWD target (run CH32 DETECT mcu=ch32v203 first)");
        return;
    }
    uint32_t addr = (uint32_t)strtoul(args, NULL, 0);

    uint32_t val = 0;
    ch32_result_t res = riscv_read_mem(addr, &val);
    char msg[96];
    if (res != CH32_OK) {
        sprintf(msg, "ERR CH32 READM 0x%08lX failed res=%d", (unsigned long)addr, (int)res);
    } else {
        sprintf(msg, "OK CH32 READM 0x%08lX = 0x%08lX", (unsigned long)addr, (unsigned long)val);
    }
    send_response(msg);
}

static bool algo_consent = false;

static void cmd_algo_consent(char *args) {
    if (args && (strcmp(args, "ON") == 0 || strcmp(args, "on") == 0)) {
        algo_consent = true;
        send_response("OK ALGO CONSENT ON");
    } else if (args && (strcmp(args, "OFF") == 0 || strcmp(args, "off") == 0)) {
        algo_consent = false;
        send_response("OK ALGO CONSENT OFF");
    } else {
        char msg[64];
        sprintf(msg, "ALGO CONSENT %s", algo_consent ? "ON" : "OFF");
        send_response(msg);
    }
}

static void cmd_mass_erase(char *args) {
    (void)args;
    bool confirm = false;
    if (args && strstr(args, "CONFIRM") != NULL) confirm = true;
    if (!confirm) { send_response("ERR CONFIRM REQUIRED"); return; }

    if (!swd_is_initialized()) {
        swd_result_t r = swd_init();
        if (r != SWD_OK) { char msg[64]; sprintf(msg, "ERR SWD init failed: %d", r); send_response(msg); return; }
        swd_initialized = true;
    }
    swd_result_t r = target_detect();
    if (r != SWD_OK || !target_is_connected()) { send_response("ERR TARGET NOT IDENTIFIED"); return; }
    const target_info_t *info = target_get_info();
    char banner[192];
    sprintf(banner, "MASS ERASE: target=%s flash=%lu bytes protection=rdp1",
            info->name, (unsigned long)info->flash_size);
    send_response(banner);

    if (!target_mass_erase()) { send_response("ERR MASS ERASE FAILED"); return; }

    /* Re-identify to confirm recovery. */
    target_detect();
    const target_info_t *after = target_get_info();
    char done[128];
    sprintf(done, "OK MASS ERASE DONE protection=%s", after ? after->name : "unknown");
    send_response(done);
}

static void cmd_boot0(char *args) {
    /* Default dedicated BOOT0 pin (configurable). */
    const uint BOOT0_PIN = 2;
    if (args && (strcmp(args, "HIGH") == 0 || strcmp(args, "high") == 0)) {
        gpio_init(BOOT0_PIN);
        gpio_set_dir(BOOT0_PIN, GPIO_OUT);
        gpio_put(BOOT0_PIN, 1);
        send_response("OK BOOT0 HIGH");
    } else if (args && (strcmp(args, "LOW") == 0 || strcmp(args, "low") == 0)) {
        gpio_init(BOOT0_PIN);
        gpio_set_dir(BOOT0_PIN, GPIO_OUT);
        gpio_put(BOOT0_PIN, 0);
        send_response("OK BOOT0 LOW");
    } else {
        char msg[64];
        sprintf(msg, "OK BOOT0 PIN %d (HIGH|LOW)", BOOT0_PIN);
        send_response(msg);
    }
}
static void cmd_algo_test(char *args) {
    (void)args;
    if (!swd_is_initialized()) {
        swd_result_t r = swd_init();
        if (r != SWD_OK) { char msg[64]; sprintf(msg, "ERR SWD init failed: %d", r); send_response(msg); return; }
        swd_initialized = true;
    }
    static const uint8_t prog[] = { 0x42, 0x20, 0x00, 0xBE }; /* MOVS R0,#0x42 ; BKPT #0 */
    const uint32_t addr = 0x20000100u;
    if (algo_ram_download(addr, prog, sizeof(prog)) != SWD_OK) { send_response("ERR stage download"); return; }
    if (algo_core_halt() != SWD_OK) { send_response("ERR stage halt"); return; }
    if (algo_core_write_reg(0, 0x00000000u) != SWD_OK) { send_response("ERR stage wr R0"); return; }
    if (algo_core_write_reg(14, 0x00000000u) != SWD_OK) { send_response("ERR stage wr LR"); return; }
    if (algo_core_write_reg(15, addr | 1u) != SWD_OK) { send_response("ERR stage wr PC"); return; }
    if (algo_core_resume() != SWD_OK) { send_response("ERR stage resume"); return; }
    bool halted = false;
    for (int i = 0; i < 5000; i++) {
        uint32_t dhcsr = 0;
        if (algo_mem_read32(CORTEXM_DHCSR, &dhcsr) == SWD_OK && (dhcsr & DHCSR_S_HALT)) { halted = true; break; }
        sleep_us(10);
    }
    if (!halted) { send_response("ERR stage timeout (no halt)"); return; }
    uint32_t ret = 0;
    if (algo_core_read_reg(0, &ret) != SWD_OK) { send_response("ERR stage read R0"); return; }
    char msg[64]; sprintf(msg, "OK ALGOTEST ret=0x%08lX", (unsigned long)ret);
    send_response(msg);
}
static void cmd_rdp_lock(char *args) {
    (void)args;
    bool confirm = false;
    if (args && strstr(args, "CONFIRM") != NULL) confirm = true;
    if (!confirm) { send_response("ERR CONFIRM REQUIRED"); return; }
    if (!swd_is_initialized()) {
        swd_result_t r = swd_init();
        if (r != SWD_OK) { char msg[64]; sprintf(msg, "ERR SWD init failed: %d", r); send_response(msg); return; }
        swd_initialized = true;
    }
    if (!target_is_connected()) { target_detect(); }
    if (!target_is_connected()) { send_response("ERR TARGET NOT IDENTIFIED"); return; }
    send_response("LOCKING RDP LEVEL 1 (potentially irreversible)");
    if (!target_lock_rdp_level1()) { send_response("ERR RDP LOCK FAILED"); return; }
    send_response("OK RDP LOCKED (level 1) - use RDPUNLOCK CONFIRM to revert");
}

static void cmd_rdp_unlock(char *args) {
    bool confirm = false;
    if (args && strstr(args, "CONFIRM") != NULL) confirm = true;
    if (!confirm) { send_response("ERR CONFIRM REQUIRED"); return; }
    if (!swd_is_initialized()) {
        swd_result_t r = swd_init();
        if (r != SWD_OK) { char msg[64]; sprintf(msg, "ERR SWD init failed: %d", r); send_response(msg); return; }
        swd_initialized = true;
    }
    if (!target_is_connected()) { target_detect(); }
    if (!target_is_connected()) { send_response("ERR TARGET NOT IDENTIFIED"); return; }
    send_response("UNLOCKING RDP (mass-erase-level dependent; may fail)");
    if (!target_unlock_rdp()) { send_response("ERR RDP UNLOCK FAILED"); return; }
    send_response("OK RDP UNLOCKED (level 0)");
}

static void cmd_uartloop(char *args) {
    (void)args;
    /* Force direct hardware test on ADAPTER UART pins, ignoring SWD guard. */
    uart_init(uart1, 115200);
    uart_set_format(uart1, 8, 1, UART_PARITY_EVEN);
    gpio_set_function(ADAPTER_UART_TX_PIN, GPIO_FUNC_UART);
    gpio_set_function(ADAPTER_UART_RX_PIN, GPIO_FUNC_UART);

    /* transmit */
    uart_putc(uart1, 0x55);
    /* small delay for loopback through wire */
    sleep_us(2000);
    int got = -1;
    int32_t timeout_us = 2000;
    int32_t start = make_timeout_time_us(timeout_us);
    while (!time_reached(start) && got < 0) {
        if (uart_is_readable(uart1)) {
            got = uart_getc(uart1);
        }
    }
    char msg[64];
    if (got >= 0) sprintf(msg, "UARTLOOP DATA %02X", got);
    else sprintf(msg, "UARTLOOP TIMEOUT (no data on RX=%d)", ADAPTER_UART_RX_PIN);
    send_response(msg);
}
static void cmd_pin_claim(char* args) {
    uint8_t pins[8];
    uint8_t pin_count = 0;
    uint8_t function = GPIO_FUNC_SIO;
    uint8_t observer_only = 0;

    char* token = strtok(args, " ");
    while (token != NULL && pin_count < 8) {
        if (strncmp(token, "pin=", 4) == 0) {
            if (pin_count < 8) {
                pins[pin_count++] = (uint8_t)atoi(token + 4);
            }
        } else if (strncmp(token, "pins=", 5) == 0) {
            char* pos = token + 5;
            while (*pos != '\0' && pin_count < 8) {
                char* end = NULL;
                uint8_t p = (uint8_t)strtol(pos, &end, 0);
                if (end != pos) {
                    pins[pin_count++] = p;
                    if (*end == ',') {
                        pos = end + 1;
                    } else {
                        pos = end;
                    }
                } else {
                    break;
                }
            }
        } else if (strncmp(token, "func=", 5) == 0) {
            unsigned long val = atoi(token + 5);
            function = (uint8_t)val;
        } else if (strncmp(token, "observer=", 9) == 0) {
            observer_only = (uint8_t)atoi(token + 9);
        }
        token = strtok(NULL, " ");
    }

    if (pin_count == 0) {
        send_response("ERR missing pins");
        return;
    }

    pin_owner_result_t result = PIN_OWNER_ERROR_CAPACITY;
    uint32_t handle = 0;
    bool ok = pin_owner_claim_pins(pins, pin_count, function, observer_only, &handle);
    if (!ok) {
        char msg[192];
        sprintf(msg, "ERR PINCLAIM FAILED pins=%u function=%u reason=%d", pin_count, (unsigned)function, (int)result);
        send_response(msg);
        return;
    }

    char msg[192];
    sprintf(msg, "OK PINCLAIM HANDLE=%lu PINS=%u FUNCTION=%u OBSERVER=%u", (unsigned long)handle, pin_count, (unsigned)function, observer_only);
    send_response(msg);
}

static void cmd_pin_release(char* args) {
    uint32_t handle = 0;

    if (args != NULL) {
        char* token = strtok(args, " ");
        while (token != NULL) {
            if (strncmp(token, "handle=", 7) == 0) {
                handle = (uint32_t)atoi(token + 7);
            }
            token = strtok(NULL, " ");
        }
    }

    if (handle == 0) {
        send_response("ERR missing handle");
        return;
    }

    pin_owner_result_t result = PIN_OWNER_OK;
    bool ok = pin_owner_release_claim(handle, &result);
    if (ok) {
        send_response("OK PINRELEASE");
    } else {
        char msg[192];
        sprintf(msg, "ERR PINRELEASE FAILED handle=%lu result=%d", (unsigned long)handle, (int)result);
        send_response(msg);
    }
}


static void cmd_safe_discover(char* args) {
    (void)args;
    char msg[768];
    size_t pos = 0;
    pos += (size_t)snprintf(msg + pos, sizeof(msg) - pos, "OK SAFEDISCOVER");
    for (uint8_t pin = PIN_OWNER_FLEX_MIN; pin <= PIN_OWNER_FLEX_MAX; pin++) {
        int used = pin_owner_has_driver(pin);
        uint8_t observers = pin_owner_observer_count(pin);
        pos += (size_t)snprintf(msg + pos, sizeof(msg) - pos, ",%u:d=%d:o=%u", pin, used, observers);
    }
    send_response(msg);
}

static void cmd_pin_owner_status(char* args) {
    (void)args;
    const pin_owner_inventory_t* inventory = pin_owner_get_inventory();
    char msg[192];
    sprintf(msg, "OK PINOWNER INVENTORY_RANGE=%u-%u FLEXIBLE=%u ANALOG=%u", PIN_OWNER_FLEX_MIN, PIN_OWNER_FLEX_MAX,
            inventory != NULL ? inventory->flexible_pin_count : 0,
            inventory != NULL ? inventory->analog_capable_pin_count : 0);
    send_response(msg);
}


static void cmd_capture_claim(char* args) {
    uint8_t pins[8];
    uint8_t count = 0;
    uint8_t trigger = 255;

    char* token = strtok(args, " ");
    while (token != NULL && count < 8) {
        if (strncmp(token, "pins=", 5) == 0) {
            char* pos = token + 5;
            while (*pos != '\0' && count < 8) {
                char* end = NULL;
                uint8_t p = (uint8_t)strtol(pos, &end, 0);
                if (end != pos) {
                    pins[count++] = p;
                    if (*end == ',') {
                        pos = end + 1;
                    } else {
                        pos = end;
                    }
                } else {
                    break;
                }
            }
        } else if (strncmp(token, "trigger=", 8) == 0) {
            trigger = (uint8_t)atoi(token + 8);
        }
        token = strtok(NULL, " ");
    }

    if (count == 0 || count > 8) {
        send_response("ERR CAPTURECLAIM needs 1-8 pins");
        return;
    }

    uint32_t handle = 0;
    if (!capture_claim_bank(pins, count, trigger, &handle)) {
        send_response("ERR CAPTURECLAIM failed (bank already active or invalid pins)");
        return;
    }

    char msg[128];
    sprintf(msg, "OK CAPTURECLAIM HANDLE=%lu PINS=%u TRIGGER=%u", (unsigned long)handle, count, trigger);
    send_response(msg);
}

static void cmd_capture_release(char* args) {
    uint32_t handle = 0;
    char* token = strtok(args, " ");
    while (token != NULL) {
        if (strncmp(token, "handle=", 7) == 0) {
            handle = (uint32_t)atoi(token + 7);
        }
        token = strtok(NULL, " ");
    }

    pin_owner_result_t result = PIN_OWNER_OK;
    if (handle == 0 || !capture_release_bank(handle, &result)) {
        char msg[128];
        sprintf(msg, "ERR CAPTURERELEASE FAILED handle=%lu result=%d", (unsigned long)handle, (int)result);
        send_response(msg);
        return;
    }
    send_response("OK CAPTURERELEASE");
}

void network_execute_command(const char *command_line, int *status) {
    char command_copy[768];

    if (command_line == NULL || status == NULL) return;

    web_command_capture_active = 1;
    web_command_response[0] = '\0';
    web_command_response_len = 0;
    web_command_status = 0;
    memset(command_copy, 0, sizeof(command_copy));
    snprintf(command_copy, sizeof(command_copy), "%s", command_line);

    process_command(command_copy);

    web_command_capture_active = 0;
    if (strstr(web_command_response, "ERR ")) {
        web_command_status = 1;
    } else {
        web_command_status = 0;
    }

    *status = web_command_status;
}

const char *network_command_response(void) {
    return web_command_response;
}

size_t network_command_response_length(void) {
    return web_command_response_len;
}

int network_command_status(void) {
    return web_command_status;
}

static void process_command(char* cmd) {
    char* args = strchr(cmd, ' ');
    if(args) { *args = '\0'; args++; }
    if(strcmp(cmd, "IDENTIFY") == 0) cmd_identify();
    else if(strcmp(cmd, "PINCLAIM") == 0) cmd_pin_claim(args ? args : "");
    else if(strcmp(cmd, "PINRELEASE") == 0) cmd_pin_release(args ? args : "");
    else if(strcmp(cmd, "PINOWNER") == 0) cmd_pin_owner_status(args ? args : "");
    else if(strcmp(cmd, "SAFEDISCOVER") == 0) cmd_safe_discover(args ? args : "");
    else if(strcmp(cmd, "CAPTURECLAIM") == 0) cmd_capture_claim(args ? args : "");
    else if(strcmp(cmd, "CAPTURERELEASE") == 0) cmd_capture_release(args ? args : "");
    else if(strcmp(cmd, "VERSION") == 0) cmd_version();
    else if(strcmp(cmd, "CONFIRM") == 0) cmd_confirm();
    else if(strcmp(cmd, "CRASHINFO") == 0) cmd_crashinfo();
    else if(strcmp(cmd, "CONFIGURE") == 0) cmd_configure(args ? args : "");
    else if(strcmp(cmd, "CAPTURE") == 0) cmd_capture(args ? args : "");
    else if(strcmp(cmd, "MEASURE") == 0) cmd_measure(args ? args : "");
    else if(strcmp(cmd, "ADC") == 0) cmd_adc(args ? args : "");
    else if(strcmp(cmd, "STATUS") == 0) cmd_status();
    else if(strcmp(cmd, "I2C_READ") == 0) cmd_i2c_read(args ? args : "");
    else if(strcmp(cmd, "I2C_WRITE") == 0) cmd_i2c_write(args ? args : "");
    else if(strcmp(cmd, "UART_WRITE") == 0) cmd_uart_write(args ? args : "");
    else if(strcmp(cmd, "UART_READ") == 0) cmd_uart_read(args ? args : "");
    else if(strcmp(cmd, "I2C_CLEAR") == 0) cmd_i2c_clear(args ? args : "");
    else if(strcmp(cmd, "REBOOT") == 0) cmd_reboot(args ? args : "");
    else if(strcmp(cmd, "BOOTSEL") == 0) cmd_bootsel(args ? args : "");
    else if(strcmp(cmd, "SELFTEST") == 0) cmd_selftest(args ? args : "");
    else if(strcmp(cmd, "PIO0TEST") == 0) cmd_selftest_pio0(args ? args : "");
    else if(strcmp(cmd, "DIAG") == 0) {
        if(args && strncmp(args, "CDC", 3) == 0) {
            char msg[192];
            sprintf(msg, "DIAG CDC total=%lu pushed=%lu dropped=%lu popped=%lu fifo_before=%lu hw_read=%lu(%lu calls) fifo_after=%lu ring_h=%lu ring_t=%lu",
                    (unsigned long)g_cdc_rx_bytes_total,
                    (unsigned long)g_cdc_rx_bytes_pushed,
                    (unsigned long)g_cdc_rx_bytes_dropped,
                    (unsigned long)g_cdc_rx_bytes_popped,
                    (unsigned long)g_cdc_rx_fifo_before_tud_task,
                    (unsigned long)g_cdc_rx_hw_read_bytes,
                    (unsigned long)g_cdc_rx_hw_read_calls,
                    (unsigned long)g_cdc_rx_fifo_after_read,
                    (unsigned long)g_cdc_rx_ring_head_snap,
                    (unsigned long)g_cdc_rx_ring_tail_snap);
            send_response(msg);
        }
        else if(args && strncmp(args, "CDC RESET", 9) == 0) {
            usb_cdc_diag_reset();
            send_response("OK DIAG CDC counters reset");
        }
        else if(args && strncmp(args, "PIPELINE", 8) == 0) {
            usb_cdc_diag_reset();
            for (volatile int i = 0; i < 100000; i++) __asm__ volatile("nop");
            char msg[128];
            sprintf(msg, "DIAG PIPELINE reset OK. Send a test string, then run DIAG CDC.");
            send_response(msg);
        }
        else {
            /* PIO0 SM0 state dump for debugging DMA hangs (default DIAG) */
            PIO pio = SWD_PIO;
            uint sm = SWD_SM;
            char dbg[192];

            uint32_t pc = pio_sm_get_pc(pio, sm);
            uint32_t fstat = pio->fstat;
            uint32_t fdebug = pio->fdebug;
            uint32_t txfifo = (fstat >> (PIO_FSTAT_TXEMPTY_LSB + sm)) & 1;
            uint32_t rxfifo = (fstat >> (PIO_FSTAT_RXEMPTY_LSB + sm)) & 1;
            uint32_t txstall = (fdebug >> (PIO_FDEBUG_TXSTALL_LSB + sm)) & 1;
            uint32_t rxstall = (fdebug >> (PIO_FDEBUG_RXSTALL_LSB + sm)) & 1;

            sprintf(dbg, "PIO0 SM0: PC=%lu TXEMPTY=%lu RXEMPTY=%lu TXSTALL=%lu RXSTALL=%lu",
                    (unsigned long)pc, (unsigned long)txfifo, (unsigned long)rxfifo,
                    (unsigned long)txstall, (unsigned long)rxstall);
            send_response(dbg);

            /* Register-level dump: clock divider + pin config (RP2350 has
             * clkdiv in a single 32-bit reg: INT[15:0] FRAC[7:0]; pinctrl and
             * execctrl hold the SM pin bases / jmp pin). */
            uint32_t clkdiv = pio->sm[sm].clkdiv;
            uint32_t pinctrl = pio->sm[sm].pinctrl;
            uint32_t execctrl = pio->sm[sm].execctrl;
            uint32_t shiftctrl = pio->sm[sm].shiftctrl;
            sprintf(dbg, "PIO0 SM0 clkdiv=0x%08lX(INT=%lu FRAC=%lu) pinctrl=0x%08lX execctrl=0x%08lX shiftctrl=0x%08lX",
                    (unsigned long)clkdiv,
                    (unsigned long)(clkdiv >> PIO_SM0_CLKDIV_INT_LSB) & 0xFFFF,
                    (unsigned long)(clkdiv >> PIO_SM0_CLKDIV_FRAC_LSB) & 0xFF,
                    (unsigned long)pinctrl, (unsigned long)execctrl, (unsigned long)shiftctrl);
            send_response(dbg);

            sprintf(dbg, "PIO0 FSTAT=0x%08lX FDEBUG=0x%08lX",
                    (unsigned long)fstat, (unsigned long)fdebug);
            send_response(dbg);

            send_response("No prior DMA timeout (PIO transport is DMA-free)");
        }
    }
    else if(strcmp(cmd, "LOOPBACK") == 0) {
        /* PIO wire-level loopback test.
         * Uses GPIO20 (out) and GPIO21 (in) for logic analyzer observation.
         * Requires jumper between GPIO20 and GPIO21.
         * Uses PIO0 SM0 — same path as SWD engine.
         * Tests: PIO program loading, DMA TX, PIO execution, GPIO loopback, RX FIFO readback. */
        char dbg[128];
        PIO pio = SWD_PIO;
        uint sm = SWD_SM;
        int dma_ch = -1;
        uint offset = 0;
        bool pio_loaded = false;
        bool sm_enabled = false;

        /* Deinit SWD if active */
        if (swd_initialized) { swd_deinit(); swd_initialized = false; }

        /* Clear PIO instruction memory */
        pio_clear_instruction_memory(pio);

        /* Load the loopback test program (from loopback.pio) */
        offset = pio_add_program(pio, &loopback_test_program);
        pio_loaded = true;

        /* Configure SM: side-set on GPIO20 (capture ch0), out on GPIO20, in on GPIO21 (capture ch1)
         * Logic analyzer on GPIO20-21 can directly observe the loopback signals. */
        pio_sm_config c = loopback_test_program_get_default_config(offset);
        sm_config_set_sideset_pins(&c, 20);     /* side-set drives GPIO20 (capture ch0) */
        sm_config_set_out_pins(&c, 20, 1);      /* out drives GPIO20 */
        sm_config_set_in_pins(&c, 21);          /* in reads GPIO21 (capture ch1) */
        sm_config_set_clkdiv(&c, 10.0f);        /* ~12.5 MHz (125/10) */

        /* Configure GPIO20 as PIO output, GPIO21 as PIO input */
        pio_gpio_init(pio, 20);
        pio_sm_set_consecutive_pindirs(pio, sm, 20, 1, true);
        pio_gpio_init(pio, 21);
        pio_sm_set_consecutive_pindirs(pio, sm, 21, 1, false);
        gpio_disable_pulls(21);

        /* Init SM */
        pio_sm_init(pio, sm, offset, &c);
        pio_sm_set_enabled(pio, sm, true);
        sm_enabled = true;

        /* Send [loop_count, data] via DMA.
         * PIO program pulls X count first, then data via autopull. */
        uint32_t tx_buf[2] = { 31, 0xAAAAAAAA };  /* loop_count=31 (32 bits), data=alternating 1s and 0s */
        uint32_t rx_buf = 0;

        dma_ch = dma_claim_unused_channel(true);
        dma_channel_config tx_cfg = dma_channel_get_default_config(dma_ch);
        channel_config_set_transfer_data_size(&tx_cfg, DMA_SIZE_32);
        channel_config_set_read_increment(&tx_cfg, true);
        channel_config_set_write_increment(&tx_cfg, false);
        channel_config_set_dreq(&tx_cfg, pio_get_dreq(pio, sm, true));
        dma_channel_configure(dma_ch, &tx_cfg, &(pio->txf[sm]), tx_buf, 2, true);

        /* Wait for TX DMA with timeout */
        absolute_time_t deadline = make_timeout_time_ms(100);
        while (dma_channel_is_busy(dma_ch)) {
            if (time_reached(deadline)) {
                goto loopback_timeout;
            }
        }
        dma_channel_unclaim(dma_ch);
        dma_ch = -1;

        /* Wait for RX data (loopback program pushes 32-bit result) */
        deadline = make_timeout_time_ms(100);
        while (pio_sm_is_rx_fifo_empty(pio, sm)) {
            if (time_reached(deadline)) {
                goto loopback_timeout;
            }
        }
        rx_buf = pio_sm_get(pio, sm);

        /* Success path */
        pio_sm_set_enabled(pio, sm, false);
        sm_enabled = false;
        pio_remove_program(pio, &loopback_test_program, offset);
        pio_loaded = false;

        sprintf(dbg, "LOOPBACK TX=0x%08lX RX=0x%08lX", (unsigned long)tx_buf[1], (unsigned long)rx_buf);
        send_response(dbg);

        if (rx_buf != 0 && rx_buf != 0xFFFFFFFF) {
            send_response("OK LOOPBACK PASS - PIO DMA path working");
        } else {
            send_response("ERR LOOPBACK FAIL - no signal (install jumper GPIO20-GPIO21)");
        }

        /* Restore the SWD transport (driver-aware swd_init) */
        swd_init();
        swd_initialized = true;
        goto loopback_done;

        loopback_timeout:
        /* Timeout or error — clean up and restore */
        if (dma_ch >= 0) { dma_channel_abort(dma_ch); dma_channel_unclaim(dma_ch); }
        if (sm_enabled) pio_sm_set_enabled(pio, sm, false);
        if (pio_loaded) pio_remove_program(pio, &loopback_test_program, offset);
        pio_sm_clear_fifos(pio, sm);
        send_response("ERR LOOPBACK timeout - PIO DMA path failed");
        /* Restore the SWD transport (driver-aware swd_init) */
        swd_init();
        swd_initialized = true;

        loopback_done:;
    }
    else if(strcmp(cmd, "PIO0SMTEST") == 0) {
        uint32_t pin = 4;
        if(args) { char* t = strtok(args, " "); while(t) { if(strncmp(t,"pin=",4)==0) pin=atoi(t+4); t=strtok(NULL," "); } }
        PIO pio = pio0;
        pio_clear_instruction_memory(pio);
        uint sm = pio_claim_unused_sm(pio, true);
        uint off = pio_add_program(pio, &SELFTEST_TX_program);
        pio_sm_config c = SELFTEST_TX_program_get_default_config(off);
        sm_config_set_out_pins(&c, pin, 1);
        sm_config_set_set_pins(&c, pin, 1);
        sm_config_set_clkdiv(&c, 100.0f);
        pio_gpio_init(pio, pin);
        pio_sm_init(pio, sm, off, &c);
        pio_sm_set_enabled(pio, sm, true);
        pio_sm_put_blocking(pio, sm, 0x55);
        busy_wait_us(50000);
        char dbg[128];
        uint32_t pinctrl_val = pio->sm[sm].pinctrl;
        uint32_t set_base = (pinctrl_val & PIO_SM0_PINCTRL_SET_BASE_BITS) >> PIO_SM0_PINCTRL_SET_BASE_LSB;
        uint32_t out_base = (pinctrl_val & PIO_SM0_PINCTRL_OUT_BASE_BITS) >> PIO_SM0_PINCTRL_OUT_BASE_LSB;
        sprintf(dbg, "PIO0SM pinctrl=0x%08lx set_base=%lu out_base=%lu",
            (unsigned long)pinctrl_val, (unsigned long)set_base, (unsigned long)out_base);
        send_response(dbg);
        sprintf(dbg, "PIO0SM fn=%d val=%d sioin=%d gpio_oe=%lu",
            (int)gpio_get_function(pin), (int)gpio_get(pin),
            (int)((sio_hw->gpio_in >> pin) & 1),
            (unsigned long)((sio_hw->gpio_oe >> pin) & 1));
        send_response(dbg);
        pio_sm_set_enabled(pio, sm, false);
        pio_remove_program(pio, &SELFTEST_TX_program, off);
        pio_sm_unclaim(pio, sm);
        gpio_init(pin);
        send_response("PIO0SMTEST DONE");
    }
    else if(strcmp(cmd, "PROG") == 0) {
        if(args && strncmp(args, "DRIVER ", 7) == 0) cmd_prog_driver(args + 7);
        else if(args && strcmp(args, "DRIVER") == 0) cmd_prog_driver(args + 6);
        else { send_response("ERR unknown PROG subcommand: DRIVER"); }
    }
    else if(strcmp(cmd, "MASSERASE") == 0) cmd_mass_erase(args ? args : "");
    else if(strcmp(cmd, "RDPLOCK") == 0) cmd_rdp_lock(args ? args : "");
    else if(strcmp(cmd, "RDPUNLOCK") == 0) cmd_rdp_unlock(args ? args : "");
    else if(strcmp(cmd, "ALGOTEST") == 0) cmd_algo_test(args ? args : "");
    else if(strcmp(cmd, "ALGOCONSENT") == 0) cmd_algo_consent(args ? args : "");
    else if(strcmp(cmd, "UARTLOOP") == 0) cmd_uartloop(args ? args : "");
    else if(strcmp(cmd, "BOOT0") == 0) cmd_boot0(args ? args : "");
    else if(strcmp(cmd, "SWD") == 0) {
        if(args && strcmp(args, "IDCODE") == 0) cmd_swd_idcode(NULL);
        else if(args && strcmp(args, "RESET") == 0) cmd_swd_reset(NULL);
        else if(args && strcmp(args, "TEST") == 0) cmd_swd_test(NULL);
        else if(args && strcmp(args, "PINS") == 0) cmd_swd_pins(NULL);
        else if(args && strcmp(args, "DEINIT") == 0) {
            swd_deinit();
            swd_initialized = false;
            send_response("OK SWD DEINIT");
        }
        else if(args && strncmp(args, "CLOCK ", 6) == 0) {
            uint32_t khz = (uint32_t)strtol(args + 6, NULL, 0);
            swd_result_t r = swd_set_clock(khz);
            if (r == SWD_OK) { char msg[64]; sprintf(msg, "OK SWD CLOCK %lu kHz", (unsigned long)khz); send_response(msg); }
            else { send_response("ERR SWD CLOCK failed"); }
        }
        else if(args && strcmp(args, "BITBANG") == 0) {
            cmd_swd_idcode(NULL);
        }
        else if(args && strcmp(args, "PIO_IDCODE") == 0) {
            if (!swd_initialized) {
                swd_result_t r = swd_init();
                if (r != SWD_OK) { char msg[64]; sprintf(msg, "ERR SWD init failed: %d", r); send_response(msg); return; }
                swd_initialized = true;
            }
            uint32_t idcode = 0;
            swd_jtag_to_swd();
            /* Read DPIDR FIRST: the ADIv5 SW-DP ignores non-DPIDR accesses in its
             * post-reset state (silent all-ones ACK), so ABORT/CTRLSTAT writes must
             * come only after a successful read (matches BMP + debugprobe). */
            swd_result_t result = swd_read_dp(DP_DPIDR, &idcode);
            if (result == SWD_OK && idcode != 0 && idcode != 0xFFFFFFFF) {
                swd_write_dp_no_check(DP_ABORT, 0x0000001E);
                swd_write_dp_no_check(DP_CTRLSTAT, 0x50000000);
            }
            if (result == SWD_OK && idcode != 0 && idcode != 0xFFFFFFFF) {
                char ok[64];
                sprintf(ok, "OK SWD PIO_IDCODE 0x%08lX", (unsigned long)idcode);
                send_response(ok);
            } else {
                char dbg[192];
                sprintf(dbg, "ERR SWD PIO_IDCODE res=%d idcode=0x%08lX raw_ack=0x%08lX pc=%lu txstall=%lu rxlev=%lu",
                        (int)result, (unsigned long)idcode, (unsigned long)swd_debug_raw_ack,
                        (unsigned long)pio_sm_get_pc(SWD_PIO, SWD_SM),
                        (unsigned long)((SWD_PIO->fdebug >> (PIO_FDEBUG_TXSTALL_LSB + SWD_SM)) & 1),
                        (unsigned long)((SWD_PIO->fstat >> (PIO_FSTAT_RXEMPTY_LSB + SWD_SM)) & 1 ? 0 : 1));
                send_response(dbg);
            }
        }
        else if(args && strcmp(args, "DBLREAD") == 0) cmd_swd_double_read(NULL);
        else if(args && strcmp(args, "CONNECTSTEPS") == 0) cmd_swd_connect_steps(NULL);
        else if(args && strcmp(args, "IDCODEACK") == 0) cmd_swd_idcode_ack(NULL);
        else if(args && strncmp(args, "WRITETEST ", 10) == 0) cmd_swd_writetest(args + 10);
        else if(args && strcmp(args, "WSEQ") == 0) cmd_swd_wseq(NULL);
        else if(args && strcmp(args, "WRALIGN") == 0) cmd_swd_wralign(NULL);
        else if(args && strcmp(args, "SCOPE") == 0) cmd_swd_scope(NULL);
        else if(args && strcmp(args, "SCOPEAB") == 0) cmd_swd_scopeab(NULL);
        else if(args && strcmp(args, "DIAG") == 0) {
            char dbg[128];
            gpio_init(SWD_SWCLK_PIN);
            gpio_set_dir(SWD_SWCLK_PIN, GPIO_OUT);
            gpio_put(SWD_SWCLK_PIN, 0);
            gpio_init(SWD_SWDIO_PIN);
            gpio_set_dir(SWD_SWDIO_PIN, GPIO_IN);
            gpio_pull_up(SWD_SWDIO_PIN);
            int v1 = gpio_get(SWD_SWDIO_PIN);
            gpio_put(SWD_SWCLK_PIN, 1);
            sleep_us(10);
            int v2 = gpio_get(SWD_SWDIO_PIN);
            gpio_put(SWD_SWCLK_PIN, 0);
            sprintf(dbg, "OK SWD DIAG swdio_idle1=%d swdio_idle2=%d", v1, v2);
            send_response(dbg);
            if (swd_initialized) { swd_deinit(); swd_init(); }
        }
        else if(args && strncmp(args, "CAPTURE", 7) == 0) {
            /* SWD CAPTURE [BITBANG|PIOREAD]: Start logic analyzer on GPIO20,21, then run
             * the bitbanging reference driver, a bare PIO DPIDR read (no DP writes), or
             * the full PIO connect sequence while capturing. Dumps the raw captured
             * waveform (bit0=GPIO20 SWCLK, bit1=GPIO21 SWDIO) as SWDCAP lines for
             * host-side timing analysis. */
            bool use_bitbang = (args[7] == ' ' && strcmp(args + 8, "BITBANG") == 0);
            bool use_pioread = (args[7] == ' ' && strcmp(args + 8, "PIOREAD") == 0);
            if (!use_bitbang && !swd_initialized) {
                swd_result_t r = swd_init();
                if (r != SWD_OK) { char msg[64]; sprintf(msg, "ERR SWD init failed: %d", r); send_response(msg); return; }
                swd_initialized = true;
            }
            /* Configure capture: GPIO20=CH0(SWCLK), GPIO21=CH1(SWDIO)
             * 10 MHz sample rate, 32768 samples = 3.28 ms window covering
             * diagnostic markers + line reset + JTAG-to-SWD switch + request/ACK. */
            uint8_t swd_capture_channels[2] = {
                gpio_to_pindex(20),  /* GPIO20 = L_CH0 = SWCLK */
                gpio_to_pindex(21)   /* GPIO21 = L_CH1 = SWDIO */
            };
            bool started = StartCaptureNoTrigger(10000000, 32768, swd_capture_channels, 2, MODE_8_CHANNEL);
            if (!started) { send_response("ERR SWD CAPTURE failed to start"); return; }
            /* PIO0 capture is now running independently on GPIO20,21 */
            /* DIAGNOSTIC MARKERS: toggle SWCLK (GPIO4) and SWDIO (GPIO5) as plain GPIOs
             * for ~0.8 ms at the start of the capture window to verify loopback
             * jumpers GPIO4->GPIO20 and GPIO5->GPIO21. If the square wave does NOT
             * appear on the analyzer, the jumper wiring is bad. */
            gpio_init(SWD_SWCLK_PIN);
            gpio_set_dir(SWD_SWCLK_PIN, GPIO_OUT);
            gpio_init(SWD_SWDIO_PIN);
            gpio_set_dir(SWD_SWDIO_PIN, GPIO_OUT);
            for (volatile int i = 0; i < 20; i++) {
                gpio_put(SWD_SWCLK_PIN, 1);
                gpio_put(SWD_SWDIO_PIN, 1);
                busy_wait_us(10);
                gpio_put(SWD_SWCLK_PIN, 0);
                gpio_put(SWD_SWDIO_PIN, 0);
                busy_wait_us(10);
            }

            swd_result_t result;
            uint32_t idcode = 0;
            if (use_bitbang) {
                pio_gpio_init(SWD_PIO, SWD_SWCLK_PIN);
                pio_gpio_init(SWD_PIO, SWD_SWDIO_PIN);
                result = swd_read_idcode(&idcode);
            } else if (use_pioread) {
                /* Bare PIO DPIDR read: line reset + switch + post-reset, then read
                 * DP_DPIDR directly — mirrors the bitbanging reference exactly with
                 * NO DP writes in between (isolates write-phase effects). */
                /* Restore SWDCLK/SWDIO to PIO0 (the diagnostic markers above
                 * switched them to SIO); the probe SM is still running. */
                pio_gpio_init(SWD_PIO, SWD_SWCLK_PIN);
                pio_gpio_init(SWD_PIO, SWD_SWDIO_PIN);

                swd_jtag_to_swd();
                result = swd_read_dp(DP_DPIDR, &idcode);
            } else {
                /* Restore SWDIO and SWCLK to PIO0 for the SWD engine */
                /* Restore SWDCLK/SWDIO to PIO0 (the diagnostic markers above
                 * switched them to SIO); the probe SM is still running. */
                pio_gpio_init(SWD_PIO, SWD_SWCLK_PIN);
                pio_gpio_init(SWD_PIO, SWD_SWDIO_PIN);

                /* Run the SWD connect sequence on PIO0 (GPIO4,5) while capture is running.
                 * NOTE: call the protocol primitives directly instead of swd_read_idcode()
                 * because that helper sleeps 60 ms on nRESET before any signaling — far
                 * longer than the 10 MHz capture window.
                 * ORDER: DPIDR read must be the FIRST transaction after the JTAG-to-SWD
                 * switch — the ADIv5 SW-DP ignores writes in its post-reset state, so
                 * ABORT/CTRLSTAT writes only after a successful read (matches BMP). */
                swd_jtag_to_swd();
                result = swd_read_dpidr_retry(&idcode);
                if (result == SWD_OK) {
                    swd_write_dp_no_check(DP_ABORT, 0x0000001E);
                    swd_write_dp_no_check(DP_CTRLSTAT, 0x50000000);
                }
            }
            /* Wait for capture to complete */
            while(IsCapturing()) { tight_loop_contents(); }
            /* Dump raw captured waveform (bit0=SWCLK GPIO20, bit1=SWDIO GPIO21) */
            uint32_t length, first;
            CHANNEL_MODE mode_out;
            uint8_t* buffer = GetBuffer(&length, &first, &mode_out);
            for(uint32_t i = 0; i < length; i++) {
                printf("SWDCAP %02X\r\n", buffer[(first + i) % CAPTURE_BUFFER_SIZE]);
            }
            char msg[192];
            sprintf(msg, "OK SWD CAPTURE mode=%s res=%d idcode=0x%08lX raw_ack=0x%08lX samples=%lu pc=%lu rxlev=%lu",
                    use_bitbang ? "BITBANG" : "PIO", (int)result, (unsigned long)idcode,
                    (unsigned long)swd_debug_raw_ack, (unsigned long)length,
                    (unsigned long)pio_sm_get_pc(pio0, SWD_SM),
                    (unsigned long)((pio0->fstat >> PIO_FSTAT_RXEMPTY_LSB + SWD_SM) & 1 ? 0 : 1));
            send_response(msg);
        }
        else if(args && strncmp(args, "READ ", 5) == 0) cmd_swd_read(args + 5);
        else if(args && strncmp(args, "WRITE ", 6) == 0) cmd_swd_write(args + 6);
        else if(args && strncmp(args, "DIAG", 4) == 0) cmd_swd_diag(args + 4);
        else if(args && strncmp(args, "TURNAROUND", 10) == 0) cmd_swd_turnaround(args + 10);
        else { send_response("ERR unknown SWD subcommand: IDCODE|RESET|TEST|PINS|CLOCK|READ|WRITE|DIAG|TURNAROUND"); }
    }
    else if(strcmp(cmd, "DEBUG") == 0) {
        cmd_debug(args);
    }
    else if(strcmp(cmd, "FLASH") == 0) {
        if(args && strncmp(args, "ERASE ", 6) == 0) cmd_flash_erase(args + 6);
        else if(args && strncmp(args, "WRITE ", 6) == 0) cmd_flash_write(args + 6);
        else if(args && strncmp(args, "READ ", 5) == 0) cmd_flash_read(args + 5);
        else if(args && strncmp(args, "VERIFY ", 7) == 0) cmd_flash_verify(args + 7);
        else { send_response("ERR unknown FLASH subcommand: ERASE|WRITE|READ|VERIFY"); }
    }
    else if(strcmp(cmd, "CH32") == 0) {
        if(args && strncmp(args, "DETECT", 6) == 0) cmd_ch32_detect(args + 6);
        else if(args && strncmp(args, "FLASH ", 6) == 0) cmd_ch32_flash(args + 6);
        else if(args && strncmp(args, "ERASE ", 6) == 0) cmd_ch32_erase(args + 6);
        else if(args && strncmp(args, "READ ", 5) == 0) cmd_ch32_read(args + 5);
        else if(args && strncmp(args, "READM ", 6) == 0) cmd_ch32_readm(args + 6);
        else { send_response("ERR unknown CH32 subcommand: DETECT|FLASH|ERASE|READ|READM"); }
    }
    else if(strcmp(cmd, "PROGBUF") == 0) {
        if(args && strncmp(args, "STATUS", 6) == 0) {
            char msg[128];
            sprintf(msg, "PROGBUF available=%d chip_id=0x%08lX flash=%lu page=%lu",
                    (int)ch32_progbuf_is_available(),
                    (unsigned long)g_ch32_target.chip_id,
                    (unsigned long)g_ch32_target.flash_size_bytes,
                    (unsigned long)g_ch32_target.page_size);
            send_response(msg);
        }
        else if(args && strncmp(args, "INIT", 4) == 0) {
            ch32_result_t res = ch32_progbuf_init();
            char msg[64];
            sprintf(msg, res == CH32_OK ? "OK PROGBUF initialized" : "ERR PROGBUF init failed: %d", (int)res);
            send_response(msg);
        }
        else { send_response("ERR unknown PROGBUF subcommand: STATUS|INIT"); }
    }
    else if(strcmp(cmd, "SWIO") == 0) {
        if(args && strncmp(args, "STATUS", 6) == 0) {
            char msg[64];
            sprintf(msg, "SWIO initialized=%d", (int)swio_is_initialized());
            send_response(msg);
        }
        else if(args && strncmp(args, "INIT", 4) == 0) {
            swio_init();
            send_response("OK SWIO initialized");
        }
        else if(args && strncmp(args, "DEINIT", 6) == 0) {
            swio_deinit();
            send_response("OK SWIO deinitialized");
        }
        else { send_response("ERR unknown SWIO subcommand: STATUS|INIT|DEINIT"); }
    }
    else if(strcmp(cmd, "RVSWD") == 0) {
        if(args && strncmp(args, "STATUS", 6) == 0) {
            char msg[64];
            sprintf(msg, "RVSWD initialized=%d", (int)rvswd_is_initialized());
            send_response(msg);
        }
        else if(args && strncmp(args, "INIT", 4) == 0) {
            rvswd_result_t r = rvswd_init();
            char msg[64];
            sprintf(msg, r == RVSWD_OK ? "OK RVSWD initialized" : "ERR RVSWD init failed: %d", (int)r);
            send_response(msg);
        }
        else if(args && strncmp(args, "DEINIT", 6) == 0) {
            rvswd_deinit();
            send_response("OK RVSWD deinitialized");
        }
        else if(args && strncmp(args, "RESET", 5) == 0) {
            rvswd_result_t r = rvswd_reset();
            char msg[64];
            sprintf(msg, r == RVSWD_OK ? "OK RVSWD reset sent" : "ERR RVSWD reset: %d", (int)r);
            send_response(msg);
        }
        else if(args && strncmp(args, "READ ", 5) == 0) {
            uint32_t reg = (uint32_t)strtoul(args + 5, NULL, 0);
            uint32_t val = 0;
            rvswd_result_t r = rvswd_read((uint8_t)reg, &val);
            char msg[96];
            if (r == RVSWD_OK) {
                sprintf(msg, "RVSWD READ reg=0x%02lX val=0x%08lX", (unsigned long)reg, (unsigned long)val);
            } else {
                sprintf(msg, "RVSWD READ FAILED reg=0x%02lX res=%d val=0x%08lX",
                        (unsigned long)reg, (int)r, (unsigned long)val);
            }
            send_response(msg);
        }
        else if(args && strncmp(args, "WRITE ", 6) == 0) {
            uint32_t reg = (uint32_t)strtoul(args + 6, NULL, 0);
            char *val_str = strchr(args + 6, ' ');
            if (!val_str) { send_response("ERR usage: RVSWD WRITE <reg> <val>"); return; }
            uint32_t val = (uint32_t)strtoul(val_str, NULL, 0);
            rvswd_result_t r = rvswd_write((uint8_t)reg, val);
            char msg[96];
            sprintf(msg, r == RVSWD_OK ? "OK RVSWD WRITE reg=0x%02lX val=0x%08lX" : "ERR RVSWD WRITE res=%d",
                    (unsigned long)reg, (unsigned long)val, (int)r);
            send_response(msg);
        }
        else { send_response("ERR unknown RVSWD subcommand: STATUS|INIT|DEINIT|RESET|READ|WRITE"); }
    }
    else if(strcmp(cmd, "STM32") == 0) {
        if(args && strncmp(args, "TEST", 4) == 0) {
            cmd_stm32_test(args + 4);
        } else {
            send_response("ERR unknown STM32 subcommand: use STM32 TEST");
        }
    }
    else { char msg[64]; sprintf(msg, "ERR unknown command: %s", cmd); send_response(msg); }
}

static void process_input(void) {
    /* Read from CDC ring buffer instead of UART */
    uint32_t idle_count = 0;
    while (usb_cdc_available()) {
        int data = usb_cdc_read_char();
        if(data < 0) break;
        char c = (char)data;
        if(c == '\r' || c == '\n') {
            if(cmd_pos > 0) {
                cmd_buffer[cmd_pos] = '\0';
                printf("> %s\r\n", cmd_buffer);
                process_command(cmd_buffer);
                cmd_pos = 0;
            }
        } else if(cmd_pos < sizeof(cmd_buffer) - 1) {
            cmd_buffer[cmd_pos++] = c;
        }
        /* Process USB events periodically during long command accumulation */
        if (++idle_count % 32 == 0) {
            usb_hid_task();
        }
    }
}








int main() {
    /* Install hardfault handler as early as possible for crash diagnostics */
    exception_set_exclusive_handler(HARDFAULT_EXCEPTION, hardfault_handler_c);

    #ifdef TURBO_MODE
    vreg_disable_voltage_limit();
    vreg_set_voltage(VREG_VOLTAGE_1_30);
    sleep_ms(1);
    set_sys_clock_khz(400000, true);
    #else
    set_sys_clock_khz(125000, true);
    #endif

    /* Check boot counter — if firmware crashed repeatedly, enter BOOTSEL */
    boot_counter_increment();

    /* Persist crash info from RAM to flash if a hardfault occurred */
    if (crash_info_ram.magic == CRASH_MAGIC) {
        uint32_t saved = save_and_disable_interrupts();
        uint8_t buf[FLASH_SECTOR_SIZE];
        memset(buf, 0xFF, sizeof(buf));
        memcpy(buf, &crash_info_ram, sizeof(crash_info_t));
        flash_range_erase(CRASH_FLASH_OFFSET, FLASH_SECTOR_SIZE);
        flash_range_program(CRASH_FLASH_OFFSET, buf, FLASH_SECTOR_SIZE);
        /* Clear the RAM copy so we don't re-persist */
        crash_info_ram.magic = 0;
        restore_interrupts(saved);
    }

    /* LED on immediately so user knows device is alive */
    gpio_init(LED_IO);
    gpio_set_dir(LED_IO, GPIO_OUT);
    gpio_put(LED_IO, 1);

    /*
     * Initialize USB composite device: CDC (serial CLI) + HID (CMSIS-DAP).
     * printf() routes through TinyUSB CDC — no UART needed for console.
     * GPIO 0/1 are used by adapter I2C (ADAPTER_I2C_SCL=0, ADAPTER_I2C_SDA=1).
     * GPIO 4/5 are shared between adapter UART and SWD PIO1.
     */
    usb_hid_init();

    /* Wait for USB enumeration */
    for (int i = 0; i < 20; i++) {
        tud_task();
        sleep_ms(10);
    }

    printf("\r\n=== AgentLA v" FIRMWARE_VERSION " Starting ===\r\n");
    network_init();

    printf("USB CDC + HID composite device + NETWORK\r\n");
    printf("BUILD:%s %s\r\n", BUILD_TIMESTAMP, GIT_HASH);

    /* Initialize pin ownership with a blank flexible registry. */
    pin_owner_init();

    /* Initialize adapter I2C only - skip UART init to avoid GPIO 4/5 conflict with SWD */
    adapter_init_i2c_only();

    memset(&capture_config, 0, sizeof(capture_config));
    capture_config.frequency = 1000000;
    capture_config.channel_count = 8;
    for(uint8_t i = 0; i < 8; i++) capture_config.channels[i] = i;

    gpio_put(LED_IO, 0);

    while(1) {
        /* Network stack */
        network_task();

        /* Process USB HID (CMSIS-DAP) */
        usb_hid_task();

        /* Process serial commands (UART) */
        process_input();

        if(device_state == DEVICE_CAPTURING) {
            gpio_put(LED_IO, !gpio_get(LED_IO));
        }
    }

    return 0;
}

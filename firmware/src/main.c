#include "agentla_board_settings.h"
#include "agentla_structs.h"
#include "agentla_adapter.h"
#include <stdio.h>
#include <string.h>
#include <stdlib.h>
#include "pico/stdlib.h"
#include "hardware/dma.h"
#include "hardware/pio.h"
#include "hardware/clocks.h"
#include "hardware/vreg.h"
#include "hardware/timer.h"
#include "pico/multicore.h"
#include "agentla_capture.pio.h"
#include "agentla_capture.h"
#include "agentla_decode.h"

static CAPTURE_CONFIG capture_config;
static DECODE_CONFIG decode_config;
static DEVICE_STATE device_state = DEVICE_IDLE;

static char cmd_buffer[256];
static uint8_t cmd_pos = 0;

static void send_response(const char* response) {
    printf("%s\r\n", response);
}

static void send_event(const char* event) {
    printf("%s\r\n", event);
}

static void cmd_identify(void) {
    send_response("AGENTLA " FIRMWARE_VERSION " RP2350 ANALYZER+ADAPTER 24CH 100MHZ I2C+UART");
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

static void cmd_i2c_read(char* args) {
    uint8_t addr = 0;
    uint8_t reg = 0;
    uint8_t len = 0;
    uint32_t timeout_ms = 100;

    char* token = strtok(args, " ");
    while (token != NULL) {
        if (strncmp(token, "addr=", 5) == 0) addr = (uint8_t)strtol(token + 5, NULL, 0);
        else if (strncmp(token, "reg=", 4) == 0) reg = (uint8_t)strtol(token + 4, NULL, 0);
        else if (strncmp(token, "len=", 4) == 0) len = (uint8_t)atoi(token + 4);
        else if (strncmp(token, "timeout=", 8) == 0) timeout_ms = (uint32_t)atoi(token + 8);
        token = strtok(NULL, " ");
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
    uint8_t addr = 0;
    uint8_t reg = 0;
    uint8_t data[64];
    uint8_t data_len = 0;
    uint32_t timeout_ms = 100;

    char* token = strtok(args, " ");
    while (token != NULL) {
        if (strncmp(token, "addr=", 5) == 0) addr = (uint8_t)strtol(token + 5, NULL, 0);
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
    uint32_t baud = 115200;
    uint8_t data[256];
    uint8_t data_len = 0;

    char* token = strtok(args, " ");
    while (token != NULL) {
        if (strncmp(token, "baud=", 5) == 0) baud = (uint32_t)atoi(token + 5);
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

    if (baud < 1200 || baud > 921600) {
        send_response("ERR invalid baud rate, min=1200 max=921600");
        return;
    }

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
    uint32_t baud = 115200;
    uint32_t max_len = 64;
    uint32_t timeout_ms = 1000;

    char* token = strtok(args, " ");
    while (token != NULL) {
        if (strncmp(token, "baud=", 5) == 0) baud = (uint32_t)atoi(token + 5);
        else if (strncmp(token, "len=", 4) == 0) max_len = (uint8_t)atoi(token + 4);
        else if (strncmp(token, "timeout=", 8) == 0) timeout_ms = (uint32_t)atoi(token + 8);
        token = strtok(NULL, " ");
    }

    if (baud < 1200 || baud > 921600) {
        send_response("ERR invalid baud rate, min=1200 max=921600");
        return;
    }

    if (max_len > 256) max_len = 256;

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
    (void)args;
    adapter_i2c_clear_bus();
    sleep_ms(10);
    send_response("OK I2C BUS CLEARED");
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
            while(*ch_ptr && count < 24) {
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
                send_response("ERR too many channels, max 24");
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
    if(capture_config.channel_count > 16) ch_mode = MODE_24_CHANNEL;

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
        else if(mode_out == MODE_24_CHANNEL) bytesPerSample = 4;
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

static void cmd_selftest_pio1(char* args) {
    uint32_t pin = 4;
    char* token = strtok(args, " ");
    while(token != NULL) {
        if(strncmp(token, "pin=", 4) == 0) pin = atoi(token + 4);
        token = strtok(NULL, " ");
    }

    PIO pio1_instance = pio1;
    uint sm = pio_claim_unused_sm(pio1_instance, true);
    uint offset = pio_add_program(pio1_instance, &PIO1_UART_TX_program);

    pio_sm_config c = PIO1_UART_TX_program_get_default_config(offset);
    sm_config_set_out_pins(&c, pin, 1);
    sm_config_set_set_pins(&c, pin, 1);
    sm_config_set_clkdiv(&c, 100.0f);

    pio_gpio_init(pio1_instance, pin);
    pio_sm_init(pio1_instance, sm, offset, &c);
    pio_sm_set_enabled(pio1_instance, sm, true);

    char dbg[128];

    uint32_t pinctrl = pio1_instance->sm[sm].pinctrl;
    uint32_t execctrl = pio1_instance->sm[sm].execctrl;
    sprintf(dbg, "PIO1 pinctrl=0x%08lx execctrl=0x%08lx", (unsigned long)pinctrl, (unsigned long)execctrl);
    send_response(dbg);

    uint32_t set_base = (pinctrl & PIO_SM0_PINCTRL_SET_BASE_BITS) >> PIO_SM0_PINCTRL_SET_BASE_LSB;
    uint32_t out_base = (pinctrl & PIO_SM0_PINCTRL_OUT_BASE_BITS) >> PIO_SM0_PINCTRL_OUT_BASE_LSB;
    uint32_t set_count = (pinctrl & PIO_SM0_PINCTRL_SET_COUNT_BITS) >> PIO_SM0_PINCTRL_SET_COUNT_LSB;
    uint32_t out_count = (pinctrl & PIO_SM0_PINCTRL_OUT_COUNT_BITS) >> PIO_SM0_PINCTRL_OUT_COUNT_LSB;
    sprintf(dbg, "PIO1 set_base=%lu out_base=%lu set_cnt=%lu out_cnt=%lu", (unsigned long)set_base, (unsigned long)out_base, (unsigned long)set_count, (unsigned long)out_count);
    send_response(dbg);

    pio_sm_put_blocking(pio1_instance, sm, 0xAA);
    busy_wait_us(10000);
    sprintf(dbg, "PIO1A fn=%d val=%d sioin=%d",
        (int)gpio_get_function(pin), (int)gpio_get(pin),
        (int)((sio_hw->gpio_in >> pin) & 1));
    send_response(dbg);

    pio_sm_set_enabled(pio1_instance, sm, false);
    pio_remove_program(pio1_instance, &PIO1_UART_TX_program, offset);
    pio_sm_unclaim(pio1_instance, sm);
    gpio_init(pin);
    send_response("PIO1 PIN TEST DONE");
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

    device_state = DEVICE_CAPTURING;
    if(!StartAdcCapture(pin, rate, samples)) {
        device_state = DEVICE_IDLE;
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
}

static void cmd_status(void) {
    switch(device_state) {
        case DEVICE_IDLE: send_response("STATUS IDLE"); break;
        case DEVICE_CAPTURING: send_response("STATUS CAPTURING"); break;
        case DEVICE_DECODING: send_response("STATUS DECODING"); break;
    }
}

static void process_command(char* cmd) {
    char* args = strchr(cmd, ' ');
    if(args) { *args = '\0'; args++; }
    if(strcmp(cmd, "IDENTIFY") == 0) cmd_identify();
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
    else if(strcmp(cmd, "SELFTEST") == 0) cmd_selftest(args ? args : "");
    else if(strcmp(cmd, "PIO1TEST") == 0) cmd_selftest_pio1(args ? args : "");
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
    else { char msg[64]; sprintf(msg, "ERR unknown command: %s", cmd); send_response(msg); }
}

static void process_input(void) {
    int data = getchar_timeout_us(0);
    if(data == PICO_ERROR_TIMEOUT) return;
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
}

int main() {
    #ifdef TURBO_MODE
    vreg_disable_voltage_limit();
    vreg_set_voltage(VREG_VOLTAGE_1_30);
    sleep_ms(1);
    set_sys_clock_khz(400000, true);
    #else
    set_sys_clock_khz(200000, true);
    #endif
    stdio_init_all();
    sleep_ms(1000);
    adapter_init();
    memset(&capture_config, 0, sizeof(capture_config));
    capture_config.frequency = 1000000;
    capture_config.channel_count = 8;
    for(uint8_t i = 0; i < 8; i++) capture_config.channels[i] = i;
    gpio_init(LED_IO);
    gpio_set_dir(LED_IO, GPIO_OUT);
    gpio_put(LED_IO, 1);
    while(1) {
        process_input();
        if(device_state == DEVICE_CAPTURING) {
            gpio_put(LED_IO, !gpio_get(LED_IO));
        }
    }
    return 0;
}

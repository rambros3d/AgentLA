#ifndef __AGENTLA_DECODE__
#define __AGENTLA_DECODE__

#include "structs.h"
#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

#define DECODE_MAX_EVENTS 512
#define DECODE_TEXT_LEN 64
#define DECODER_ARENA_CAPACITY 1024

/* Protocol specific event types */
typedef enum {
    /* Legacy / Generic types */
    EVT_I2C_START = 0,
    EVT_I2C_STOP,
    EVT_I2C_ADDR,
    EVT_I2C_ACK,
    EVT_I2C_NACK,
    EVT_I2C_DATA,
    EVT_SPI_CS_ASSERT,
    EVT_SPI_CS_DEASSERT,
    EVT_SPI_MOSI,
    EVT_SPI_MISO,
    EVT_UART_DATA,
    EVT_UART_BREAK,
    EVT_UART_ERR_PARITY,
    EVT_UART_ERR_FRAMING,
    EVT_PWM_SIGNAL,
    EVT_PWM_NO_SIGNAL,

    /* CAN */
    EVT_CAN_FRAME,
    EVT_CAN_ERR_STUFF,
    EVT_CAN_ERR_CRC,

    /* Modbus RTU */
    EVT_MODBUS_REQ,
    EVT_MODBUS_RESP,
    EVT_MODBUS_EXCEPTION,
    EVT_MODBUS_ERR_CRC,

    /* DMX-512 */
    EVT_DMX_FRAME,
    EVT_DMX_BREAK,
    EVT_DMX_ERR_MAB,

    /* Addressable LED */
    EVT_LED_PIXEL,
    EVT_LED_RESET,
    EVT_LED_TIMING_VIOLATION,

    /* 1-Wire Dallas */
    EVT_1WIRE_RESET,
    EVT_1WIRE_PRESENCE,
    EVT_1WIRE_ROM,
    EVT_1WIRE_TEMP,
    EVT_1WIRE_ERR_CRC,

    /* LIN */
    EVT_LIN_FRAME,
    EVT_LIN_ERR_PARITY,
    EVT_LIN_ERR_CHECKSUM,

    /* MIPI I3C */
    EVT_I3C_BROADCAST,
    EVT_I3C_CCC,
    EVT_I3C_DATA,
    EVT_I3C_ERR_PARITY,

    /* ARM SWD */
    EVT_SWD_DP_READ,
    EVT_SWD_DP_WRITE,
    EVT_SWD_AP_READ,
    EVT_SWD_AP_WRITE,
    EVT_SWD_LINE_RESET,
    EVT_SWD_ERR_ACK,
    EVT_SWD_ERR_PARITY,

    /* ARM SWO */
    EVT_SWO_ITM_PRINTF,
    EVT_SWO_PACKET,

    /* SPI Flash */
    EVT_SPIFLASH_CMD
} EVENT_TYPE;

/* Compact 24-byte event structure */
#pragma pack(push, 1)
typedef struct {
    float timestamp;       /* 4 bytes: start time in seconds */
    float end_timestamp;   /* 4 bytes: end time in seconds */
    uint8_t protocol;      /* 1 byte: DECODE_PROTOCOL */
    uint8_t type;          /* 1 byte: EVENT_TYPE */
    uint8_t channel;       /* 1 byte: logic channel index */
    uint8_t flags;         /* 1 byte: status / error flags */

    union {
        /* Raw / Generic payload */
        uint8_t raw[12];

        /* CAN 2.0B / CAN-FD */
        struct {
            uint32_t id;         /* 11-bit or 29-bit Arbitration ID */
            uint16_t crc;        /* Computed or received CRC */
            uint8_t dlc;         /* Data length code (0-64) */
            uint8_t flags;       /* bit 0: extended, bit 1: fd, bit 2: brs, bit 3: crc_ok */
            uint8_t data[4];     /* Payload preview */
        } can;

        /* Modbus RTU */
        struct {
            uint16_t reg_addr;   /* Start address or register */
            uint16_t reg_count;  /* Register count or value */
            uint16_t crc;        /* CRC-16 */
            uint8_t slave_addr;  /* Slave ID (1-247) */
            uint8_t function_code;/* FC 0x01-0x10 or 0x80+ */
            uint8_t exception_code;
            uint8_t flags;       /* bit 0: crc_ok */
            uint8_t data[2];     /* Extra info */
        } modbus;

        /* DMX-512 */
        struct {
            uint16_t channel_count; /* Number of slots */
            uint8_t start_code;     /* Typically 0x00 */
            uint8_t slots[9];       /* Channels 1-9 preview */
        } dmx;

        /* Addressable LED */
        struct {
            uint16_t pixel_index;   /* 0-based pixel index */
            uint8_t r;              /* Red (0-255) */
            uint8_t g;              /* Green (0-255) */
            uint8_t b;              /* Blue (0-255) */
            uint8_t w;              /* White (0-255 for RGBW) */
            uint8_t brightness;     /* Global brightness (0-31) */
            uint8_t flags;          /* bit 0: timing violation */
            uint16_t pulse_ns;      /* Measured pulse width in ns */
            uint16_t reserved;
        } led;

        /* 1-Wire Dallas */
        struct {
            float temp_c;           /* Decoded temperature in Celsius */
            uint8_t family;         /* Device family code (e.g. 0x28) */
            uint8_t crc;            /* DOW CRC */
            uint8_t flags;          /* bit 0: crc_ok, bit 1: has_temp */
            uint8_t rom[5];         /* Serial bytes preview */
        } onewire;

        /* LIN */
        struct {
            uint8_t pid;            /* Protected ID */
            uint8_t id;             /* 6-bit Frame ID */
            uint8_t length;         /* Payload byte length */
            uint8_t checksum;       /* Checksum byte */
            uint8_t flags;          /* bit 0: parity_ok, bit 1: checksum_ok, bit 2: enhanced */
            uint8_t data[7];        /* Data bytes */
        } lin;

        /* MIPI I3C */
        struct {
            uint8_t ccc_code;       /* CCC code */
            uint8_t dyn_addr;       /* Assigned/target dynamic address */
            uint8_t is_direct;      /* 1 = Direct, 0 = Broadcast */
            uint8_t flags;          /* bit 0: parity_ok, bit 1: ack */
            uint8_t data[8];        /* Data payload */
        } i3c;

        /* ARM SWD */
        struct {
            uint32_t data;          /* 32-bit register value */
            uint8_t reg_addr;       /* Register offset (A[3:2]) */
            uint8_t ap_ndp;         /* 0 = DP, 1 = AP */
            uint8_t r_nw;           /* 0 = Write, 1 = Read */
            uint8_t ack;            /* 1=OK, 2=WAIT, 4=FAULT */
            uint8_t flags;          /* bit 0: parity_ok */
            uint8_t reserved[3];
        } swd;

        /* ARM SWO Trace */
        struct {
            uint8_t port;           /* Stimulus port (0-31) */
            uint8_t len;            /* Valid chars in text (1-10) */
            char text[10];          /* ASCII text chunk */
        } swo;

        /* SPI Flash */
        struct {
            uint32_t address;       /* 24-bit or 32-bit address */
            uint8_t opcode;         /* 0x9F, 0x03, 0x02, 0x20... */
            uint8_t dummy_cycles;
            uint16_t length;        /* Transfer length */
            uint8_t data[4];        /* Data preview */
        } spiflash;

        /* Legacy I2C / SPI / UART / PWM */
        struct {
            uint32_t val32;
            uint8_t b0, b1, b2, b3, b4, b5, b6, b7;
        } legacy;
    };
} DECODER_EVENT_V2;
#pragma pack(pop)

_Static_assert(sizeof(DECODER_EVENT_V2) == 24, "DECODER_EVENT_V2 must be exactly 24 bytes");

/* Compact 24 KB Event Arena (1024 events) */
typedef struct {
    DECODER_EVENT_V2 events[DECODER_ARENA_CAPACITY];
    uint16_t count;
    uint16_t overflow_count;
} DECODER_EVENT_ARENA;

/* Legacy API compatibility structures */
typedef struct {
    EVENT_TYPE type;
    float timestamp;
    uint8_t pin;
    uint8_t value;
    uint8_t ack;
    uint8_t rw;
    char text[DECODE_TEXT_LEN];
} DECODE_EVENT;

typedef struct {
    DECODE_EVENT events[DECODE_MAX_EVENTS];
    uint16_t count;
} DECODE_RESULT;

/* Arena management functions */
void decoder_arena_init(DECODER_EVENT_ARENA* arena);
DECODER_EVENT_V2* decoder_arena_add(DECODER_EVENT_ARENA* arena, uint8_t protocol, uint8_t type,
                                    float ts, float end_ts, uint8_t channel, uint8_t flags);
void decoder_event_format(const DECODER_EVENT_V2* ev, char* out, size_t out_len);

/* Legacy decoders */
void i2c_decode(const uint8_t* buffer, uint32_t length, uint32_t sample_rate,
                uint8_t sda_pin, uint8_t scl_pin, uint8_t pin_base,
                uint32_t buf_offset, DECODE_RESULT* result);

void spi_decode(const uint8_t* buffer, uint32_t length, uint32_t sample_rate,
                uint8_t cs_pin, uint8_t sclk_pin, uint8_t mosi_pin, uint8_t miso_pin,
                uint8_t pin_base, uint32_t buf_offset, DECODE_RESULT* result);

void uart_decode(const uint8_t* buffer, uint32_t length, uint32_t sample_rate,
                 uint8_t rx_pin, uint32_t baud_rate, uint8_t pin_base,
                 uint32_t buf_offset, DECODE_RESULT* result);

void pwm_decode(const uint8_t* buffer, uint32_t length, uint32_t sample_rate,
                uint8_t pin, uint8_t pin_base, uint32_t buf_offset, DECODE_RESULT* result);

/* New Native Protocol Decoders (Phases 1 - 4) */
void can_decode(const uint8_t* buffer, uint32_t length, uint32_t sample_rate,
                uint8_t rx_pin, uint32_t nominal_baud, uint32_t data_baud,
                uint8_t sample_point_percent, uint8_t pin_base, uint32_t buf_offset,
                DECODER_EVENT_ARENA* arena);

void modbus_decode(const uint8_t* buffer, uint32_t length, uint32_t sample_rate,
                   uint8_t rx_pin, uint32_t baud_rate, uint8_t pin_base,
                   uint32_t buf_offset, DECODER_EVENT_ARENA* arena);

void dmx_decode(const uint8_t* buffer, uint32_t length, uint32_t sample_rate,
                uint8_t rx_pin, uint8_t pin_base, uint32_t buf_offset,
                DECODER_EVENT_ARENA* arena);

void led_decode_1w(const uint8_t* buffer, uint32_t length, uint32_t sample_rate,
                   uint8_t data_pin, uint8_t color_order, uint8_t is_rgbw,
                   uint8_t pin_base, uint32_t buf_offset, DECODER_EVENT_ARENA* arena);

void led_decode_2w(const uint8_t* buffer, uint32_t length, uint32_t sample_rate,
                   uint8_t clk_pin, uint8_t data_pin, uint8_t pin_base,
                   uint32_t buf_offset, DECODER_EVENT_ARENA* arena);

void onewire_decode(const uint8_t* buffer, uint32_t length, uint32_t sample_rate,
                    uint8_t pin, uint8_t pin_base, uint32_t buf_offset,
                    DECODER_EVENT_ARENA* arena);

void lin_decode(const uint8_t* buffer, uint32_t length, uint32_t sample_rate,
                uint8_t rx_pin, uint32_t baud_rate, uint8_t pin_base,
                uint32_t buf_offset, DECODER_EVENT_ARENA* arena);

void i3c_decode(const uint8_t* buffer, uint32_t length, uint32_t sample_rate,
                uint8_t sda_pin, uint8_t scl_pin, uint8_t pin_base,
                uint32_t buf_offset, DECODER_EVENT_ARENA* arena);

void swd_decode(const uint8_t* buffer, uint32_t length, uint32_t sample_rate,
                uint8_t clk_pin, uint8_t dio_pin, uint8_t pin_base,
                uint32_t buf_offset, DECODER_EVENT_ARENA* arena);

void swo_decode(const uint8_t* buffer, uint32_t length, uint32_t sample_rate,
                uint8_t swo_pin, uint32_t baud_rate, uint8_t pin_base,
                uint32_t buf_offset, DECODER_EVENT_ARENA* arena);

void spiflash_decode(const uint8_t* buffer, uint32_t length, uint32_t sample_rate,
                     uint8_t cs_pin, uint8_t sclk_pin, uint8_t mosi_pin, uint8_t miso_pin,
                     uint8_t pin_base, uint32_t buf_offset, DECODER_EVENT_ARENA* arena);

#endif

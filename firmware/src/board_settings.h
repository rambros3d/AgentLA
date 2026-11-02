#ifndef __AGENTLA_BOARD_SETTINGS__
#define __AGENTLA_BOARD_SETTINGS__

#include "pico/stdlib.h"

#if defined(BAKUPROBE_BOARD) || defined(SILINGPROBE_BOARD) || defined(AGENTPROBE_BOARD)
    #include "boards/board_bakuprobe.h"
#else
    #include "boards/board_pico2.h"
#endif

#define COMPLEX_TRIGGER_OUT_PIN 0
#define COMPLEX_TRIGGER_IN_PIN 1

/* Adapter mode pins — dedicated, no sharing with programmer */
#ifndef ADAPTER_I2C_SCL_PIN
#define ADAPTER_I2C_SCL_PIN 10  /* I2C1 SCL (F2) */
#endif
#ifndef ADAPTER_I2C_SDA_PIN
#define ADAPTER_I2C_SDA_PIN 11  /* I2C1 SDA (F2) */
#endif
#ifndef ADAPTER_I3C_BASE_PIN
#define ADAPTER_I3C_BASE_PIN 10
#endif
#ifndef ADAPTER_I3C_SDA_PIN
#define ADAPTER_I3C_SDA_PIN  10  /* I3C SDA on PIO2 */
#endif
#ifndef ADAPTER_I3C_SCL_PIN
#define ADAPTER_I3C_SCL_PIN  11  /* I3C SCL on PIO2 */
#endif
#ifndef ADAPTER_UART_TX_PIN
#define ADAPTER_UART_TX_PIN 12  /* UART TX */
#endif
#ifndef ADAPTER_UART_RX_PIN
#define ADAPTER_UART_RX_PIN 13  /* UART RX */
#endif

/* Dedicated Transceiver Pin Blocks */
#ifndef CAN_TX_PIN
#define CAN_TX_PIN   18
#endif
#ifndef CAN_RX_PIN
#define CAN_RX_PIN   19
#endif
#ifndef CAN_TERM_PIN
#define CAN_TERM_PIN 22
#endif
#ifndef RS485_TX_PIN
#define RS485_TX_PIN 12
#endif
#ifndef RS485_RX_PIN
#define RS485_RX_PIN 13
#endif
#ifndef RS485_DE_PIN
#define RS485_DE_PIN 14
#endif

/* I3C PIO Assignment on RP2350 */
#if NUM_PIOS >= 3
#define I3C_PIO_INSTANCE    pio2
#else
#define I3C_PIO_INSTANCE    pio0
#endif
#define I3C_PIO_SM          0

#define GPIO_LED
#define LED_IO 25  /* Pico 2 LED is on GPIO25, shared with USB D- */
                    /* USB still works because Pico 2 has a series resistor */
                    /* that allows LED to be driven without killing USB */

/* Test wiring: RP2350 GPIO6-9 ↔ ESP32 GPIO19-16 */
#define TEST_PIN_0 6
#define TEST_PIN_1 7
#define TEST_PIN_2 8
#define TEST_PIN_3 9

/* ═══════════════════════════════════════════════════════════════════
 * PROGRAMMER PINS (GPIO 0-9)
 * Flash and debug target MCUs via UART bootloader + SWD
 * ═══════════════════════════════════════════════════════════════════ */
#define PROG_I2C_SDA_PIN    0   /* I2C0 SDA (F2) */
#define PROG_I2C_SCL_PIN    1   /* I2C0 SCL (F2) */
#define PROG_SWDCLK_PIN     4   /* PIO0 (F7) — target SWCLK */
#define PROG_SWDIO_PIN      5   /* PIO0 (F7) — target SWDIO */
#define PROG_NRESET_PIN     6   /* GPIO (SIO) — target nRST (open-drain) */
/* GPIO 7: NOT USED — target must be powered externally (RP2350 GPIO7 max ~12mA insufficient) */
#define PROG_UART_TX_PIN    8   /* UART1 TX (F1) — target RX */
#define PROG_UART_RX_PIN    9   /* UART1 RX (F1) — target TX */

/* SWD PIO and DMA Assignments */
#define SWD_PIO_INSTANCE    pio0
#define SWD_PIO_SM          0
#define SWD_DMA_CHANNEL     0

#ifdef TURBO_MODE
    #define MAX_FREQ 200000000
    #define MAX_BLAST_FREQ 400000000
#else
    #define MAX_FREQ 100000000
    #define MAX_BLAST_FREQ 200000000
#endif

#define CAPTURE_BUFFER_SIZE (64 * 3 * 1024)
#ifndef MAX_CHANNELS
#define MAX_CHANNELS 10
#endif

#define FIRMWARE_VERSION "v1.0"
#define USB_VID 0x1209
#define USB_PID 0x414C

/* ═══════════════════════════════════════════════════════════════════
 * PROGRAMMER MODE STATE
 * ═══════════════════════════════════════════════════════════════════ */
extern volatile bool programmer_mode_active;
#define PROG_MODE_IS_ACTIVE() (programmer_mode_active)

/* ═══════════════════════════════════════════════════════════════════
 * PROGRAMMER CONSTANTS
 * ═══════════════════════════════════════════════════════════════════ */
#define PROG_BAUD_RATE       115200
#define PROG_POWER_DELAY_MS  100  /* Deprecated: target powered externally */
#define PROG_NRESET_PULSE_MS 100
#define PROG_UART_TIMEOUT_MS 1000

#endif

#ifndef __AGENTLA_BOARD_SETTINGS__
#define __AGENTLA_BOARD_SETTINGS__

#include "pico/stdlib.h"

#define BOARD_NAME "RP2350"
#define INPUT_PIN_BASE 2
#define COMPLEX_TRIGGER_OUT_PIN 0
#define COMPLEX_TRIGGER_IN_PIN 1
#define GPIO_LED
#define LED_IO 25  /* Pico 2 LED is on GPIO25, shared with USB D- */
                    /* USB still works because Pico 2 has a series resistor */
                    /* that allows LED to be driven without killing USB */

/* Test wiring: RP2350 GPIO6-9 ↔ ESP32 GPIO19-16 */
#define TEST_PIN_0 6
#define TEST_PIN_1 7
#define TEST_PIN_2 8
#define TEST_PIN_3 9

#ifdef TURBO_MODE
    #define MAX_FREQ 200000000
    #define MAX_BLAST_FREQ 400000000
#else
    #define MAX_FREQ 100000000
    #define MAX_BLAST_FREQ 200000000
#endif

#define CAPTURE_BUFFER_SIZE (128 * 3 * 1024)
#define MAX_CHANNELS 24

#define FIRMWARE_VERSION "v1.0"
#define USB_VID 0x1209
#define USB_PID 0x414C

#endif

/**
 * @file board_pico2.h
 * @brief Hardware Board Definition for Raspberry Pi Pico 2 (RP2350A)
 */
#ifndef __AGENTLA_BOARD_PICO2_H__
#define __AGENTLA_BOARD_PICO2_H__

#define BOARD_ID   "pico2"
#define BOARD_NAME "Pico 2"
#define BOARD_MCU  "RP2350A (QFN-60)"
#define NUM_GPIOS  30

/* Programmer Pins (GPIO 0-9) */
#define PROG_I2C_SDA_PIN    0
#define PROG_I2C_SCL_PIN    1
#define PROG_BOOT0_PIN      2
#define PROG_SWDCLK_PIN     4
#define PROG_SWDIO_PIN      5
#define PROG_NRESET_PIN     6
#define PROG_SDI_PIN        7
#define PROG_UART_TX_PIN    8
#define PROG_UART_RX_PIN    9

/* Logic Analyzer Default Base */
#define INPUT_PIN_BASE      20
#define MAX_CHANNELS        10

/* Default Adapter Pins */
#define ADAPTER_I2C_SDA_PIN 10
#define ADAPTER_I2C_SCL_PIN 11
#define ADAPTER_UART_TX_PIN 12
#define ADAPTER_UART_RX_PIN 13

/* Hardware Features */
#define HAS_ONBOARD_LED     1
#define LED_IO              25

#define HAS_HW_CAN          0
#define HAS_HW_RS485        0

/* ADC Analog Scope Pins */
#define ADC_CH0_PIN         26
#define ADC_CH1_PIN         27
#define ADC_CH2_PIN         28
#define ADC_CH3_PIN         29
#define AGENTLA_ADC_CHANNELS 4

#endif /* __AGENTLA_BOARD_PICO2_H__ */

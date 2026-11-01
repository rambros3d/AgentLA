/**
 * @file board_agentprobe.h
 * @brief Hardware Board Definition for AgentProbe Custom PCB (RP2350B QFN-80)
 */
#ifndef __AGENTLA_BOARD_AGENTPROBE_H__
#define __AGENTLA_BOARD_AGENTPROBE_H__

#define BOARD_ID   "agentprobe"
#define BOARD_NAME "AgentProbe"
#define BOARD_MCU  "RP2350B (QFN-80)"
#define NUM_GPIOS  48

/* Zone 1: Target Programmer Header (GPIO 0-9) */
#define PROG_TRST_PIN       0
#define PROG_TDO_PIN        1
#define PROG_BOOT0_PIN      2
#define PROG_TDI_PIN        3
#define PROG_SWDCLK_PIN     4
#define PROG_SWDIO_PIN      5
#define PROG_NRESET_PIN     6
#define PROG_SDI_PIN        7
#define PROG_UART_TX_PIN    8
#define PROG_UART_RX_PIN    9

/* Zone 2: Industrial Differential Transceivers (GPIO 10-15) */
#define HAS_HW_CAN          1
#define CAN_TX_PIN          10
#define CAN_RX_PIN          11
#define CAN_TERM_PIN        12  /* Switchable 120-ohm termination relay */

#define HAS_HW_RS485        1
#define RS485_TX_PIN        13
#define RS485_RX_PIN        14
#define RS485_DE_PIN        15  /* Driver / Receiver auto direction enable */

/* Zone 3: Dedicated Bus Master Pod (GPIO 16-23) */
#define ADAPTER_I2C_SDA_PIN 16
#define ADAPTER_I2C_SCL_PIN 17
#define ADAPTER_I3C_SDA_PIN 16
#define ADAPTER_I3C_SCL_PIN 17

#define ADAPTER_SPI_SCK_PIN 18
#define ADAPTER_SPI_MOSI_PIN 19
#define ADAPTER_SPI_MISO_PIN 20
#define ADAPTER_SPI_CS_PIN  21

#define ADAPTER_UART_TX_PIN 22
#define ADAPTER_UART_RX_PIN 23

/* Zone 4: 16-Channel 100 MS/s Logic Analyzer & Pattern Generator (GPIO 24-39) */
#define INPUT_PIN_BASE      24
#define MAX_CHANNELS        16
#define LA_PIN_START        24
#define LA_PIN_END          39

/* Zone 5: 8-Channel Analog Oscilloscope (GPIO 40-47) */
#define AGENTLA_ADC_CHANNELS 8
#define ADC_SCOPE_CH1_PIN   40  /* ADC0: BNC / SMA Input 1 */
#define ADC_SCOPE_CH2_PIN   41  /* ADC1: BNC / SMA Input 2 */
#define ADC_SCOPE_CH3_PIN   42  /* ADC2: SMA Input 3 */
#define ADC_SCOPE_CH4_PIN   43  /* ADC3: SMA Input 4 */
#define ADC_AUX_CH5_PIN     44  /* ADC4: Auxiliary Pin */
#define ADC_AUX_CH6_PIN     45  /* ADC5: Auxiliary Pin */
#define ADC_CURRENT_PIN     46  /* ADC6: High-Side Target Current Sense */
#define ADC_VTARGET_PIN     47  /* ADC7: Target Voltage Sense (1.2V-5.0V) */

#define HAS_ONBOARD_LED     0
#define VREF_VOLTAGE        3.0f

#endif /* __AGENTLA_BOARD_AGENTPROBE_H__ */

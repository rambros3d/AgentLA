/**
 * @file tusb_config.h
 * @brief TinyUSB configuration for AgentLA CMSIS-DAP + CDC composite device
 */

#ifndef _TUSB_CONFIG_H_
#define _TUSB_CONFIG_H_

#ifdef __cplusplus
 extern "C" {
#endif

/* Common Configuration */
#ifndef CFG_TUSB_MCU
  #error CFG_TUSB_MCU must be defined
#endif

#define LIB_TINYUSB_DEVICE         1
#define CFG_TUSB_RHPORT0_MODE     OPT_MODE_DEVICE

#ifndef CFG_TUSB_OS
#define CFG_TUSB_OS                 OPT_OS_NONE
#endif

#ifndef CFG_TUSB_MEM_SECTION
#define CFG_TUSB_MEM_SECTION
#endif

#ifndef CFG_TUSB_MEM_ALIGN
#define CFG_TUSB_MEM_ALIGN          __attribute__ ((aligned(4)))
#endif

/* Device Configuration */
#ifndef CFG_TUD_ENDPOINT0_SIZE
#define CFG_TUD_ENDPOINT0_SIZE    64
#endif

/* Class Configuration */
#define CFG_TUD_HID             1
#define CFG_TUD_CDC             2
#define CFG_TUD_MSC             0
#define CFG_TUD_MIDI            0
#define CFG_TUD_VENDOR          1

/* Network (RNDIS/ECM) Configuration */
#define CFG_TUD_NET              1
#define CFG_TUD_RNDIS            0
#define CFG_TUD_ECM_RNDIS        1
#ifndef CFG_TUD_NET_MTU
#define CFG_TUD_NET_MTU          1500
#endif

/* CDC Buffer Sizes */
#define CFG_TUD_CDC_RX_BUFSIZE  1024
#define CFG_TUD_CDC_TX_BUFSIZE  1024

/* Vendor Endpoint Buffer Size */
#define CFG_TUD_VENDOR_RX_BUFSIZE  64
#define CFG_TUD_VENDOR_TX_BUFSIZE  64

#ifdef __cplusplus
 }
#endif

#endif /* _TUSB_CONFIG_H_ */

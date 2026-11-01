/**
 * @file cmsis_dap.c
 * @brief CMSIS-DAP v1 protocol implementation
 */
#include "cmsis_dap.h"
#include "swd.h"
#include "target.h"
#include "board_settings.h"
#include "pico/stdlib.h"
#include <string.h>

/* Firmware info */
#define FW_VERSION_MAJOR    1
#define FW_VERSION_MINOR    0
#define FW_VERSION_PATCH    0
#define VENDOR_NAME         "AgentLA"
#define PRODUCT_NAME        "Debug Probe"
#define SERIAL_NUMBER       "00000001"

/* Transfer state */
static uint8_t transfer_idle_cycles = 0;
static uint16_t transfer_wait_retry = 100;
static uint32_t transfer_match_value = 0;
static uint32_t transfer_match_mask = 0;

/* Helper: write 16-bit LE */
static void put_le16(uint8_t *buf, uint16_t val) {
    buf[0] = val & 0xFF;
    buf[1] = (val >> 8) & 0xFF;
}

/* Helper: write 32-bit LE */
static void put_le32(uint8_t *buf, uint32_t val) {
    buf[0] = val & 0xFF;
    buf[1] = (val >> 8) & 0xFF;
    buf[2] = (val >> 16) & 0xFF;
    buf[3] = (val >> 24) & 0xFF;
}

/* DAP_Info command */
static uint16_t dap_info(const uint8_t *req, uint8_t *response) {
    response[0] = DAP_INFO_ID;
    uint8_t id = req[1];
    
    switch (id) {
        case 0x01:  /* DAP_INFO_ID_VENDOR */
            response[1] = strlen(VENDOR_NAME);
            memcpy(&response[2], VENDOR_NAME, strlen(VENDOR_NAME));
            return 2 + strlen(VENDOR_NAME);
            
        case 0x02:  /* DAP_INFO_ID_PRODUCT */
            response[1] = strlen(PRODUCT_NAME);
            memcpy(&response[2], PRODUCT_NAME, strlen(PRODUCT_NAME));
            return 2 + strlen(PRODUCT_NAME);
            
        case 0x03:  /* DAP_INFO_ID_SER_NUM */
            response[1] = strlen(SERIAL_NUMBER);
            memcpy(&response[2], SERIAL_NUMBER, strlen(SERIAL_NUMBER));
            return 2 + strlen(SERIAL_NUMBER);

        case 0x04: {  /* DAP_INFO_ID_FW_VER */
            const char *ver = "1.0";
            response[1] = strlen(ver);
            memcpy(&response[2], ver, strlen(ver));
            return 2 + strlen(ver);
        }
            
        case 0x05: {  /* DAP_INFO_ID_HW_VER */
            const char *ver = "1.0";
            response[1] = strlen(ver);
            memcpy(&response[2], ver, strlen(ver));
            return 2 + strlen(ver);
        }
            
        case 0xF0:  /* DAP_INFO_ID_CAPABILITIES */
            response[1] = 1;  /* 1 byte info */
            response[2] = 1;  /* Bit 0: SWD supported (1), Bit 1: JTAG supported (0) */
            return 3;

        case 0xFE:  /* DAP_INFO_ID_MAX_PACKET_COUNT */
            response[1] = 1;  /* 1 byte info */
            response[2] = 1;  /* 1 packet count */
            return 3;
            
        case 0xFF:  /* DAP_INFO_ID_MAX_PACKET_SIZE */
            response[1] = 2;  /* 2 bytes short integer */
            put_le16(&response[2], 64);  /* 64 bytes packet size */
            return 4;
            
        default:
            response[1] = 0;
            return 2;
    }
}

/* DAP_Connect command */
static uint16_t dap_connect(const uint8_t *req, uint8_t *response) {
    response[0] = DAP_CONNECT;
    uint8_t port = req[1];
    
    if (port == 0 || port == DAP_PORT_SWD) {
        /* Auto-detect or SWD */
        swd_result_t result = swd_init();
        if (result == SWD_OK) {
            uint32_t idcode = 0;
            swd_read_idcode(&idcode);
        }
        response[1] = (result == SWD_OK) ? DAP_PORT_SWD : 0;
    } else {
        response[1] = 0;  /* Unsupported port */
    }
    return 2;
}

/* DAP_Disconnect command */
static uint16_t dap_disconnect(const uint8_t *req, uint8_t *response) {
    (void)req;
    response[0] = DAP_DISCONNECT;
    swd_deinit();
    response[1] = 0;
    return 2;
}

/* DAP_TransferConfigure command */
static uint16_t dap_transfer_configure(const uint8_t *req, uint8_t *response) {
    response[0] = DAP_TRANSFER_CONFIGURE;
    transfer_idle_cycles = req[1];
    transfer_wait_retry = req[2] | (req[3] << 8);
    swd_state.idle_cycles = transfer_idle_cycles;
    /* req[4..5] = match_retry (not used) */
    response[1] = 0;
    return 2;
}

/* DAP_Transfer command */
static uint16_t dap_transfer(const uint8_t *req, uint8_t *response, uint16_t req_len) {
    response[0] = DAP_TRANSFER;
    uint8_t dap_index = req[1];
    (void)dap_index;
    uint8_t request_count = req[2];
    response[1] = 0;  /* Response count */
    response[2] = DAP_TRANSFER_OK;  /* Status */
    
    uint16_t resp_idx = 3;
    uint16_t req_idx = 3;
    uint8_t response_count = 0;
    uint8_t post_read = 0;
    
    for (uint8_t i = 0; i < request_count; i++) {
        if (req_idx >= req_len) {
            response[2] = DAP_TRANSFER_ERROR | DAP_TRANSFER_NO_ACK;
            break;
        }
        
        uint8_t req_type = req[req_idx++];
        bool is_ap = (req_type & DAP_TRANSFER_APnDP) != 0;
        bool is_write = (req_type & DAP_TRANSFER_RnW) == 0;
        uint8_t a23 = (req_type >> 2) & 3;
        uint8_t wire_req = swd_build_request(is_ap ? 1 : 0, is_write ? 0 : 1, a23);
        
        uint32_t value = 0;
        if (is_write) {
            if (post_read) {
                /* Flush previous posted AP read to response before write */
                uint8_t rdbuff_wire = swd_build_request(0, 1, 3);
                uint32_t prev_data = 0;
                uint8_t ack_prev = SWD_ACK_OK;
                uint16_t r = transfer_wait_retry;
                do {
                    ack_prev = swd_transfer(rdbuff_wire, &prev_data);
                    if (ack_prev == SWD_ACK_WAIT) sleep_us(10);
                    else break;
                } while (r-- > 0);
                if (ack_prev != SWD_ACK_OK) {
                    if (ack_prev == SWD_ACK_WAIT) response[2] = DAP_TRANSFER_WAIT;
                    else if (ack_prev == SWD_ACK_FAULT) response[2] = DAP_TRANSFER_FAULT;
                    else response[2] = DAP_TRANSFER_ERROR | DAP_TRANSFER_NO_ACK;
                    break;
                }
                put_le32(&response[resp_idx], prev_data);
                resp_idx += 4;
                post_read = 0;
            }

            if (req_idx + 4 > req_len) {
                response[2] = DAP_TRANSFER_ERROR | DAP_TRANSFER_NO_ACK;
                break;
            }
            value = req[req_idx] | (req[req_idx+1] << 8) | 
                    (req[req_idx+2] << 16) | (req[req_idx+3] << 24);
            req_idx += 4;
            
            uint8_t ack = SWD_ACK_OK;
            uint16_t retry = transfer_wait_retry;
            do {
                ack = swd_transfer(wire_req, &value);
                if (ack == SWD_ACK_WAIT) sleep_us(10);
                else break;
            } while (retry-- > 0);
            
            if (ack != SWD_ACK_OK) {
                if (ack == SWD_ACK_WAIT) response[2] = DAP_TRANSFER_WAIT;
                else if (ack == SWD_ACK_FAULT) response[2] = DAP_TRANSFER_FAULT;
                else response[2] = DAP_TRANSFER_ERROR | DAP_TRANSFER_NO_ACK;
                break;
            }
            response_count++;
        } else {
            /* Read register */
            if (post_read) {
                /* Read was posted before: this transfer returns previous data */
                uint32_t prev_data = 0;
                uint8_t ack = SWD_ACK_OK;
                uint16_t retry = transfer_wait_retry;
                do {
                    ack = swd_transfer(wire_req, &prev_data);
                    if (ack == SWD_ACK_WAIT) sleep_us(10);
                    else break;
                } while (retry-- > 0);
                
                if (ack != SWD_ACK_OK) {
                    if (ack == SWD_ACK_WAIT) response[2] = DAP_TRANSFER_WAIT;
                    else if (ack == SWD_ACK_FAULT) response[2] = DAP_TRANSFER_FAULT;
                    else response[2] = DAP_TRANSFER_ERROR | DAP_TRANSFER_NO_ACK;
                    break;
                }
                put_le32(&response[resp_idx], prev_data);
                resp_idx += 4;
            } else if (is_ap) {
                /* First AP read in packet: post it */
                uint32_t dummy = 0;
                uint8_t ack = SWD_ACK_OK;
                uint16_t retry = transfer_wait_retry;
                do {
                    ack = swd_transfer(wire_req, &dummy);
                    if (ack == SWD_ACK_WAIT) sleep_us(10);
                    else break;
                } while (retry-- > 0);
                
                if (ack != SWD_ACK_OK) {
                    if (ack == SWD_ACK_WAIT) response[2] = DAP_TRANSFER_WAIT;
                    else if (ack == SWD_ACK_FAULT) response[2] = DAP_TRANSFER_FAULT;
                    else response[2] = DAP_TRANSFER_ERROR | DAP_TRANSFER_NO_ACK;
                    break;
                }
                post_read = 1;
            } else {
                /* DP read: immediate data */
                uint32_t dp_data = 0;
                uint8_t ack = SWD_ACK_OK;
                uint16_t retry = transfer_wait_retry;
                do {
                    ack = swd_transfer(wire_req, &dp_data);
                    if (ack == SWD_ACK_WAIT) sleep_us(10);
                    else break;
                } while (retry-- > 0);
                
                if (ack != SWD_ACK_OK) {
                    if (ack == SWD_ACK_WAIT) response[2] = DAP_TRANSFER_WAIT;
                    else if (ack == SWD_ACK_FAULT) response[2] = DAP_TRANSFER_FAULT;
                    else response[2] = DAP_TRANSFER_ERROR | DAP_TRANSFER_NO_ACK;
                    break;
                }
                put_le32(&response[resp_idx], dp_data);
                resp_idx += 4;
            }
            response_count++;
        }
    }
    
    if (response[2] == DAP_TRANSFER_OK && post_read) {
        /* Flush final posted AP read from DP_RDBUFF */
        uint8_t rdbuff_wire = swd_build_request(0, 1, 3);
        uint32_t final_data = 0;
        uint8_t ack = SWD_ACK_OK;
        uint16_t retry = transfer_wait_retry;
        do {
            ack = swd_transfer(rdbuff_wire, &final_data);
            if (ack == SWD_ACK_WAIT) sleep_us(10);
            else break;
        } while (retry-- > 0);
        
        if (ack == SWD_ACK_OK) {
            put_le32(&response[resp_idx], final_data);
            resp_idx += 4;
        } else {
            if (ack == SWD_ACK_WAIT) response[2] = DAP_TRANSFER_WAIT;
            else if (ack == SWD_ACK_FAULT) response[2] = DAP_TRANSFER_FAULT;
            else response[2] = DAP_TRANSFER_ERROR | DAP_TRANSFER_NO_ACK;
        }
    }
    
    response[1] = response_count;
    return resp_idx;
}

/* DAP_SWJ_Clock command */
static uint16_t dap_swj_clock(const uint8_t *req, uint8_t *response) {
    response[0] = DAP_SWJ_CLOCK;
    
    uint32_t freq_hz = req[1] | (req[2] << 8) | (req[3] << 16) | (req[4] << 24);
    
    /* CMSIS-DAP sends frequency in Hz, swd_set_clock expects kHz */
    uint32_t freq_khz = freq_hz / 1000;
    if (freq_khz == 0) freq_khz = 1;
    
    /* Auto-init SWD if not yet initialized (OpenOCD calls SWJ_Clock before Connect) */
    if (!swd_is_initialized()) {
        swd_init();
    }
    
    swd_result_t result = swd_set_clock(freq_khz);
    response[1] = (result == SWD_OK) ? 0 : 1;
    return 2;
}

/* DAP_SWJ_Sequence command */
static uint16_t dap_swj_sequence(const uint8_t *req, uint8_t *response) {
    response[0] = DAP_SWJ_SEQUENCE;
    
    uint8_t bit_count = req[1];
    uint8_t byte_count = (bit_count + 7) / 8;
    
    /* Send sequence */
    swd_result_t result = SWD_OK;
    for (uint8_t i = 0; i < byte_count; i++) {
        swd_write_bits(8, req[2 + i]);
    }
    response[1] = (result == SWD_OK) ? 0 : 1;
    return 2;
}

/* DAP_ResetTarget command */
static uint16_t dap_reset_target(const uint8_t *req, uint8_t *response) {
    (void)req;
    response[0] = DAP_RESET_TARGET;
    
    /* Toggle nRST pin */
    swd_line_reset();
    response[1] = 0;
    return 2;
}

/* DAP_SWJ_Pins command */
static uint16_t dap_swj_pins(const uint8_t *req, uint8_t *response) {
    response[0] = DAP_SWJ_PINS;
    
    uint8_t pin_output = req[1];
    uint8_t pin_select = req[2];
    uint32_t wait_us = req[3] | (req[4] << 8) | (req[5] << 16) | (req[6] << 24);
    
    /* TODO: Implement pin control */
    (void)pin_output;
    (void)pin_select;
    (void)wait_us;
    
    response[1] = 0;  /* Current pin state */
    return 2;
}

/* DAP_Delay command */
static uint16_t dap_delay(const uint8_t *req, uint8_t *response) {
    response[0] = DAP_DELAY;
    
    uint32_t delay_us = req[1] | (req[2] << 8);
    busy_wait_us(delay_us);
    
    response[1] = 0;
    return 2;
}

/* DAP_LED command (stub - always ok) */
static uint16_t dap_led(const uint8_t *req, uint8_t *response) {
    response[0] = DAP_LED;
    response[1] = 0;  /* OK */
    return 2;
}

/* DAP_SWD_Configure command */
static uint16_t dap_swd_configure(const uint8_t *req, uint8_t *response) {
    response[0] = DAP_SWD_CONFIGURE;
    /* req[1] bits [1:0] = turnaround clocks (we accept but ignore), bit [2] = data_phase */
    response[1] = 0;  /* OK */
    return 2;
}

/* DAP_WriteAbort command */
static uint16_t dap_write_abort(const uint8_t *req, uint8_t *response, uint16_t req_len) {
    response[0] = DAP_WRITE_ABORT;
    if (req_len >= 6) {
        uint32_t abort_val = req[2] | (req[3] << 8) | (req[4] << 16) | (req[5] << 24);
        swd_write_dp(0x00, abort_val);
    } else {
        swd_write_dp(0x00, 0x1E);
    }
    response[1] = 0;  /* OK */
    return 2;
}

/* DAP_TransferBlock command */
static uint16_t dap_transfer_block(const uint8_t *req, uint8_t *response, uint16_t req_len) {
    response[0] = DAP_TRANSFER_BLOCK;
    uint8_t dap_index = req[1];
    (void)dap_index;
    uint16_t count = req[2] | (req[3] << 8);
    response[1] = 0;
    response[2] = 0;
    response[3] = DAP_TRANSFER_OK;

    if (count == 0) return 4;
    if (req_len < 5) {
        response[3] = DAP_TRANSFER_ERROR | DAP_TRANSFER_NO_ACK;
        return 4;
    }

    uint8_t req_type = req[4];
    bool is_ap = (req_type & DAP_TRANSFER_APnDP) != 0;
    bool is_write = (req_type & DAP_TRANSFER_RnW) == 0;
    uint8_t a23 = (req_type >> 2) & 3;
    uint8_t wire_req = swd_build_request(is_ap ? 1 : 0, is_write ? 0 : 1, a23);

    uint16_t resp_idx = 4;
    uint16_t req_idx = 5;
    uint16_t completed = 0;

    if (!is_write && is_ap) {
        /* Prime the MEM-AP pipeline for block reads */
        uint32_t dummy = 0;
        uint8_t ack = SWD_ACK_OK;
        uint16_t retry = transfer_wait_retry;
        do {
            ack = swd_transfer(wire_req, &dummy);
            if (ack == SWD_ACK_WAIT) sleep_us(10);
            else break;
        } while (retry-- > 0);

        if (ack != SWD_ACK_OK) {
            if (ack == SWD_ACK_WAIT) response[3] = DAP_TRANSFER_WAIT;
            else if (ack == SWD_ACK_FAULT) response[3] = DAP_TRANSFER_FAULT;
            else response[3] = DAP_TRANSFER_ERROR | DAP_TRANSFER_NO_ACK;
            return 4;
        }
    }

    for (uint16_t i = 0; i < count; i++) {
        uint32_t value = 0;
        if (is_write) {
            if (req_idx + 4 > req_len) {
                response[3] = DAP_TRANSFER_ERROR | DAP_TRANSFER_NO_ACK;
                break;
            }
            value = req[req_idx] | (req[req_idx+1] << 8) |
                    (req[req_idx+2] << 16) | (req[req_idx+3] << 24);
            req_idx += 4;
        } else if (is_ap && (i == count - 1)) {
            /* Final transfer of AP read block: read DP_RDBUFF */
            wire_req = swd_build_request(0, 1, 3);
        }

        uint8_t ack = SWD_ACK_OK;
        uint16_t retry = transfer_wait_retry;
        do {
            uint32_t transfer_val = value;
            ack = swd_transfer(wire_req, &transfer_val);
            if (!is_write) {
                value = transfer_val;
            }
            if (ack == SWD_ACK_WAIT) {
                sleep_us(10);
            } else {
                break;
            }
        } while (retry-- > 0);

        if (ack == SWD_ACK_OK) {
            if (!is_write) {
                put_le32(&response[resp_idx], value);
                resp_idx += 4;
            }
            completed++;
        } else {
            if (ack == SWD_ACK_WAIT) response[3] = DAP_TRANSFER_WAIT;
            else if (ack == SWD_ACK_FAULT) response[3] = DAP_TRANSFER_FAULT;
            else response[3] = DAP_TRANSFER_ERROR | DAP_TRANSFER_NO_ACK;
            break;
        }
    }

    response[1] = completed & 0xFF;
    response[2] = (completed >> 8) & 0xFF;
    return resp_idx;
}

/* Main CMSIS-DAP command processor */
uint16_t cmsis_dap_process(const uint8_t *request, uint8_t *response, uint16_t req_len) {
    if (req_len < 1) return 0;
    
    uint8_t cmd = request[0];
    
    switch (cmd) {
        case DAP_INFO_ID:
            return dap_info(request, response);
            
        case DAP_LED:
            return dap_led(request, response);
            
        case DAP_CONNECT:
            return dap_connect(request, response);
            
        case DAP_DISCONNECT:
            return dap_disconnect(request, response);
            
        case DAP_TRANSFER_CONFIGURE:
            return dap_transfer_configure(request, response);
            
        case DAP_TRANSFER:
            return dap_transfer(request, response, req_len);
            
        case DAP_TRANSFER_BLOCK:
            return dap_transfer_block(request, response, req_len);
            
        case DAP_TRANSFER_ABORT:
            response[0] = DAP_TRANSFER_ABORT;
            response[1] = 0;
            return 2;
            
        case DAP_WRITE_ABORT:
            return dap_write_abort(request, response, req_len);
            
        case DAP_DELAY:
            return dap_delay(request, response);
            
        case DAP_RESET_TARGET:
            return dap_reset_target(request, response);
            
        case DAP_SWJ_PINS:
            return dap_swj_pins(request, response);
            
        case DAP_SWJ_CLOCK:
            return dap_swj_clock(request, response);
            
        case DAP_SWJ_SEQUENCE:
            return dap_swj_sequence(request, response);
            
        case DAP_SWD_CONFIGURE:
            return dap_swd_configure(request, response);
            
        default:
            /* Unknown command - return zero length */
            response[0] = cmd;
            return 1;
    }
}

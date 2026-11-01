#ifndef __AGENTLA_ADAPTER_I3C__
#define __AGENTLA_ADAPTER_I3C__

#include "pico/stdlib.h"
#include <stdint.h>
#include <stdbool.h>

typedef enum {
    I3C_STATUS_OK = 0,
    I3C_STATUS_IBI,
    I3C_STATUS_NAK_DURING_ARBHDR,
    I3C_STATUS_NAK_DURING_SDRADDR,
    I3C_STATUS_PARAM_OUTOFRANGE,
    I3C_STATUS_NO_IBI,
    I3C_STATUS_NAK_DDR,
    I3C_STATUS_DDR_EARLY_TERMINATION,
    I3C_STATUS_DDR_INVALID_PREAMBLE,
    I3C_STATUS_DDR_PARITY_WRONG,
    I3C_STATUS_DDR_CRC_WRONG,
    I3C_STATUS_I2C_XFERERROR,
    I3C_STATUS_CLAIM_ERROR,
    I3C_STATUS_NOT_INITIALIZED,
} i3c_status_t;

typedef struct {
    uint8_t pid[6];
    uint8_t bcr;
    uint8_t dcr;
    uint8_t dynamic_addr;
} i3c_device_info_t;

bool adapter_i3c_bind_claim(uint32_t handle);
bool adapter_i3c_release_claim(void);
bool adapter_i3c_is_claimed(void);

i3c_status_t adapter_i3c_init(uint8_t sda_pin, uint8_t scl_pin, uint32_t freq_khz);
void adapter_i3c_deinit(void);
const char *adapter_i3c_get_errorstring(i3c_status_t errcode);

i3c_status_t adapter_i3c_set_clkrate(uint32_t targetfreq_khz);
i3c_status_t adapter_i3c_targetreset(void);

i3c_status_t adapter_i3c_entdaa(uint8_t addr, i3c_device_info_t *dev_info);
i3c_status_t adapter_i3c_rstdaa(void);
i3c_status_t adapter_i3c_checkack(uint8_t addr);

i3c_status_t adapter_i3c_poll(uint8_t *pdat, uint32_t *plen);

i3c_status_t adapter_i3c_sdr_privwrite(uint8_t addr, const uint8_t *pdat, uint32_t bytecount);
i3c_status_t adapter_i3c_sdr_privread(uint8_t addr, uint8_t *pdat, uint32_t *pbytecount);
i3c_status_t adapter_i3c_sdr_privwriteread(uint8_t addr, const uint8_t *pwritedat, uint32_t writebytecount,
                                           uint8_t *preaddat, uint32_t *preadbytecount);

i3c_status_t adapter_i3c_sdr_ccc_broadcast_write(const uint8_t *pdat, uint32_t bytecount);
i3c_status_t adapter_i3c_sdr_ccc_direct_write(const uint8_t *pdat, uint32_t bytecount,
                                              uint8_t addr, const uint8_t *pdirectdat, uint32_t directbytecount);
i3c_status_t adapter_i3c_sdr_ccc_direct_read(const uint8_t *pdat, uint32_t bytecount,
                                             uint8_t addr, uint8_t *pdirectdat, uint32_t *pdirectbytecount);

i3c_status_t adapter_i3c_set_drivestrength(uint8_t drivestrength_mA);

#endif /* __AGENTLA_ADAPTER_I3C__ */

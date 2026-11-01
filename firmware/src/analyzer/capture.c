#include "board_settings.h"
#include "capture.h"
#include "hardware/gpio.h"
#include "hardware/dma.h"
#include "hardware/irq.h"
#include "hardware/clocks.h"
#include "string.h"
#include "hardware/sync.h"
#include "pin_owner.h"
#include "hardware/structs/syscfg.h"
#include "hardware/structs/systick.h"
#include "hardware/structs/bus_ctrl.h"
#include "capture.pio.h"
#include "rle_util.h"
#include "pico/multicore.h"

static PIO capturePIO;
static PIO triggerPIO;
static uint sm_Capture;
static uint captureOffset;
static uint sm_Trigger;
static uint triggerOffset;

static uint32_t dmaPingPong0;
static uint32_t dmaPingPong1;
static uint32_t transferCount;

static uint8_t lastCapturePins[MAX_CHANNELS];
static uint8_t lastCapturePinCount;
static uint32_t lastTriggerCapture;
static uint32_t lastPreSize;
static uint32_t lastPostSize;
static uint32_t lastLoopCount;
static bool lastTriggerInverted;
static uint8_t lastTriggerPin;
static uint32_t lastStartPosition;
static bool lastCaptureComplexFast;
static uint8_t lastCaptureType;
static uint8_t lastTriggerPinBase;
static uint32_t lastTriggerPinCount;
static uint32_t lastTail;
static CHANNEL_MODE lastCaptureMode = MODE_8_CHANNEL;

static bool captureFinished;
static bool captureProcessed;

/* GPIO20-29: 10 contiguous capture channels */
const uint8_t pinMap[] = {20,21,22,23,24,25,26,27,28,29};
#define PIN_MAP_SIZE (sizeof(pinMap) / sizeof(pinMap[0]))

uint8_t gpio_to_pindex(uint8_t gpio) {
    for(uint8_t i = 0; i < PIN_MAP_SIZE; i++) {
        if(pinMap[i] == gpio) return i;
    }
    return 0xFF;
}

uint8_t captureBuffer[SCOPE_TOTAL_SIZE] __attribute__((aligned(4)));

#define CAPTURE_TYPE_SIMPLE 0
#define CAPTURE_TYPE_COMPLEX 1
#define CAPTURE_TYPE_FAST 2
#define CAPTURE_TYPE_BLAST 3
#define CAPTURE_TYPE_NO_TRIGGER 4
#define CAPTURE_TYPE_RLE 10
#define CAPTURE_TYPE_TURBO 11
#define CAPTURE_TYPE_SCOPE 12

#ifdef SUPPORTS_COMPLEX_TRIGGER

#define COMPLEX_TRIGGER_wrap_target 0
#define COMPLEX_TRIGGER_wrap 8

uint16_t COMPLEX_TRIGGER_program_instructions[] = {
    0x80a0, 0x6020, 0xe000, 0xc007,
    0xa0e0, 0x6044, 0x00a4, 0xe001,
    0x0008,
};

struct pio_program COMPLEX_TRIGGER_program = {
    .instructions = COMPLEX_TRIGGER_program_instructions,
    .length = 9,
    .origin = -1,
};

static inline pio_sm_config COMPLEX_TRIGGER_program_get_default_config(uint offset) {
    pio_sm_config c = pio_get_default_sm_config();
    sm_config_set_wrap(&c, offset + COMPLEX_TRIGGER_wrap_target, offset + COMPLEX_TRIGGER_wrap);
    return c;
}

#define FAST_TRIGGER_wrap_target 0
#define FAST_TRIGGER_wrap 31

uint16_t FAST_TRIGGER_program_instructions[32];

struct pio_program FAST_TRIGGER_program = {
    .instructions = FAST_TRIGGER_program_instructions,
    .length = 32,
    .origin = 0,
};

static inline pio_sm_config FAST_TRIGGER_program_get_default_config(uint offset) {
    pio_sm_config c = pio_get_default_sm_config();
    sm_config_set_wrap(&c, offset + FAST_TRIGGER_wrap_target, offset + FAST_TRIGGER_wrap);
    sm_config_set_sideset(&c, 1, false, false);
    return c;
}

uint8_t create_fast_trigger_program(uint8_t pattern, uint8_t length) {
    uint8_t mask = (1 << length) - 1;
    uint8_t first = 255;
    for(uint8_t i = 0; i < 32; i++) {
        if((i & mask) == pattern)
            FAST_TRIGGER_program_instructions[i] = 0x1000 | i;
        else {
            FAST_TRIGGER_program_instructions[i] = 0xA0A0;
            first = i;
        }
    }
    return first;
}
#endif

uint32_t find_capture_tail() {
    int transferCountLocal;
    switch(lastCaptureMode) {
        case MODE_8_CHANNEL: transferCountLocal = CAPTURE_BUFFER_SIZE; break;
        case MODE_16_CHANNEL: transferCountLocal = CAPTURE_BUFFER_SIZE / 2; break;
    }
    busy_wait_ms(5);
    int32_t transferPos = 0xFFFFFFFF;
    if(dma_channel_is_busy(dmaPingPong0))
        transferPos = dma_channel_hw_addr(dmaPingPong0)->transfer_count;
    else if(dma_channel_is_busy(dmaPingPong1))
        transferPos = dma_channel_hw_addr(dmaPingPong1)->transfer_count;
    if(transferPos == 0xFFFFFFFF) return 0xFFFFFFFF;
    return (transferCountLocal - transferPos) - 1;
}

void disable_gpios() {
    #ifdef SUPPORTS_COMPLEX_TRIGGER
    gpio_deinit(COMPLEX_TRIGGER_OUT_PIN);
    gpio_deinit(COMPLEX_TRIGGER_IN_PIN);
    #endif
    for(uint8_t i = 0; i < lastCapturePinCount; i++)
        gpio_deinit(lastCapturePins[i]);
    gpio_set_inover(lastTriggerPin, 0);
}

void __not_in_flash_func(dma_handler)() {
    if(dma_channel_get_irq0_status(dmaPingPong0)) {
        dma_channel_acknowledge_irq0(dmaPingPong0);
        dma_channel_set_write_addr(dmaPingPong0, captureBuffer, false);
    } else {
        dma_channel_acknowledge_irq0(dmaPingPong1);
        dma_channel_set_write_addr(dmaPingPong1, captureBuffer, false);
    }
}

void abort_DMAs() {
    hw_clear_bits(&dma_hw->ch[dmaPingPong0].al1_ctrl, DMA_CH0_CTRL_TRIG_EN_BITS);
    hw_clear_bits(&dma_hw->ch[dmaPingPong1].al1_ctrl, DMA_CH0_CTRL_TRIG_EN_BITS);
    dma_channel_abort(dmaPingPong0);
    dma_channel_abort(dmaPingPong1);
    dma_channel_set_irq0_enabled(dmaPingPong0, false);
    dma_channel_set_irq0_enabled(dmaPingPong1, false);
    irq_set_enabled(DMA_IRQ_0, false);
    irq_remove_handler(DMA_IRQ_0, dma_handler);
    dma_channel_unclaim(dmaPingPong0);
    dma_channel_unclaim(dmaPingPong1);
}

#ifdef SUPPORTS_COMPLEX_TRIGGER
void fast_capture_completed() {
    disable_gpios();
    lastTail = find_capture_tail();
    abort_DMAs();
    pio_interrupt_clear(capturePIO, 0);
    pio_sm_set_enabled(capturePIO, sm_Capture, false);
    pio_sm_unclaim(capturePIO, sm_Capture);
    pio_remove_program(capturePIO, &FAST_CAPTURE_program, captureOffset);
    pio_sm_set_enabled(triggerPIO, sm_Trigger, false);
    pio_sm_set_pins(triggerPIO, sm_Trigger, 0);
    pio_sm_unclaim(triggerPIO, sm_Trigger);
    pio_remove_program(triggerPIO, &FAST_TRIGGER_program, triggerOffset);
    captureFinished = true;
}

void check_fast_interrupt() {
    if(lastCaptureType == CAPTURE_TYPE_FAST && capturePIO->irq & 1)
        fast_capture_completed();
}

void complex_capture_completed() {
    disable_gpios();
    lastTail = find_capture_tail();
    abort_DMAs();
    pio_interrupt_clear(capturePIO, 0);
    irq_set_enabled(PIO1_IRQ_0, false);
    irq_remove_handler(PIO1_IRQ_0, complex_capture_completed);
    pio_sm_set_enabled(capturePIO, sm_Capture, false);
    pio_sm_unclaim(capturePIO, sm_Capture);
    pio_remove_program(capturePIO, &COMPLEX_CAPTURE_program, captureOffset);
    pio_sm_set_enabled(capturePIO, sm_Trigger, false);
    pio_sm_set_pins(capturePIO, sm_Trigger, 0);
    pio_sm_unclaim(capturePIO, sm_Trigger);
    pio_remove_program(capturePIO, &COMPLEX_TRIGGER_program, triggerOffset);
    captureFinished = true;
}
#endif

void blast_capture_completed() {
    dma_channel_acknowledge_irq0(dmaPingPong0);
    hw_clear_bits(&dma_hw->ch[dmaPingPong0].al1_ctrl, DMA_CH0_CTRL_TRIG_EN_BITS);
    dma_channel_set_irq0_enabled(dmaPingPong0, false);
    irq_set_enabled(DMA_IRQ_0, false);
    irq_remove_handler(DMA_IRQ_0, blast_capture_completed);
    dma_channel_unclaim(dmaPingPong0);
    bus_ctrl_hw->priority = 0;
    lastTail = lastPostSize;
    pio_sm_set_enabled(capturePIO, sm_Capture, false);
    pio_sm_unclaim(capturePIO, sm_Capture);
    pio_remove_program(capturePIO, &BLAST_CAPTURE_program, captureOffset);
    captureFinished = true;
}

void simple_capture_completed() {
    disable_gpios();
    lastTail = find_capture_tail();
    abort_DMAs();
    pio_interrupt_clear(capturePIO, 0);
    irq_set_enabled(PIO1_IRQ_0, false);
    irq_remove_handler(PIO1_IRQ_0, simple_capture_completed);
    pio_sm_set_enabled(capturePIO, sm_Capture, false);
    pio_sm_unclaim(capturePIO, sm_Capture);
    if(lastTriggerInverted)
        pio_remove_program(capturePIO, &POSITIVE_CAPTURE_program, captureOffset);
    else
        pio_remove_program(capturePIO, &NEGATIVE_CAPTURE_program, captureOffset);
    captureFinished = true;
}

void no_trigger_capture_completed() {
    disable_gpios();
    lastTail = find_capture_tail();
    abort_DMAs();
    pio_interrupt_clear(capturePIO, 0);
    irq_set_enabled(PIO1_IRQ_0, false);
    irq_remove_handler(PIO1_IRQ_0, no_trigger_capture_completed);
    pio_sm_set_enabled(capturePIO, sm_Capture, false);
    pio_sm_unclaim(capturePIO, sm_Capture);
    pio_remove_program(capturePIO, &NO_TRIGGER_CAPTURE_program, captureOffset);
    captureFinished = true;
}

void configureBlastDMA(CHANNEL_MODE channelMode, uint32_t length) {
    enum dma_channel_transfer_size transferSize;
    dma_channel_config dmaConfig;
    switch(channelMode) {
        case MODE_8_CHANNEL: transferSize = DMA_SIZE_8; break;
        case MODE_16_CHANNEL: transferSize = DMA_SIZE_16; break;
    }
    dmaPingPong0 = dma_claim_unused_channel(true);
    dmaConfig = dma_channel_get_default_config(dmaPingPong0);
    channel_config_set_read_increment(&dmaConfig, false);
    channel_config_set_write_increment(&dmaConfig, true);
    channel_config_set_transfer_data_size(&dmaConfig, transferSize);
    channel_config_set_dreq(&dmaConfig, pio_get_dreq(capturePIO, sm_Capture, false));
    channel_config_set_enable(&dmaConfig, true);
    dma_channel_set_irq0_enabled(dmaPingPong0, true);
    irq_set_exclusive_handler(DMA_IRQ_0, blast_capture_completed);
    irq_set_enabled(DMA_IRQ_0, true);
    irq_set_priority(DMA_IRQ_0, 0);
    bus_ctrl_hw->priority = BUSCTRL_BUS_PRIORITY_DMA_W_BITS | BUSCTRL_BUS_PRIORITY_DMA_R_BITS;
    dma_channel_configure(dmaPingPong0, &dmaConfig, captureBuffer, &capturePIO->rxf[sm_Capture], length, true);
}

void configureCaptureDMAs(CHANNEL_MODE channelMode) {
    enum dma_channel_transfer_size transferSize;
    dma_channel_config dmaPingPong0Config;
    dma_channel_config dmaPingPong1Config;
    switch(channelMode) {
        case MODE_8_CHANNEL: transferSize = DMA_SIZE_8; transferCount = CAPTURE_BUFFER_SIZE; break;
        case MODE_16_CHANNEL: transferSize = DMA_SIZE_16; transferCount = CAPTURE_BUFFER_SIZE / 2; break;
    }
    dmaPingPong0 = dma_claim_unused_channel(true);
    dmaPingPong1 = dma_claim_unused_channel(true);
    dmaPingPong0Config = dma_channel_get_default_config(dmaPingPong0);
    channel_config_set_read_increment(&dmaPingPong0Config, false);
    channel_config_set_write_increment(&dmaPingPong0Config, true);
    channel_config_set_transfer_data_size(&dmaPingPong0Config, transferSize);
    channel_config_set_chain_to(&dmaPingPong0Config, dmaPingPong1);
    channel_config_set_dreq(&dmaPingPong0Config, pio_get_dreq(capturePIO, sm_Capture, false));
    channel_config_set_enable(&dmaPingPong0Config, true);
    dma_channel_set_irq0_enabled(dmaPingPong0, true);
    dmaPingPong1Config = dma_channel_get_default_config(dmaPingPong1);
    channel_config_set_read_increment(&dmaPingPong1Config, false);
    channel_config_set_write_increment(&dmaPingPong1Config, true);
    channel_config_set_transfer_data_size(&dmaPingPong1Config, transferSize);
    channel_config_set_chain_to(&dmaPingPong1Config, dmaPingPong0);
    channel_config_set_dreq(&dmaPingPong1Config, pio_get_dreq(capturePIO, sm_Capture, false));
    channel_config_set_enable(&dmaPingPong1Config, true);
    dma_channel_set_irq0_enabled(dmaPingPong1, true);
    irq_set_exclusive_handler(DMA_IRQ_0, dma_handler);
    irq_set_enabled(DMA_IRQ_0, true);
    irq_set_priority(DMA_IRQ_0, 0);
    dma_channel_configure(dmaPingPong1, &dmaPingPong1Config, captureBuffer, &capturePIO->rxf[sm_Capture], transferCount, false);
    dma_channel_configure(dmaPingPong0, &dmaPingPong0Config, captureBuffer, &capturePIO->rxf[sm_Capture], transferCount, true);
}

static void rle_capture_completed(void);
static void turbo_capture_completed(void);
static void scope_capture_completed(void);

void StopCapture() {
    if(!captureFinished) {
        hw_clear_bits(&dma_hw->ch[dmaPingPong0].al1_ctrl, DMA_CH0_CTRL_TRIG_EN_BITS);
        hw_clear_bits(&dma_hw->ch[dmaPingPong1].al1_ctrl, DMA_CH0_CTRL_TRIG_EN_BITS);
        uint32_t int_status = save_and_disable_interrupts();
        if(lastCaptureType == CAPTURE_TYPE_SIMPLE) simple_capture_completed();
        else if(lastCaptureType == CAPTURE_TYPE_NO_TRIGGER) no_trigger_capture_completed();
        else if(lastCaptureType == CAPTURE_TYPE_BLAST) blast_capture_completed();
        #ifdef SUPPORTS_COMPLEX_TRIGGER
        else if(lastCaptureType == CAPTURE_TYPE_COMPLEX) complex_capture_completed();
        else if(lastCaptureType == CAPTURE_TYPE_FAST) fast_capture_completed();
        #endif
        else if(lastCaptureType == CAPTURE_TYPE_RLE) rle_capture_completed();
        else if(lastCaptureType == CAPTURE_TYPE_TURBO) turbo_capture_completed();
        else if(lastCaptureType == CAPTURE_TYPE_SCOPE) scope_capture_completed();
        restore_interrupts(int_status);
    }
}

bool StartCaptureBlast(uint32_t freq, uint32_t length, const uint8_t* capturePins, uint8_t capturePinCount, uint8_t triggerPin, bool invertTrigger, CHANNEL_MODE captureMode) {
    int maxSamples;
    switch(captureMode) {
        case MODE_8_CHANNEL: maxSamples = CAPTURE_BUFFER_SIZE; break;
        case MODE_16_CHANNEL: maxSamples = CAPTURE_BUFFER_SIZE / 2; break;
    }
    if(length > maxSamples) return false;
    if(freq > MAX_BLAST_FREQ) return false;
    if(capturePinCount < 1 || capturePinCount > MAX_CHANNELS) return false;
    if(triggerPin > MAX_CHANNELS) return false;
    memset(captureBuffer, 0, sizeof(captureBuffer));
    lastPreSize = 0;
    lastPostSize = length;
    lastLoopCount = 0;
    lastCapturePinCount = capturePinCount;
    lastTriggerInverted = invertTrigger;
    lastCaptureComplexFast = false;
    lastCaptureMode = captureMode;
    for(uint8_t i = 0; i < capturePinCount; i++)
        lastCapturePins[i] = pinMap[capturePins[i]];
    triggerPin = pinMap[triggerPin];
    lastTriggerPin = triggerPin;
    float clockDiv = (float)clock_get_hz(clk_sys) / (float)(freq);
    capturePIO = pio1;
    pio_clear_instruction_memory(capturePIO);
    sm_Capture = pio_claim_unused_sm(capturePIO, true);
    pio_sm_clear_fifos(capturePIO, sm_Capture);
    pio_sm_restart(capturePIO, sm_Capture);
    captureOffset = pio_add_program(capturePIO, &BLAST_CAPTURE_program);
    for(int i = 0; i < PIN_MAP_SIZE; i++) {
        gpio_disable_pulls(pinMap[i]);
        pio_sm_set_consecutive_pindirs(capturePIO, sm_Capture, pinMap[i], 1, false);
    }
    for(uint8_t i = 0; i < PIN_MAP_SIZE; i++)
        pio_gpio_init(capturePIO, pinMap[i]);
    gpio_disable_pulls(triggerPin);
    pio_sm_set_consecutive_pindirs(capturePIO, sm_Capture, triggerPin, 1, false);
    pio_gpio_init(capturePIO, triggerPin);
    if(!invertTrigger) gpio_set_inover(triggerPin, 1);
    pio_sm_config smConfig = BLAST_CAPTURE_program_get_default_config(captureOffset);
    sm_config_set_in_pins(&smConfig, INPUT_PIN_BASE);
    sm_config_set_clkdiv(&smConfig, clockDiv);
    sm_config_set_in_shift(&smConfig, true, true, 0);
    sm_config_set_jmp_pin(&smConfig, triggerPin);
    pio_sm_set_enabled(capturePIO, sm_Capture, false);
    pio_sm_init(capturePIO, sm_Capture, captureOffset, &smConfig);
    configureBlastDMA(captureMode, length);
    pio_sm_set_enabled(capturePIO, sm_Capture, true);
    captureFinished = false;
    captureProcessed = false;
    lastCaptureType = CAPTURE_TYPE_BLAST;
    return true;
}

bool StartCaptureSimple(uint32_t freq, uint32_t preLength, uint32_t postLength, const uint8_t* capturePins, uint8_t capturePinCount, uint8_t triggerPin, bool invertTrigger, CHANNEL_MODE captureMode) {
    int maxSamples;
    switch(captureMode) {
        case MODE_8_CHANNEL: maxSamples = CAPTURE_BUFFER_SIZE; break;
        case MODE_16_CHANNEL: maxSamples = CAPTURE_BUFFER_SIZE / 2; break;
    }
    if(preLength + postLength > maxSamples) return false;
    if(freq > MAX_FREQ) return false;
    if(capturePinCount < 1 || capturePinCount > MAX_CHANNELS) return false;
    if(triggerPin > MAX_CHANNELS) return false;
    memset(captureBuffer, 0, sizeof(captureBuffer));
    lastPreSize = preLength;
    lastPostSize = postLength;
    lastLoopCount = 0;
    lastCapturePinCount = capturePinCount;
    lastTriggerInverted = invertTrigger;
    lastCaptureComplexFast = false;
    lastCaptureMode = captureMode;
    for(uint8_t i = 0; i < capturePinCount; i++)
        lastCapturePins[i] = pinMap[capturePins[i]];
    triggerPin = pinMap[triggerPin];
    lastTriggerPin = triggerPin;
    float clockDiv = (float)clock_get_hz(clk_sys) / (float)(freq * 2);
    capturePIO = pio1;
    pio_clear_instruction_memory(capturePIO);
    sm_Capture = pio_claim_unused_sm(capturePIO, true);
    pio_sm_clear_fifos(capturePIO, sm_Capture);
    pio_sm_restart(capturePIO, sm_Capture);
    if(invertTrigger)
        captureOffset = pio_add_program(capturePIO, &NEGATIVE_CAPTURE_program);
    else
        captureOffset = pio_add_program(capturePIO, &POSITIVE_CAPTURE_program);
    for(int i = 0; i < PIN_MAP_SIZE; i++) {
        gpio_disable_pulls(pinMap[i]);
        pio_sm_set_consecutive_pindirs(capturePIO, sm_Capture, pinMap[i], 1, false);
    }
    for(uint8_t i = 0; i < PIN_MAP_SIZE; i++)
        pio_gpio_init(capturePIO, pinMap[i]);
    gpio_disable_pulls(triggerPin);
    pio_sm_set_consecutive_pindirs(capturePIO, sm_Capture, triggerPin, 1, false);
    pio_gpio_init(capturePIO, triggerPin);
    pio_sm_config smConfig = invertTrigger ?
        NEGATIVE_CAPTURE_program_get_default_config(captureOffset) :
        POSITIVE_CAPTURE_program_get_default_config(captureOffset);
    sm_config_set_in_pins(&smConfig, INPUT_PIN_BASE);
    sm_config_set_clkdiv(&smConfig, clockDiv);
    sm_config_set_in_shift(&smConfig, true, true, 0);
    sm_config_set_jmp_pin(&smConfig, triggerPin);
    pio_interrupt_clear(capturePIO, 0);
    pio_set_irq0_source_enabled(capturePIO, pis_interrupt0, true);
    irq_set_exclusive_handler(PIO1_IRQ_0, simple_capture_completed);
    irq_set_enabled(PIO1_IRQ_0, true);
    pio_sm_init(capturePIO, sm_Capture, captureOffset, &smConfig);
    configureCaptureDMAs(captureMode);
    pio_sm_set_enabled(capturePIO, sm_Capture, true);
    pio_sm_put_blocking(capturePIO, sm_Capture, postLength - 1);
    captureFinished = false;
    captureProcessed = false;
    lastCaptureType = CAPTURE_TYPE_SIMPLE;
    return true;
}

bool StartCaptureNoTrigger(uint32_t freq, uint32_t length, const uint8_t* capturePins, uint8_t capturePinCount, CHANNEL_MODE captureMode) {
    int maxSamples;
    switch(captureMode) {
        case MODE_8_CHANNEL: maxSamples = CAPTURE_BUFFER_SIZE; break;
        case MODE_16_CHANNEL: maxSamples = CAPTURE_BUFFER_SIZE / 2; break;
    }
    if(length > maxSamples) return false;
    if(freq > MAX_FREQ) return false;
    if(capturePinCount < 1 || capturePinCount > MAX_CHANNELS) return false;
    memset(captureBuffer, 0, sizeof(captureBuffer));
    lastPreSize = 0;
    lastPostSize = length;
    lastLoopCount = 0;
    lastCapturePinCount = capturePinCount;
    lastTriggerInverted = false;
    lastCaptureComplexFast = false;
    lastCaptureMode = captureMode;
    for(uint8_t i = 0; i < capturePinCount; i++)
        lastCapturePins[i] = pinMap[capturePins[i]];
    lastTriggerPin = 0;
    float clockDiv = (float)clock_get_hz(clk_sys) / (float)(freq * 2);
    capturePIO = pio1;
    pio_clear_instruction_memory(capturePIO);
    sm_Capture = pio_claim_unused_sm(capturePIO, true);
    pio_sm_clear_fifos(capturePIO, sm_Capture);
    pio_sm_restart(capturePIO, sm_Capture);
    captureOffset = pio_add_program(capturePIO, &NO_TRIGGER_CAPTURE_program);
    for(int i = 0; i < PIN_MAP_SIZE; i++) {
        gpio_disable_pulls(pinMap[i]);
        pio_sm_set_consecutive_pindirs(capturePIO, sm_Capture, pinMap[i], 1, false);
    }
    for(uint8_t i = 0; i < PIN_MAP_SIZE; i++)
        pio_gpio_init(capturePIO, pinMap[i]);
    pio_sm_config smConfig = NO_TRIGGER_CAPTURE_program_get_default_config(captureOffset);
    sm_config_set_in_pins(&smConfig, INPUT_PIN_BASE);
    sm_config_set_clkdiv(&smConfig, clockDiv);
    sm_config_set_in_shift(&smConfig, true, true, 0);
    pio_interrupt_clear(capturePIO, 0);
    pio_set_irq0_source_enabled(capturePIO, pis_interrupt0, true);
    irq_set_exclusive_handler(PIO1_IRQ_0, no_trigger_capture_completed);
    irq_set_enabled(PIO1_IRQ_0, true);
    pio_sm_init(capturePIO, sm_Capture, captureOffset, &smConfig);
    configureCaptureDMAs(captureMode);
    pio_sm_set_enabled(capturePIO, sm_Capture, true);
    pio_sm_put_blocking(capturePIO, sm_Capture, length);
    captureFinished = false;
    captureProcessed = false;
    lastCaptureType = CAPTURE_TYPE_NO_TRIGGER;
    return true;
}

bool StartCaptureNoTriggerSkip(uint32_t freq, uint32_t length, const uint8_t* capturePins, uint8_t capturePinCount, CHANNEL_MODE captureMode, uint8_t skipPin) {
    int maxSamples;
    switch(captureMode) {
        case MODE_8_CHANNEL: maxSamples = CAPTURE_BUFFER_SIZE; break;
        case MODE_16_CHANNEL: maxSamples = CAPTURE_BUFFER_SIZE / 2; break;
    }
    if(length > maxSamples) return false;
    if(freq > MAX_FREQ) return false;
    if(capturePinCount < 1 || capturePinCount > MAX_CHANNELS) return false;
    memset(captureBuffer, 0, sizeof(captureBuffer));
    lastPreSize = 0;
    lastPostSize = length;
    lastLoopCount = 0;
    lastCapturePinCount = capturePinCount;
    lastTriggerInverted = false;
    lastCaptureComplexFast = false;
    lastCaptureMode = captureMode;
    for(uint8_t i = 0; i < capturePinCount; i++)
        lastCapturePins[i] = pinMap[capturePins[i]];
    lastTriggerPin = 0;
    float clockDiv = (float)clock_get_hz(clk_sys) / (float)(freq * 2);
    capturePIO = pio1;
    pio_clear_instruction_memory(capturePIO);
    sm_Capture = pio_claim_unused_sm(capturePIO, true);
    pio_sm_clear_fifos(capturePIO, sm_Capture);
    pio_sm_restart(capturePIO, sm_Capture);
    captureOffset = pio_add_program(capturePIO, &NO_TRIGGER_CAPTURE_program);
    for(int i = 0; i < PIN_MAP_SIZE; i++) {
        if(pinMap[i] == skipPin) continue;
        gpio_disable_pulls(pinMap[i]);
        pio_sm_set_consecutive_pindirs(capturePIO, sm_Capture, pinMap[i], 1, false);
    }
    for(uint8_t i = 0; i < PIN_MAP_SIZE; i++) {
        if(pinMap[i] == skipPin) continue;
        pio_gpio_init(capturePIO, pinMap[i]);
    }
    pio_sm_config smConfig = NO_TRIGGER_CAPTURE_program_get_default_config(captureOffset);
    sm_config_set_in_pins(&smConfig, INPUT_PIN_BASE);
    sm_config_set_clkdiv(&smConfig, clockDiv);
    sm_config_set_in_shift(&smConfig, true, true, 0);
    pio_interrupt_clear(capturePIO, 0);
    pio_set_irq0_source_enabled(capturePIO, pis_interrupt0, true);
    irq_set_exclusive_handler(PIO1_IRQ_0, no_trigger_capture_completed);
    irq_set_enabled(PIO1_IRQ_0, true);
    pio_sm_init(capturePIO, sm_Capture, captureOffset, &smConfig);
    configureCaptureDMAs(captureMode);
    pio_sm_set_enabled(capturePIO, sm_Capture, true);
    pio_sm_put_blocking(capturePIO, sm_Capture, length);
    captureFinished = false;
    captureProcessed = false;
    lastCaptureType = CAPTURE_TYPE_NO_TRIGGER;
    return true;
}

#ifdef SUPPORTS_COMPLEX_TRIGGER
bool StartCaptureComplex(uint32_t freq, uint32_t preLength, uint32_t postLength, const uint8_t* capturePins, uint8_t capturePinCount, uint8_t triggerPinBase, uint8_t triggerPinCount, uint16_t triggerValue, CHANNEL_MODE captureMode) {
    int maxSamples;
    switch(captureMode) {
        case MODE_8_CHANNEL: maxSamples = CAPTURE_BUFFER_SIZE; break;
        case MODE_16_CHANNEL: maxSamples = CAPTURE_BUFFER_SIZE / 2; break;
    }
    if(preLength + postLength > maxSamples) return false;
    if(freq > MAX_FREQ) return false;
    if(capturePinCount > MAX_CHANNELS) return false;
    if(triggerPinBase > 15 || triggerPinCount > 16 || triggerPinCount < 1 || triggerPinCount + triggerPinBase > 16) return false;
    memset(captureBuffer, 0, sizeof(captureBuffer));
    lastPreSize = preLength;
    lastPostSize = postLength;
    lastLoopCount = 0;
    lastCapturePinCount = capturePinCount;
    lastCaptureComplexFast = true;
    lastCaptureMode = captureMode;
    for(uint8_t i = 0; i < capturePinCount; i++)
        lastCapturePins[i] = pinMap[capturePins[i]];
    triggerPinBase = pinMap[triggerPinBase];
    lastTriggerPinBase = triggerPinBase;
    float clockDiv = (float)clock_get_hz(clk_sys) / (float)(freq * 2);
    capturePIO = pio1;
    pio_clear_instruction_memory(capturePIO);
    gpio_disable_pulls(COMPLEX_TRIGGER_OUT_PIN);
    gpio_disable_pulls(COMPLEX_TRIGGER_IN_PIN);
    pio_gpio_init(capturePIO, COMPLEX_TRIGGER_OUT_PIN);
    pio_gpio_init(capturePIO, COMPLEX_TRIGGER_IN_PIN);
    for(uint8_t i = 0; i < PIN_MAP_SIZE; i++) {
        gpio_disable_pulls(pinMap[i]);
        pio_gpio_init(capturePIO, pinMap[i]);
    }
    sm_Capture = pio_claim_unused_sm(capturePIO, true);
    pio_sm_clear_fifos(capturePIO, sm_Capture);
    pio_sm_restart(capturePIO, sm_Capture);
    captureOffset = pio_add_program(capturePIO, &COMPLEX_CAPTURE_program);
    for(int i = 0; i < PIN_MAP_SIZE; i++)
        pio_sm_set_consecutive_pindirs(capturePIO, sm_Capture, pinMap[i], 1, false);
    pio_sm_config smConfig = COMPLEX_CAPTURE_program_get_default_config(captureOffset);
    sm_config_set_in_pins(&smConfig, INPUT_PIN_BASE);
    sm_config_set_clkdiv(&smConfig, clockDiv);
    sm_config_set_in_shift(&smConfig, false, true, 29);
    sm_config_set_jmp_pin(&smConfig, COMPLEX_TRIGGER_IN_PIN);
    pio_interrupt_clear(capturePIO, 0);
    pio_set_irq0_source_enabled(capturePIO, pis_interrupt0, true);
    irq_set_exclusive_handler(PIO1_IRQ_0, complex_capture_completed);
    irq_set_enabled(PIO1_IRQ_0, true);
    pio_sm_init(capturePIO, sm_Capture, captureOffset, &smConfig);
    sm_Trigger = pio_claim_unused_sm(capturePIO, true);
    pio_sm_clear_fifos(capturePIO, sm_Trigger);
    pio_sm_restart(capturePIO, sm_Trigger);
    COMPLEX_TRIGGER_program_instructions[5] = 0x6040 | triggerPinCount;
    triggerOffset = pio_add_program(capturePIO, &COMPLEX_TRIGGER_program);
    pio_sm_set_consecutive_pindirs(capturePIO, sm_Trigger, COMPLEX_TRIGGER_OUT_PIN, 1, true);
    pio_sm_set_consecutive_pindirs(capturePIO, sm_Trigger, triggerPinBase, triggerPinCount, false);
    smConfig = COMPLEX_TRIGGER_program_get_default_config(triggerOffset);
    sm_config_set_in_pins(&smConfig, triggerPinBase);
    sm_config_set_set_pins(&smConfig, COMPLEX_TRIGGER_OUT_PIN, 1);
    sm_config_set_clkdiv(&smConfig, 1);
    sm_config_set_in_shift(&smConfig, false, false, 0);
    pio_sm_init(capturePIO, sm_Trigger, triggerOffset, &smConfig);
    configureCaptureDMAs(captureMode);
    pio_sm_set_enabled(capturePIO, sm_Capture, true);
    pio_sm_put_blocking(capturePIO, sm_Capture, postLength - 1);
    pio_sm_set_enabled(capturePIO, sm_Trigger, true);
    pio_sm_put_blocking(capturePIO, sm_Trigger, triggerValue);
    captureFinished = false;
    captureProcessed = false;
    lastCaptureType = CAPTURE_TYPE_COMPLEX;
    return true;
}

bool StartCaptureFast(uint32_t freq, uint32_t preLength, uint32_t postLength, const uint8_t* capturePins, uint8_t capturePinCount, uint8_t triggerPinBase, uint8_t triggerPinCount, uint16_t triggerValue, CHANNEL_MODE captureMode) {
    int maxSamples;
    switch(captureMode) {
        case MODE_8_CHANNEL: maxSamples = CAPTURE_BUFFER_SIZE; break;
        case MODE_16_CHANNEL: maxSamples = CAPTURE_BUFFER_SIZE / 2; break;
    }
    if(preLength + postLength > maxSamples) return false;
    if(freq > MAX_FREQ) return false;
    if(capturePinCount > MAX_CHANNELS) return false;
    if(triggerPinBase > 15 || triggerPinCount > 5 || triggerPinCount < 1 || triggerPinCount + triggerPinBase > 16) return false;
    memset(captureBuffer, 0, sizeof(captureBuffer));
    lastPreSize = preLength;
    lastPostSize = postLength;
    lastLoopCount = 0;
    lastCapturePinCount = capturePinCount;
    lastCaptureComplexFast = true;
    lastCaptureMode = captureMode;
    for(uint8_t i = 0; i < capturePinCount; i++)
        lastCapturePins[i] = pinMap[capturePins[i]];
    triggerPinBase = pinMap[triggerPinBase];
    lastTriggerPinBase = triggerPinBase;
    float clockDiv = (float)clock_get_hz(clk_sys) / (float)(freq * 2);
    capturePIO = pio1;
    triggerPIO = pio1;
    pio_clear_instruction_memory(triggerPIO);
    gpio_disable_pulls(COMPLEX_TRIGGER_OUT_PIN);
    gpio_disable_pulls(COMPLEX_TRIGGER_IN_PIN);
    pio_gpio_init(triggerPIO, COMPLEX_TRIGGER_OUT_PIN);
    pio_gpio_init(capturePIO, COMPLEX_TRIGGER_IN_PIN);
    for(uint8_t i = 0; i < PIN_MAP_SIZE; i++) {
        gpio_disable_pulls(pinMap[i]);
        pio_gpio_init(capturePIO, pinMap[i]);
    }
    sm_Capture = pio_claim_unused_sm(capturePIO, true);
    pio_sm_clear_fifos(capturePIO, sm_Capture);
    pio_sm_restart(capturePIO, sm_Capture);
    captureOffset = pio_add_program(capturePIO, &FAST_CAPTURE_program);
    for(int i = 0; i < PIN_MAP_SIZE; i++)
        pio_sm_set_consecutive_pindirs(capturePIO, sm_Capture, pinMap[i], 1, false);
    pio_sm_config smConfig = FAST_CAPTURE_program_get_default_config(captureOffset);
    sm_config_set_in_pins(&smConfig, INPUT_PIN_BASE);
    sm_config_set_clkdiv(&smConfig, clockDiv);
    sm_config_set_in_shift(&smConfig, false, true, 29);
    sm_config_set_jmp_pin(&smConfig, COMPLEX_TRIGGER_IN_PIN);
    pio_interrupt_clear(capturePIO, 0);
    pio_sm_init(capturePIO, sm_Capture, captureOffset, &smConfig);
    sm_Trigger = pio_claim_unused_sm(triggerPIO, true);
    pio_sm_clear_fifos(triggerPIO, sm_Trigger);
    pio_sm_restart(triggerPIO, sm_Trigger);
    uint8_t triggerFirstInstruction = create_fast_trigger_program(triggerValue, triggerPinCount);
    triggerOffset = pio_add_program(triggerPIO, &FAST_TRIGGER_program);
    pio_sm_set_consecutive_pindirs(triggerPIO, sm_Trigger, COMPLEX_TRIGGER_OUT_PIN, 1, true);
    pio_sm_set_consecutive_pindirs(triggerPIO, sm_Trigger, triggerPinBase, triggerPinCount, false);
    smConfig = FAST_TRIGGER_program_get_default_config(triggerOffset);
    sm_config_set_in_pins(&smConfig, triggerPinBase);
    sm_config_set_set_pins(&smConfig, COMPLEX_TRIGGER_OUT_PIN, 1);
    sm_config_set_sideset_pins(&smConfig, COMPLEX_TRIGGER_OUT_PIN);
    sm_config_set_clkdiv(&smConfig, 1);
    configureCaptureDMAs(captureMode);
    pio_sm_set_enabled(capturePIO, sm_Capture, true);
    pio_sm_put_blocking(capturePIO, sm_Capture, postLength - 1);
    pio_sm_init(triggerPIO, sm_Trigger, triggerOffset, &smConfig);
    pio_sm_set_enabled(triggerPIO, sm_Trigger, true);
    captureFinished = false;
    captureProcessed = false;
    lastCaptureType = CAPTURE_TYPE_FAST;
    return true;
}
#endif

bool IsCapturing() { return !captureFinished; }

uint GetCaptureSM() { return sm_Capture; }

uint8_t* GetBuffer(uint32_t* bufferSize, uint32_t* firstSample, CHANNEL_MODE* captureMode) {
    uint32_t totalSamples = lastPreSize + (lastPostSize * (lastLoopCount + 1));
    if(!captureProcessed) {
        uint32_t maxSize;
        switch(lastCaptureMode) {
            case MODE_8_CHANNEL: maxSize = CAPTURE_BUFFER_SIZE; break;
            case MODE_16_CHANNEL: maxSize = CAPTURE_BUFFER_SIZE / 2; break;
        }
        if(lastTail < totalSamples - 1)
            lastStartPosition = (maxSize - totalSamples) + lastTail + 1;
        else
            lastStartPosition = lastTail - totalSamples + 1;
        uint32_t currentPos = lastStartPosition;
        switch(lastCaptureMode) {
            case MODE_8_CHANNEL: {
                uint8_t* buffer = captureBuffer;
                for(uint32_t buc = 0; buc < totalSamples; buc++) {
                    uint8_t oldValue = buffer[currentPos];
                    uint8_t newValue = 0;
                    for(int pin = 0; pin < lastCapturePinCount; pin++) {
                        uint8_t lastPin = lastCapturePins[pin] - INPUT_PIN_BASE;
                        newValue |= (((oldValue & (1 << lastPin))) >> lastPin) << pin;
                    }
                    buffer[currentPos++] = newValue;
                    if(currentPos >= maxSize) currentPos = 0;
                }
            } break;
            case MODE_16_CHANNEL: {
                uint16_t* buffer = (uint16_t*)captureBuffer;
                for(uint32_t buc = 0; buc < totalSamples; buc++) {
                    uint16_t oldValue = buffer[currentPos];
                    uint16_t newValue = 0;
                    for(int pin = 0; pin < lastCapturePinCount; pin++) {
                        uint8_t lastPin = lastCapturePins[pin] - INPUT_PIN_BASE;
                        newValue |= (((oldValue & (1 << lastPin))) >> lastPin) << pin;
                    }
                    buffer[currentPos++] = newValue;
                    if(currentPos >= maxSize) currentPos = 0;
                }
            } break;
        }
        captureProcessed = true;
    }
    *captureMode = lastCaptureMode;
    *bufferSize = totalSamples;
    *firstSample = lastStartPosition;
    return captureBuffer;
}

/* ─── ADC CAPTURE ────────────────────────────────────────────────────────── */

#include "hardware/adc.h"
#include "hardware/timer.h"

#define ADC_MAX_SAMPLES 32768
#define ADC_BUF_SIZE ADC_MAX_SAMPLES

#define adcBuffer ((uint16_t*)captureBuffer)
static volatile uint32_t adc_sample_index = 0;
static volatile uint32_t adc_total_samples = 0;
static volatile bool adc_capturing = false;
static repeating_timer_t adc_timer;

static bool adc_timer_callback(repeating_timer_t *rt) {
    if(adc_sample_index < adc_total_samples) {
        adcBuffer[adc_sample_index++] = adc_read();
        if(adc_sample_index >= adc_total_samples) {
            adc_capturing = false;
            return false;  /* stop repeating */
        }
    }
    return true;  /* continue */
}

bool StartAdcCapture(uint32_t pin, uint32_t freq, uint32_t samples) {
    if(samples > ADC_MAX_SAMPLES) samples = ADC_MAX_SAMPLES;
    if(pin < 26 || pin > 29) return false;

    adc_init();
    adc_gpio_init(pin);

    /* Select ADC channel: GPIO26=0, GPIO27=1, GPIO28=2, GPIO29=3 */
    adc_select_input(pin - 26);

    adc_sample_index = 0;
    adc_total_samples = samples;
    adc_capturing = true;

    /* Calculate timer interval in microseconds.
     * For freq in Hz, period = 1000000 / freq us.
     * repeating_timer takes negative interval for periodic. */
    int64_t interval_us = -(int64_t)(1000000 / freq);
    if(interval_us > -1) interval_us = -1;

    if(!add_repeating_timer_us(interval_us, adc_timer_callback, NULL, &adc_timer)) {
        adc_capturing = false;
        return false;
    }

    return true;
}

bool IsAdcCapturing() {
    return adc_capturing;
}

uint16_t* GetAdcBuffer(uint32_t* size, uint32_t* first) {
    *size = adc_sample_index;
    *first = 0;
    return adcBuffer;
}

static uint32_t capture_active_bank_claim = 0;

bool capture_validate_claimed_pins(const uint8_t* channels, uint8_t channel_count, uint8_t trigger_pin) {
    if (channel_count == 0 || channel_count > MAX_CHANNELS) return false;
    if (channel_count > 8) return false;

    uint8_t selected[MAX_CHANNELS];
    for (uint8_t i = 0; i < channel_count; i++) selected[i] = channels[i];

    uint8_t span_width = 1;
    uint8_t low = selected[0];
    uint8_t high = selected[0];
    bool trigger_is_selected = (trigger_pin == 255);
    for (uint8_t i = 0; i < channel_count; i++) {
        const uint8_t pin = selected[i];
        if (!pin_owner_is_flexible(pin)) return false;
        if (pin < low) low = pin;
        if (pin > high) high = pin;
        if (trigger_pin != 255 && trigger_pin == pin) trigger_is_selected = true;
    }
    span_width = (uint8_t)(high - low + 1);
    if (span_width > 8) return false;
    if (trigger_pin != 255 && !trigger_is_selected) return false;
    return true;
}

bool capture_claim_bank(const uint8_t* channels, uint8_t channel_count,
                        uint8_t trigger_pin, uint32_t* claim_handle) {
    if (capture_active_bank_claim != 0) return false;
    if (channels == NULL || claim_handle == NULL ||
        !capture_validate_claimed_pins(channels, channel_count, trigger_pin)) {
        return false;
    }

    uint8_t low = channels[0];
    uint8_t high = channels[0];
    for (uint8_t i = 1; i < channel_count; i++) {
        if (channels[i] < low) low = channels[i];
        if (channels[i] > high) high = channels[i];
    }

    uint8_t span_width = (uint8_t)(high - low + 1);
    uint8_t span[8];
    for (uint8_t i = 0; i < span_width; i++) {
        span[i] = (uint8_t)(low + i);
    }

    uint32_t handle = 0;
    if (!pin_owner_claim_pins(span, span_width, GPIO_FUNC_PIO0, 0, &handle)) {
        return false;
    }

    *claim_handle = handle;
    capture_active_bank_claim = handle;
    return true;
}

bool capture_release_bank(uint32_t claim_handle, pin_owner_result_t* result) {
    if (claim_handle == 0 || claim_handle != capture_active_bank_claim) {
        if (result != NULL) *result = PIN_OWNER_ERROR_NOT_FOUND;
        return false;
    }
    if (IsCapturing()) {
        StopCapture();
    }
    bool ok = pin_owner_release_claim(claim_handle, result);
    if (ok) {
        capture_active_bank_claim = 0;
    }
    return ok;
}

uint32_t capture_active_bank_handle(void) {
    return capture_active_bank_claim;
}

/* ═══════════════════════════════════════════════════════════════════
 * DECOUPLED HIGH-SPEED CAPTURE ENGINE: MODES A, B, C
 * ═══════════════════════════════════════════════════════════════════ */

static capture_engine_mode_t g_capture_engine_mode = CAPTURE_ENGINE_RAW;
static trigger_config_t g_trigger_config = {
    .mask = 0xFFFF,
    .match = 0x0001,
    .type = 1,
    .pre_pct = 50,
    .pre_size = SCOPE_PRE_SIZE,
    .post_size = SCOPE_POST_SIZE
};
static uint32_t g_rle_threshold = 65535;

void capture_set_engine_mode(capture_engine_mode_t mode) {
    g_capture_engine_mode = mode;
}

capture_engine_mode_t capture_get_engine_mode(void) {
    return g_capture_engine_mode;
}

const char* capture_get_engine_mode_name(void) {
    switch (g_capture_engine_mode) {
        case CAPTURE_ENGINE_STREAM: return "STREAM";
        case CAPTURE_ENGINE_TURBO:  return "TURBO";
        case CAPTURE_ENGINE_SCOPE:  return "SCOPE";
        case CAPTURE_ENGINE_RAW:
        default: return "RAW";
    }
}

void capture_set_rle_threshold(uint32_t ticks) {
    if (ticks == 0 || ticks > 65535) ticks = 65535;
    g_rle_threshold = ticks;
}

uint32_t capture_get_rle_threshold(void) {
    return g_rle_threshold;
}

void capture_set_trigger_config(const trigger_config_t *cfg) {
    if (!cfg) return;
    g_trigger_config = *cfg;
    if (g_trigger_config.pre_size == 0) g_trigger_config.pre_size = SCOPE_PRE_SIZE;
    if (g_trigger_config.post_size == 0) g_trigger_config.post_size = SCOPE_POST_SIZE;
}

const trigger_config_t* capture_get_trigger_config(void) {
    return &g_trigger_config;
}

/* ─── Mode A: In-Hardware PIO RLE Streaming ───────────────────────── */
static uint32_t g_rle_packet_count = 0;
static bool g_rle_capturing = false;

static void rle_capture_completed(void) {
    hw_clear_bits(&dma_hw->ch[dmaPingPong0].al1_ctrl, DMA_CH0_CTRL_TRIG_EN_BITS);
    pio_sm_set_enabled(capturePIO, sm_Capture, false);
    pio_remove_program(capturePIO, &pio_rle_capture_program, captureOffset);
    dma_channel_unclaim(dmaPingPong0);
    pio_sm_unclaim(capturePIO, sm_Capture);
    g_rle_capturing = false;
    captureFinished = true;
}

static void rle_dma_handler(void) {
    if (dma_channel_get_irq0_status(dmaPingPong0)) {
        dma_channel_acknowledge_irq0(dmaPingPong0);
        g_rle_packet_count = SCOPE_TOTAL_SIZE / 4;
        rle_capture_completed();
    }
}

bool StartCaptureRle(uint32_t freq, const uint8_t* capturePins, uint8_t capturePinCount) {
    if (capturePinCount < 1 || capturePinCount > MAX_CHANNELS) return false;
    if (freq > MAX_FREQ) freq = MAX_FREQ;

    memset(captureBuffer, 0, SCOPE_TOTAL_SIZE);
    g_rle_packet_count = 0;
    captureFinished = false;
    captureProcessed = false;

    float clockDiv = (float)clock_get_hz(clk_sys) / (float)freq;
    capturePIO = pio1;
    pio_clear_instruction_memory(capturePIO);
    sm_Capture = pio_claim_unused_sm(capturePIO, true);
    pio_sm_clear_fifos(capturePIO, sm_Capture);
    pio_sm_restart(capturePIO, sm_Capture);

    captureOffset = pio_add_program(capturePIO, &pio_rle_capture_program);

    for (int i = 0; i < PIN_MAP_SIZE; i++) {
        gpio_disable_pulls(pinMap[i]);
        pio_sm_set_consecutive_pindirs(capturePIO, sm_Capture, pinMap[i], 1, false);
        pio_gpio_init(capturePIO, pinMap[i]);
    }

    pio_sm_config smConfig = pio_rle_capture_program_get_default_config(captureOffset);
    sm_config_set_in_pins(&smConfig, INPUT_PIN_BASE);
    sm_config_set_clkdiv(&smConfig, clockDiv);
    sm_config_set_in_shift(&smConfig, false, false, 32);

    pio_sm_set_enabled(capturePIO, sm_Capture, false);
    pio_sm_init(capturePIO, sm_Capture, captureOffset, &smConfig);

    dmaPingPong0 = dma_claim_unused_channel(true);
    dma_channel_config dmaConfig = dma_channel_get_default_config(dmaPingPong0);
    channel_config_set_read_increment(&dmaConfig, false);
    channel_config_set_write_increment(&dmaConfig, true);
    channel_config_set_transfer_data_size(&dmaConfig, DMA_SIZE_32);
    channel_config_set_dreq(&dmaConfig, pio_get_dreq(capturePIO, sm_Capture, false));
    channel_config_set_enable(&dmaConfig, true);

    dma_channel_set_irq0_enabled(dmaPingPong0, true);
    irq_set_exclusive_handler(DMA_IRQ_0, rle_dma_handler);
    irq_set_enabled(DMA_IRQ_0, true);
    irq_set_priority(DMA_IRQ_0, 0);

    uint32_t totalWords = SCOPE_TOTAL_SIZE / 4;
    dma_channel_configure(dmaPingPong0, &dmaConfig, captureBuffer, &capturePIO->rxf[sm_Capture], totalWords, true);

    pio_sm_set_enabled(capturePIO, sm_Capture, true);
    g_rle_capturing = true;
    lastCaptureType = CAPTURE_TYPE_RLE;
    return true;
}

uint32_t* capture_get_rle_buffer(uint32_t *packet_count) {
    if (packet_count) {
        if (g_rle_capturing) {
            uint32_t transferred = (SCOPE_TOTAL_SIZE / 4) - dma_channel_hw_addr(dmaPingPong0)->transfer_count;
            *packet_count = transferred;
        } else {
            *packet_count = g_rle_packet_count;
        }
    }
    return (uint32_t*)captureBuffer;
}

/* ─── Mode B: Dual-Core Compression Pipeline ──────────────────────── */
#define TURBO_PINGPONG_SIZE (64 * 1024)
static volatile bool g_core1_running = false;
static volatile bool g_turbo_capturing = false;
static uint32_t g_turbo_compressed_size = 0;

#define SIO_CMD_PROCESS_BUF0 0x01
#define SIO_CMD_PROCESS_BUF1 0x02
#define SIO_CMD_STOP         0xFF

static void core1_worker_task(void) {
    while (1) {
        uint32_t cmd = multicore_fifo_pop_blocking();
        if (cmd == SIO_CMD_STOP) {
            break;
        }

        uint8_t *src_buf = (cmd == SIO_CMD_PROCESS_BUF0) ? captureBuffer : (captureBuffer + TURBO_PINGPONG_SIZE);
        uint8_t *dst_buf = captureBuffer + (2 * TURBO_PINGPONG_SIZE);
        size_t dst_cap = SCOPE_TOTAL_SIZE - (2 * TURBO_PINGPONG_SIZE);

        size_t comp_bytes = bit_delta_compress_block((const uint32_t*)src_buf, TURBO_PINGPONG_SIZE / 4, dst_buf, dst_cap);
        bool overflow = (comp_bytes == 0 || comp_bytes > dst_cap);
        compression_pipeline_record_buffer(TURBO_PINGPONG_SIZE, comp_bytes, overflow);

        g_turbo_compressed_size = (uint32_t)comp_bytes;
        multicore_fifo_push_blocking((uint32_t)comp_bytes);
    }
}

static void turbo_capture_completed(void) {
    hw_clear_bits(&dma_hw->ch[dmaPingPong0].al1_ctrl, DMA_CH0_CTRL_TRIG_EN_BITS);
    hw_clear_bits(&dma_hw->ch[dmaPingPong1].al1_ctrl, DMA_CH0_CTRL_TRIG_EN_BITS);
    pio_sm_set_enabled(capturePIO, sm_Capture, false);
    pio_remove_program(capturePIO, &NO_TRIGGER_CAPTURE_program, captureOffset);
    dma_channel_unclaim(dmaPingPong0);
    dma_channel_unclaim(dmaPingPong1);
    pio_sm_unclaim(capturePIO, sm_Capture);
    g_turbo_capturing = false;
    captureFinished = true;
}

static void turbo_dma_handler(void) {
    if (dma_channel_get_irq0_status(dmaPingPong0)) {
        dma_channel_acknowledge_irq0(dmaPingPong0);
        multicore_fifo_push_blocking(SIO_CMD_PROCESS_BUF0);
    } else if (dma_channel_get_irq0_status(dmaPingPong1)) {
        dma_channel_acknowledge_irq0(dmaPingPong1);
        multicore_fifo_push_blocking(SIO_CMD_PROCESS_BUF1);
    }
}

bool StartCaptureTurbo(uint32_t freq, uint32_t length, const uint8_t* capturePins, uint8_t capturePinCount, CHANNEL_MODE captureMode) {
    if (capturePinCount < 1 || capturePinCount > MAX_CHANNELS) return false;
    compression_pipeline_reset_stats();
    g_turbo_compressed_size = 0;
    captureFinished = false;

    if (!g_core1_running) {
        multicore_launch_core1(core1_worker_task);
        g_core1_running = true;
    }

    enum dma_channel_transfer_size transferSize = (captureMode == MODE_16_CHANNEL) ? DMA_SIZE_16 : DMA_SIZE_8;
    uint32_t transferCount = (captureMode == MODE_16_CHANNEL) ? (TURBO_PINGPONG_SIZE / 2) : TURBO_PINGPONG_SIZE;

    float clockDiv = (float)clock_get_hz(clk_sys) / (float)freq;
    capturePIO = pio1;
    pio_clear_instruction_memory(capturePIO);
    sm_Capture = pio_claim_unused_sm(capturePIO, true);
    pio_sm_clear_fifos(capturePIO, sm_Capture);
    pio_sm_restart(capturePIO, sm_Capture);
    captureOffset = pio_add_program(capturePIO, &NO_TRIGGER_CAPTURE_program);

    for (int i = 0; i < PIN_MAP_SIZE; i++) {
        gpio_disable_pulls(pinMap[i]);
        pio_sm_set_consecutive_pindirs(capturePIO, sm_Capture, pinMap[i], 1, false);
        pio_gpio_init(capturePIO, pinMap[i]);
    }

    pio_sm_config smConfig = NO_TRIGGER_CAPTURE_program_get_default_config(captureOffset);
    sm_config_set_in_pins(&smConfig, INPUT_PIN_BASE);
    sm_config_set_clkdiv(&smConfig, clockDiv);
    sm_config_set_in_shift(&smConfig, true, true, (captureMode == MODE_16_CHANNEL) ? 16 : 8);

    pio_sm_set_enabled(capturePIO, sm_Capture, false);
    pio_sm_init(capturePIO, sm_Capture, captureOffset, &smConfig);

    dmaPingPong0 = dma_claim_unused_channel(true);
    dmaPingPong1 = dma_claim_unused_channel(true);

    dma_channel_config cfg0 = dma_channel_get_default_config(dmaPingPong0);
    channel_config_set_read_increment(&cfg0, false);
    channel_config_set_write_increment(&cfg0, true);
    channel_config_set_transfer_data_size(&cfg0, transferSize);
    channel_config_set_chain_to(&cfg0, dmaPingPong1);
    channel_config_set_dreq(&cfg0, pio_get_dreq(capturePIO, sm_Capture, false));
    channel_config_set_enable(&cfg0, true);
    dma_channel_set_irq0_enabled(dmaPingPong0, true);

    dma_channel_config cfg1 = dma_channel_get_default_config(dmaPingPong1);
    channel_config_set_read_increment(&cfg1, false);
    channel_config_set_write_increment(&cfg1, true);
    channel_config_set_transfer_data_size(&cfg1, transferSize);
    channel_config_set_chain_to(&cfg1, dmaPingPong0);
    channel_config_set_dreq(&cfg1, pio_get_dreq(capturePIO, sm_Capture, false));
    channel_config_set_enable(&cfg1, true);
    dma_channel_set_irq0_enabled(dmaPingPong1, true);

    irq_set_exclusive_handler(DMA_IRQ_0, turbo_dma_handler);
    irq_set_enabled(DMA_IRQ_0, true);
    irq_set_priority(DMA_IRQ_0, 0);

    dma_channel_configure(dmaPingPong1, &cfg1, captureBuffer + TURBO_PINGPONG_SIZE, &capturePIO->rxf[sm_Capture], transferCount, false);
    dma_channel_configure(dmaPingPong0, &cfg0, captureBuffer, &capturePIO->rxf[sm_Capture], transferCount, true);

    pio_sm_set_enabled(capturePIO, sm_Capture, true);
    g_turbo_capturing = true;
    lastCaptureType = CAPTURE_TYPE_TURBO;
    return true;
}

uint8_t* capture_get_turbo_buffer(uint32_t *compressed_size) {
    if (compressed_size) *compressed_size = g_turbo_compressed_size;
    return captureBuffer + (2 * TURBO_PINGPONG_SIZE);
}

/* ─── Mode C: Circular Oscilloscope Pre/Post-Trigger ──────────────── */
static uint32_t g_scope_pre_bytes = SCOPE_PRE_SIZE;
static uint32_t g_scope_post_bytes = SCOPE_POST_SIZE;
static uint32_t g_scope_trigger_write_addr = 0;
static uint8_t* g_scope_pre_ring_ptr = NULL;
static uint8_t* g_scope_post_buf_ptr = NULL;
static bool g_scope_triggered = false;
static bool g_scope_capturing = false;

static void scope_capture_completed(void) {
    hw_clear_bits(&dma_hw->ch[dmaPingPong0].al1_ctrl, DMA_CH0_CTRL_TRIG_EN_BITS);
    hw_clear_bits(&dma_hw->ch[dmaPingPong1].al1_ctrl, DMA_CH0_CTRL_TRIG_EN_BITS);
    pio_sm_set_enabled(capturePIO, sm_Capture, false);
    pio_sm_set_enabled(capturePIO, sm_Trigger, false);
    pio_remove_program(capturePIO, &NO_TRIGGER_CAPTURE_program, captureOffset);
    pio_remove_program(capturePIO, &pio_trigger_qualifier_program, triggerOffset);
    dma_channel_unclaim(dmaPingPong0);
    dma_channel_unclaim(dmaPingPong1);
    pio_sm_unclaim(capturePIO, sm_Capture);
    pio_sm_unclaim(capturePIO, sm_Trigger);
    g_scope_capturing = false;
    captureFinished = true;
}

static void scope_dma_post_completed(void) {
    dma_channel_acknowledge_irq0(dmaPingPong1);
    g_scope_trigger_write_addr = dma_hw->ch[dmaPingPong0].write_addr;
    scope_capture_completed();
}

static void pio_trigger_irq_handler(void) {
    if (pio_interrupt_get(capturePIO, 1)) {
        pio_interrupt_clear(capturePIO, 1);
        g_scope_triggered = true;
        dma_channel_start(dmaPingPong1);
    }
}

bool StartCaptureScope(uint32_t freq, uint32_t preBytes, uint32_t postBytes, const uint8_t* capturePins, uint8_t capturePinCount, uint8_t triggerPin, uint8_t triggerType, CHANNEL_MODE captureMode) {
    if (capturePinCount < 1 || capturePinCount > MAX_CHANNELS) return false;

    g_scope_pre_ring_ptr = (uint8_t*)(((uintptr_t)captureBuffer + 0xFFFF) & ~0xFFFF);
    g_scope_post_buf_ptr = g_scope_pre_ring_ptr + SCOPE_PRE_SIZE;

    g_scope_pre_bytes = SCOPE_PRE_SIZE;
    uint32_t maxPost = (uint32_t)((captureBuffer + SCOPE_TOTAL_SIZE) - g_scope_post_buf_ptr);
    g_scope_post_bytes = (postBytes > maxPost) ? maxPost : (postBytes > 0 ? postBytes : SCOPE_POST_SIZE);

    g_scope_triggered = false;
    g_scope_capturing = false;
    captureFinished = false;

    float clockDiv = (float)clock_get_hz(clk_sys) / (float)freq;
    capturePIO = pio1;
    pio_clear_instruction_memory(capturePIO);

    sm_Capture = pio_claim_unused_sm(capturePIO, true);
    pio_sm_clear_fifos(capturePIO, sm_Capture);
    pio_sm_restart(capturePIO, sm_Capture);

    captureOffset = pio_add_program(capturePIO, &NO_TRIGGER_CAPTURE_program);

    for (int i = 0; i < PIN_MAP_SIZE; i++) {
        gpio_disable_pulls(pinMap[i]);
        pio_sm_set_consecutive_pindirs(capturePIO, sm_Capture, pinMap[i], 1, false);
        pio_gpio_init(capturePIO, pinMap[i]);
    }

    enum dma_channel_transfer_size transferSize = (captureMode == MODE_16_CHANNEL) ? DMA_SIZE_16 : DMA_SIZE_8;
    pio_sm_config smConfig = NO_TRIGGER_CAPTURE_program_get_default_config(captureOffset);
    sm_config_set_in_pins(&smConfig, INPUT_PIN_BASE);
    sm_config_set_clkdiv(&smConfig, clockDiv);
    sm_config_set_in_shift(&smConfig, true, true, (captureMode == MODE_16_CHANNEL) ? 16 : 8);

    pio_sm_set_enabled(capturePIO, sm_Capture, false);
    pio_sm_init(capturePIO, sm_Capture, captureOffset, &smConfig);

    dmaPingPong0 = dma_claim_unused_channel(true);
    dma_channel_config cfg0 = dma_channel_get_default_config(dmaPingPong0);
    channel_config_set_read_increment(&cfg0, false);
    channel_config_set_write_increment(&cfg0, true);
    channel_config_set_transfer_data_size(&cfg0, transferSize);
    channel_config_set_ring(&cfg0, true, 16);
    channel_config_set_dreq(&cfg0, pio_get_dreq(capturePIO, sm_Capture, false));
    channel_config_set_enable(&cfg0, true);

    dmaPingPong1 = dma_claim_unused_channel(true);
    dma_channel_config cfg1 = dma_channel_get_default_config(dmaPingPong1);
    channel_config_set_read_increment(&cfg1, false);
    channel_config_set_write_increment(&cfg1, true);
    channel_config_set_transfer_data_size(&cfg1, transferSize);
    channel_config_set_dreq(&cfg1, pio_get_dreq(capturePIO, sm_Capture, false));
    channel_config_set_enable(&cfg1, true);
    dma_channel_set_irq0_enabled(dmaPingPong1, true);

    irq_set_exclusive_handler(DMA_IRQ_0, scope_dma_post_completed);
    irq_set_enabled(DMA_IRQ_0, true);
    irq_set_priority(DMA_IRQ_0, 0);

    uint32_t postTransfers = (captureMode == MODE_16_CHANNEL) ? (g_scope_post_bytes / 2) : g_scope_post_bytes;
    dma_channel_configure(dmaPingPong1, &cfg1, g_scope_post_buf_ptr, &capturePIO->rxf[sm_Capture], postTransfers, false);
    dma_channel_configure(dmaPingPong0, &cfg0, g_scope_pre_ring_ptr, &capturePIO->rxf[sm_Capture], 0xFFFFFFFF, true);

    sm_Trigger = pio_claim_unused_sm(capturePIO, true);
    triggerOffset = pio_add_program(capturePIO, &pio_trigger_qualifier_program);
    pio_sm_config trigConfig = pio_trigger_qualifier_program_get_default_config(triggerOffset);
    sm_config_set_in_pins(&trigConfig, INPUT_PIN_BASE);
    sm_config_set_clkdiv(&trigConfig, clockDiv);

    pio_sm_set_enabled(capturePIO, sm_Trigger, false);
    pio_sm_init(capturePIO, sm_Trigger, triggerOffset, &trigConfig);

    uint32_t matchPattern = (g_trigger_config.match & 0xFFFF);
    pio_sm_put_blocking(capturePIO, sm_Trigger, matchPattern);

    pio_set_irq0_source_enabled(capturePIO, pis_interrupt1, true);
    irq_set_exclusive_handler(PIO1_IRQ_0, pio_trigger_irq_handler);
    irq_set_enabled(PIO1_IRQ_0, true);

    pio_sm_set_enabled(capturePIO, sm_Capture, true);
    pio_sm_set_enabled(capturePIO, sm_Trigger, true);

    g_scope_capturing = true;
    lastCaptureType = CAPTURE_TYPE_SCOPE;
    return true;
}

uint8_t* capture_get_scope_unified_buffer(uint32_t *total_bytes, uint32_t *pre_bytes, uint32_t *post_bytes) {
    if (!g_scope_pre_ring_ptr) g_scope_pre_ring_ptr = captureBuffer;
    if (!g_scope_post_buf_ptr) g_scope_post_buf_ptr = captureBuffer + SCOPE_PRE_SIZE;

    if (pre_bytes) *pre_bytes = g_scope_pre_bytes;
    if (post_bytes) *post_bytes = g_scope_post_bytes;
    if (total_bytes) *total_bytes = g_scope_pre_bytes + g_scope_post_bytes;

    return g_scope_pre_ring_ptr;
}


#include "agentla_board_settings.h"
#include "agentla_capture.h"
#include "hardware/gpio.h"
#include "hardware/dma.h"
#include "hardware/irq.h"
#include "hardware/clocks.h"
#include "string.h"
#include "hardware/sync.h"
#include "hardware/structs/syscfg.h"
#include "hardware/structs/systick.h"
#include "hardware/structs/bus_ctrl.h"
#include "agentla_capture.pio.h"

static PIO capturePIO;
static PIO triggerPIO;
static uint sm_Capture;
static uint captureOffset;
static uint sm_Trigger;
static uint triggerOffset;

static uint32_t dmaPingPong0;
static uint32_t dmaPingPong1;
static uint32_t transferCount;

static uint8_t lastCapturePins[24];
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

const uint8_t pinMap[] = {2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,26,27,28,COMPLEX_TRIGGER_IN_PIN};
#define PIN_MAP_SIZE (sizeof(pinMap) / sizeof(pinMap[0]))

uint8_t gpio_to_pindex(uint8_t gpio) {
    for(uint8_t i = 0; i < PIN_MAP_SIZE; i++) {
        if(pinMap[i] == gpio) return i;
    }
    return 0xFF;
}

static uint8_t captureBuffer[CAPTURE_BUFFER_SIZE] __attribute__((aligned(4)));

#define CAPTURE_TYPE_SIMPLE 0
#define CAPTURE_TYPE_COMPLEX 1
#define CAPTURE_TYPE_FAST 2
#define CAPTURE_TYPE_BLAST 3
#define CAPTURE_TYPE_NO_TRIGGER 4

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
        case MODE_24_CHANNEL: transferCountLocal = CAPTURE_BUFFER_SIZE / 4; break;
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
    irq_set_enabled(PIO0_IRQ_0, false);
    irq_remove_handler(PIO0_IRQ_0, complex_capture_completed);
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
    irq_set_enabled(PIO0_IRQ_0, false);
    irq_remove_handler(PIO0_IRQ_0, simple_capture_completed);
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
    irq_set_enabled(PIO0_IRQ_0, false);
    irq_remove_handler(PIO0_IRQ_0, no_trigger_capture_completed);
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
        case MODE_24_CHANNEL: transferSize = DMA_SIZE_32; break;
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
        case MODE_24_CHANNEL: transferSize = DMA_SIZE_32; transferCount = CAPTURE_BUFFER_SIZE / 4; break;
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
        restore_interrupts(int_status);
    }
}

bool StartCaptureBlast(uint32_t freq, uint32_t length, const uint8_t* capturePins, uint8_t capturePinCount, uint8_t triggerPin, bool invertTrigger, CHANNEL_MODE captureMode) {
    int maxSamples;
    switch(captureMode) {
        case MODE_8_CHANNEL: maxSamples = CAPTURE_BUFFER_SIZE; break;
        case MODE_16_CHANNEL: maxSamples = CAPTURE_BUFFER_SIZE / 2; break;
        case MODE_24_CHANNEL: maxSamples = CAPTURE_BUFFER_SIZE / 4; break;
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
    capturePIO = pio0;
    pio_clear_instruction_memory(capturePIO);
    sm_Capture = pio_claim_unused_sm(capturePIO, true);
    pio_sm_clear_fifos(capturePIO, sm_Capture);
    pio_sm_restart(capturePIO, sm_Capture);
    captureOffset = pio_add_program(capturePIO, &BLAST_CAPTURE_program);
    for(int i = 0; i < 24; i++) {
        gpio_disable_pulls(pinMap[i]);
        pio_sm_set_consecutive_pindirs(capturePIO, sm_Capture, pinMap[i], 1, false);
    }
    for(uint8_t i = 0; i < 24; i++)
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
        case MODE_24_CHANNEL: maxSamples = CAPTURE_BUFFER_SIZE / 4; break;
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
    capturePIO = pio0;
    pio_clear_instruction_memory(capturePIO);
    sm_Capture = pio_claim_unused_sm(capturePIO, true);
    pio_sm_clear_fifos(capturePIO, sm_Capture);
    pio_sm_restart(capturePIO, sm_Capture);
    if(invertTrigger)
        captureOffset = pio_add_program(capturePIO, &NEGATIVE_CAPTURE_program);
    else
        captureOffset = pio_add_program(capturePIO, &POSITIVE_CAPTURE_program);
    for(int i = 0; i < 24; i++) {
        gpio_disable_pulls(pinMap[i]);
        pio_sm_set_consecutive_pindirs(capturePIO, sm_Capture, pinMap[i], 1, false);
    }
    for(uint8_t i = 0; i < 24; i++)
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
    irq_set_exclusive_handler(PIO0_IRQ_0, simple_capture_completed);
    irq_set_enabled(PIO0_IRQ_0, true);
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
        case MODE_24_CHANNEL: maxSamples = CAPTURE_BUFFER_SIZE / 4; break;
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
    capturePIO = pio0;
    pio_clear_instruction_memory(capturePIO);
    sm_Capture = pio_claim_unused_sm(capturePIO, true);
    pio_sm_clear_fifos(capturePIO, sm_Capture);
    pio_sm_restart(capturePIO, sm_Capture);
    captureOffset = pio_add_program(capturePIO, &NO_TRIGGER_CAPTURE_program);
    for(int i = 0; i < 24; i++) {
        gpio_disable_pulls(pinMap[i]);
        pio_sm_set_consecutive_pindirs(capturePIO, sm_Capture, pinMap[i], 1, false);
    }
    for(uint8_t i = 0; i < 24; i++)
        pio_gpio_init(capturePIO, pinMap[i]);
    pio_sm_config smConfig = NO_TRIGGER_CAPTURE_program_get_default_config(captureOffset);
    sm_config_set_in_pins(&smConfig, INPUT_PIN_BASE);
    sm_config_set_clkdiv(&smConfig, clockDiv);
    sm_config_set_in_shift(&smConfig, true, true, 0);
    pio_interrupt_clear(capturePIO, 0);
    pio_set_irq0_source_enabled(capturePIO, pis_interrupt0, true);
    irq_set_exclusive_handler(PIO0_IRQ_0, no_trigger_capture_completed);
    irq_set_enabled(PIO0_IRQ_0, true);
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
        case MODE_24_CHANNEL: maxSamples = CAPTURE_BUFFER_SIZE / 4; break;
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
    capturePIO = pio0;
    pio_clear_instruction_memory(capturePIO);
    sm_Capture = pio_claim_unused_sm(capturePIO, true);
    pio_sm_clear_fifos(capturePIO, sm_Capture);
    pio_sm_restart(capturePIO, sm_Capture);
    captureOffset = pio_add_program(capturePIO, &NO_TRIGGER_CAPTURE_program);
    for(int i = 0; i < 24; i++) {
        if(pinMap[i] == skipPin) continue;
        gpio_disable_pulls(pinMap[i]);
        pio_sm_set_consecutive_pindirs(capturePIO, sm_Capture, pinMap[i], 1, false);
    }
    for(uint8_t i = 0; i < 24; i++) {
        if(pinMap[i] == skipPin) continue;
        pio_gpio_init(capturePIO, pinMap[i]);
    }
    pio_sm_config smConfig = NO_TRIGGER_CAPTURE_program_get_default_config(captureOffset);
    sm_config_set_in_pins(&smConfig, INPUT_PIN_BASE);
    sm_config_set_clkdiv(&smConfig, clockDiv);
    sm_config_set_in_shift(&smConfig, true, true, 0);
    pio_interrupt_clear(capturePIO, 0);
    pio_set_irq0_source_enabled(capturePIO, pis_interrupt0, true);
    irq_set_exclusive_handler(PIO0_IRQ_0, no_trigger_capture_completed);
    irq_set_enabled(PIO0_IRQ_0, true);
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
        case MODE_24_CHANNEL: maxSamples = CAPTURE_BUFFER_SIZE / 4; break;
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
    capturePIO = pio0;
    pio_clear_instruction_memory(capturePIO);
    gpio_disable_pulls(COMPLEX_TRIGGER_OUT_PIN);
    gpio_disable_pulls(COMPLEX_TRIGGER_IN_PIN);
    pio_gpio_init(capturePIO, COMPLEX_TRIGGER_OUT_PIN);
    pio_gpio_init(capturePIO, COMPLEX_TRIGGER_IN_PIN);
    for(uint8_t i = 0; i < 24; i++) {
        gpio_disable_pulls(pinMap[i]);
        pio_gpio_init(capturePIO, pinMap[i]);
    }
    sm_Capture = pio_claim_unused_sm(capturePIO, true);
    pio_sm_clear_fifos(capturePIO, sm_Capture);
    pio_sm_restart(capturePIO, sm_Capture);
    captureOffset = pio_add_program(capturePIO, &COMPLEX_CAPTURE_program);
    for(int i = 0; i < 24; i++)
        pio_sm_set_consecutive_pindirs(capturePIO, sm_Capture, pinMap[i], 1, false);
    pio_sm_config smConfig = COMPLEX_CAPTURE_program_get_default_config(captureOffset);
    sm_config_set_in_pins(&smConfig, INPUT_PIN_BASE);
    sm_config_set_clkdiv(&smConfig, clockDiv);
    sm_config_set_in_shift(&smConfig, false, true, 29);
    sm_config_set_jmp_pin(&smConfig, COMPLEX_TRIGGER_IN_PIN);
    pio_interrupt_clear(capturePIO, 0);
    pio_set_irq0_source_enabled(capturePIO, pis_interrupt0, true);
    irq_set_exclusive_handler(PIO0_IRQ_0, complex_capture_completed);
    irq_set_enabled(PIO0_IRQ_0, true);
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
        case MODE_24_CHANNEL: maxSamples = CAPTURE_BUFFER_SIZE / 4; break;
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
    triggerPIO = pio0;
    pio_clear_instruction_memory(triggerPIO);
    gpio_disable_pulls(COMPLEX_TRIGGER_OUT_PIN);
    gpio_disable_pulls(COMPLEX_TRIGGER_IN_PIN);
    pio_gpio_init(triggerPIO, COMPLEX_TRIGGER_OUT_PIN);
    pio_gpio_init(capturePIO, COMPLEX_TRIGGER_IN_PIN);
    for(uint8_t i = 0; i < 24; i++) {
        gpio_disable_pulls(pinMap[i]);
        pio_gpio_init(capturePIO, pinMap[i]);
    }
    sm_Capture = pio_claim_unused_sm(capturePIO, true);
    pio_sm_clear_fifos(capturePIO, sm_Capture);
    pio_sm_restart(capturePIO, sm_Capture);
    captureOffset = pio_add_program(capturePIO, &FAST_CAPTURE_program);
    for(int i = 0; i < 24; i++)
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
            case MODE_24_CHANNEL: maxSize = CAPTURE_BUFFER_SIZE / 4; break;
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
            case MODE_24_CHANNEL: {
                uint32_t* buffer = (uint32_t*)captureBuffer;
                for(uint32_t buc = 0; buc < totalSamples; buc++) {
                    uint32_t oldValue = buffer[currentPos];
                    uint32_t newValue = 0;
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

static uint16_t adcBuffer[ADC_BUF_SIZE];
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

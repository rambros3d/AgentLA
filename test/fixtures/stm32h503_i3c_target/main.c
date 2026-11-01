#include <stdint.h>
#include <stdbool.h>

#define RCC_BASE        0x44020C00
#define RCC_AHB2ENR     (*(volatile uint32_t *)(RCC_BASE + 0x8C))
#define RCC_APB1LENR    (*(volatile uint32_t *)(RCC_BASE + 0x9C))

#define GPIOA_BASE      0x42020000
#define GPIOA_MODER     (*(volatile uint32_t *)(GPIOA_BASE + 0x00))
#define GPIOA_ODR       (*(volatile uint32_t *)(GPIOA_BASE + 0x14))

#define GPIOB_BASE      0x42020400
#define GPIOB_MODER     (*(volatile uint32_t *)(GPIOB_BASE + 0x00))
#define GPIOB_OTYPER    (*(volatile uint32_t *)(GPIOB_BASE + 0x04))
#define GPIOB_OSPEEDR   (*(volatile uint32_t *)(GPIOB_BASE + 0x08))
#define GPIOB_PUPDR     (*(volatile uint32_t *)(GPIOB_BASE + 0x0C))
#define GPIOB_AFRL      (*(volatile uint32_t *)(GPIOB_BASE + 0x20))

#define I3C1_BASE       0x40003000
#define I3C1_CR         (*(volatile uint32_t *)(I3C1_BASE + 0x00))
#define I3C1_CFGR       (*(volatile uint32_t *)(I3C1_BASE + 0x04))
#define I3C1_RAFR       (*(volatile uint32_t *)(I3C1_BASE + 0x08))
#define I3C1_RDR        (*(volatile uint32_t *)(I3C1_BASE + 0x10))
#define I3C1_TDR        (*(volatile uint32_t *)(I3C1_BASE + 0x14))
#define I3C1_IBIDR      (*(volatile uint32_t *)(I3C1_BASE + 0x18))
#define I3C1_TGTR       (*(volatile uint32_t *)(I3C1_BASE + 0x1C))
#define I3C1_SR         (*(volatile uint32_t *)(I3C1_BASE + 0x20))
#define I3C1_SER        (*(volatile uint32_t *)(I3C1_BASE + 0x24))
#define I3C1_EVR        (*(volatile uint32_t *)(I3C1_BASE + 0x28))
#define I3C1_CEVR       (*(volatile uint32_t *)(I3C1_BASE + 0x2C))
#define I3C1_PIDR0      (*(volatile uint32_t *)(I3C1_BASE + 0x50))
#define I3C1_PIDR1      (*(volatile uint32_t *)(I3C1_BASE + 0x54))
#define I3C1_BCR        (*(volatile uint32_t *)(I3C1_BASE + 0x58))
#define I3C1_DCR        (*(volatile uint32_t *)(I3C1_BASE + 0x5C))

static uint8_t target_regs[32];
static volatile uint8_t reg_index = 0;


int main(void) {
    /* Enable GPIOA and GPIOB clocks */
    RCC_AHB2ENR |= (1 << 0) | (1 << 1);
    /* Enable I3C1 clock */
    RCC_APB1LENR |= (1 << 23);

    /* PA0 as output (LED) */
    GPIOA_MODER &= ~(3 << 0);
    GPIOA_MODER |= (1 << 0);
    GPIOA_ODR |= (1 << 0);

    /* PB6 (I3C1_SCL) and PB7 (I3C1_SDA) -> AF3, Open-drain, Very high speed */
    GPIOB_MODER &= ~((3 << 12) | (3 << 14));
    GPIOB_MODER |= ((2 << 12) | (2 << 14)); /* Alternate function */
    GPIOB_OTYPER |= ((1 << 6) | (1 << 7));   /* Open drain */
    GPIOB_OSPEEDR |= ((3 << 12) | (3 << 14));/* Very high speed */
    GPIOB_PUPDR |= ((1 << 12) | (1 << 14));  /* Pull-up */

    GPIOB_AFRL &= ~((0xFu << 24) | (0xFu << 28));
    GPIOB_AFRL |= ((3u << 24) | (3u << 28));  /* AF3 = I3C1 */

    /* Initialize test registers with default pattern */
    for (uint8_t i = 0; i < 32; i++) {
        target_regs[i] = 0x10 + i;
    }

    /* Configure I3C1 Target Mode */
    I3C1_CR = 0; /* Disable while configuring */

    /* Set Provisional ID: ST Vendor ID (0x0104) and H503 target identifier */
    I3C1_PIDR0 = 0x01040503;
    I3C1_PIDR1 = 0x00000001;

    /* BCR: Target role, supports dynamic address assignment, IBI payload */
    I3C1_BCR = 0x06;
    /* DCR: Generic sensor / bring-up test target */
    I3C1_DCR = 0x63;

    /* Enable I3C Target */
    I3C1_CFGR = 0; /* Target mode */
    I3C1_CR = (1 << 0); /* Peripheral Enable */

    uint32_t loop_counter = 0;

    while (1) {
        loop_counter++;
        if ((loop_counter & 0x7FFFF) == 0) {
            GPIOA_ODR ^= (1 << 0); /* Heartbeat toggle */
        }

        uint32_t evr = I3C1_EVR;

        /* Receive FIFO not empty (Controller writing to target) */
        if (evr & (1 << 5)) { /* RXNE */
            uint32_t rx_data = I3C1_RDR;
            uint8_t byte_val = (uint8_t)(rx_data & 0xFF);
            if (reg_index < 32) {
                target_regs[reg_index++] = byte_val;
            }
            I3C1_CEVR = (1 << 5); /* Clear RXNE */

            /* Check for IBI trigger request */
            if (target_regs[0] == 0xBB) { /* Magic byte to request IBI */
                I3C1_IBIDR = 0xA5; /* Mandatory Data Byte (MDB) */
                I3C1_CR |= (1 << 2); /* Request IBI */
                target_regs[0] = 0x00; /* Clear trigger */
            }
        }

        /* Transmit FIFO empty (Controller reading from target) */
        if (evr & (1 << 4)) { /* TXFNF */
            uint8_t tx_val = target_regs[reg_index < 32 ? reg_index++ : 0];
            I3C1_TDR = tx_val;
            I3C1_CEVR = (1 << 4);
        }

        /* STOP condition detected -> reset reg_index pointer */
        if (evr & (1 << 9)) { /* STOPF */
            reg_index = 0;
            I3C1_CEVR = (1 << 9);
        }

        /* Dynamic Address Updated */
        if (evr & (1 << 1)) { /* DAF */
            I3C1_CEVR = (1 << 1);
        }
    }
}

/**
 * BluePill STM32F103 Blink — Arduino Framework
 *
 * Flashes PC13 at ~1 Hz (1 second period).
 * BluePill LED is active LOW (ON when pin = 0).
 *
 * Runs at 72 MHz (PLL from 8 MHz HSI).
 */

#include <Arduino.h>

// BluePill LED pin — directly on the PC13 pin (Port C, Pin 13)
// Arduino pin PC13 = digital pin 33 on generic STM32F103
#ifndef LED_BUILTIN
#define LED_BUILTIN PC13
#endif

void setup() {
    pinMode(LED_BUILTIN, OUTPUT);
}

void loop() {
    // Pulse 1 (Systole)
    digitalWrite(LED_BUILTIN, LOW);   // LED ON (active low)
    delay(100);
    digitalWrite(LED_BUILTIN, HIGH);  // LED OFF
    delay(100);

    // Pulse 2 (Diastole)
    digitalWrite(LED_BUILTIN, LOW);   // LED ON (active low)
    delay(100);
    digitalWrite(LED_BUILTIN, HIGH);  // LED OFF
    delay(700);  // Rest period before next heartbeat
}

#include <Arduino.h>

#ifndef LED_BUILTIN
#define LED_BUILTIN PC13
#endif

static uint32_t counter = 0;
static uint32_t last_print = 0;

void setup() {
    pinMode(LED_BUILTIN, OUTPUT);
    digitalWrite(LED_BUILTIN, HIGH); // Off

    // Force USB re-enumeration on STM32F103 (pull D+ / PA12 low then release)
    pinMode(PA12, OUTPUT);
    digitalWrite(PA12, LOW);
    delay(100);
    pinMode(PA12, INPUT);
    delay(100);

    Serial.begin(115200);
}

void loop() {
    uint32_t now = millis();

    // Heartbeat status message every 1 second over USB serial
    if (now - last_print >= 1000) {
        last_print = now;
        counter++;
        digitalWrite(LED_BUILTIN, !digitalRead(LED_BUILTIN)); // Toggle LED

        if (Serial) {
            Serial.print("HELLO FROM BLUEPILL USB CDC! Count: ");
            Serial.println(counter);
        }
    }

    // Echo any incoming characters from host USB
    while (Serial && Serial.available() > 0) {
        char c = Serial.read();
        Serial.print("ECHO: ");
        Serial.write(c);
        Serial.println();
    }
}

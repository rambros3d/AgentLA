## ADDED Requirements

### Requirement: Microsecond Hardware PWM Resolution
The firmware SHALL configure RP2350 PWM slices to advance the counter at a $1.0\ \mu\text{s}$ tick rate, providing exact microsecond pulse width control across all flexible GPIO pins (GP10–GP29).

#### Scenario: Setting 50Hz frame rate with microsecond pulse
- **WHEN** a client configures PWM on a flexible pin with frequency 50Hz and pulse width $1500\ \mu\text{s}$
- **THEN** the firmware calculates divider $\text{div} = f_{\text{sys}} / 1,000,000$, wrap $= 19,999$, and compare level $= 1500$, generating a $20\text{ ms}$ period with an active-high pulse of $1500\ \mu\text{s}$.

### Requirement: Pin Ownership and Safety Quiescing
PWM channels SHALL claim their designated GPIO pin through the dynamic pin ownership manager and immediately return the pin to high-impedance safe state upon release or emergency stop.

#### Scenario: Claiming pin for PWM
- **WHEN** a pin is claimed for PWM signal generation
- **THEN** the pin ownership manager assigns ownership and sets pin function to `GPIO_FUNC_PWM`.

#### Scenario: Safe state deactivation
- **WHEN** an emergency safe state is triggered or the channel is released
- **THEN** the PWM slice is disabled and the GPIO pin is reset to `GPIO_FUNC_SIO` input high-Z.

## 1. Capability Model

- [x] 1.1 Record RP2350 pin function tables for GPIO10 through GPIO29 in a compact claim-capability data structure.
- [x] 1.2 Mark fixed programmer GPIO0 through GPIO9 as permanently reserved in the capability model.
- [x] 1.3 Record analog-capable flexible pins (GPIO26 through GPIO29) as a distinct capability set.
- [x] 1.4 Document unsupported or package-specific flexible pin limitations.

## 2. Ownership Registry

- [x] 2.1 Define a registry state for resource kinds (physical pin, peripheral instance, stream buffer, capture bank).
- [x] 2.2 Implement claim-handle generation and validation.
- [x] 2.3 Implement atomic all-or-nothing claim acquisition.
- [x] 2.4 Implement conflict reporting for pins, instances, and capture bank claims.
- [x] 2.5 Implement explicit release and release status reporting.
- [x] 2.6 Implement quiesce callbacks for adapter, capture, and scope resources.

## 3. Programmer Boundary

- [x] 3.1 Add compile-time assertions that GPIO0 through GPIO9 cannot be claimed by flexible roles.
- [x] 3.2 Audit Programmer code paths for unsafe fallback into flexible pin roles.
- [x] 3.3 Add Programmer regression checks before enabling optional claim modes.

## 4. Adapter Migration

- [x] 4.1 Add claim requirements to Adapter I2C startup and shutdown.
- [x] 4.2 Add claim requirements to Adapter UART startup and shutdown.
- [x] 4.3 Add claim requirements to Adapter SPI startup and shutdown.
- [x] 4.4 Add claim requirements to CAN bit-bang initialization and shutdown.
- [x] 4.5 Retain Programmer I2C sharing boundaries while moving Adapter claims to flexible pins only.
- [x] 4.6 Update Adapter commands to require and report active claim handles.

## 5. Capture System Migration

- [x] 5.1 Add span-based capture claim structures and pin-span reservation.
- [x] 5.2 Add active-bank uniqueness enforcement.
- [x] 5.3 Add capture metadata for selected pins, selected-bit mask, span width, and trigger index.
- [x] 5.4 Implement capture initialization only from claimed pins and metadata.
- [x] 5.5 Implement capture release and automatic quiesce.
- [x] 5.7 Enforce the initial 8-pin capture width limit and report safe maximum span.
- [ ] 5.8 Test smaller and larger banks against the documented width bounds.

## 6. ADC and Scope Migration

- [x] 6.1 Validate ADC claims against analog-capable pins.
- [x] 6.2 Attach scope/analog capabilities to capture or independent digital claims.
- [x] 6.3 Add cutoff behavior when an ADC-required digital claim is unavailable.
- [x] 6.4 Report analog capability separately from digital claim success.

## 7. Command and SDK API

- [x] 7.1 Introduce claim and release commands or SDK calls.
- [x] 7.2 Update adapter/capture/scoped commands to require handles.
- [x] 7.3 Define stable error codes for each ownership failure category.
- [x] 7.4 Ensure failed claim transactions leave no partial pin assignments.
- [x] 7.5 Update SDK helpers to create convenience bundles while retaining handle transparency.

## 8. Test Compatibility

- [ ] 8.1 Update Adapter tests to claim flexible pins explicitly instead of assuming defaults.
- [ ] 8.2 Update Capture tests to declare bank spans, masks, and locations.
- [ ] 8.3 Add conflict tests for overlapping capture spans and Adapter claims.
- [ ] 8.4 Add ADC validity tests for analog-enabled and analog-disabled pins.
- [ ] 8.5 Verify Programmer regression tests without ownership state active.

## 9. Documentation

- [x] 9.1 Update `docs/pin-allocation.md` to describe fixed and flexible pools.
- [x] 9.2 Document claim handle lifecycle and error responses.
- [x] 9.3 Document capture span metadata and one-bank-at-a-time behavior.
- [x] 9.4 Remove historical fixed assumptions from examples and migration notes.

## 11. Shared Observation and Capability Inventory

- [x] 11.1 Add read-only observer claims to the ownership registry without duplicating driver ownership.
- [x] 11.2 Add runtime pin and analog capability inventory reporting.
- [x] 11.3 Add explicit safe read-only discovery mode.
- [x] 11.4 Add SDK transparent bundles that expand only into explicit handles.
- [ ] 11.5 Test observer roles for input pins and verify output conflicts.
- [ ] 11.6 Document shared observation and discovery constraints.
- [x] 11.7 Implement idempotent release handling.
- [x] 11.8 Implement all-or-nothing transparent bundles.
- [x] 11.9 Implement report-only capability inventory and discovery.
- [ ] 11.10 Implement interrupt and edge callback lifecycle resources.
- [x] 11.11 Implement initial 8-pin capture span limit and later performance-capacity hook.

- [x] 10.1 Build firmware with ownership subsystem enabled.
- [ ] 10.2 Run Programmer regression suite.
- [ ] 10.3 Run Adapter and Capture ownership tests.
- [ ] 10.4 Run host SDK regression tests.
- [x] 10.5 Validate that Programmer pins cannot be claimed through flexible commands.
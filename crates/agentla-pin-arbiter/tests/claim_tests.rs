use agentla_pin_arbiter::{
    ClaimError, Driver, PinArbiter, PinQuiesce, SlotId, CAP_ADC, CAP_SIO, FIXED_PINS, FLEX_PINS,
    MAX_CLAIMS,
};

#[derive(Default)]
struct MockQuiesce {
    quiesced_pins: Vec<u32>,
}

impl PinQuiesce for MockQuiesce {
    fn quiesce_pin(&mut self, pin: u32) {
        self.quiesced_pins.push(pin);
    }
}

#[test]
fn fixed_and_out_of_pool_pins_rejected() {
    let mut arbiter = PinArbiter::new();

    // Fixed pins 4..9 rejected for normal claim
    for &pin in &FIXED_PINS {
        let err = arbiter.claim(Driver::Servo, &[pin], CAP_SIO).unwrap_err();
        assert_eq!(err, ClaimError::FixedPin);
    }

    // Out of pool pins (25 = LED, 30, 31 = USB)
    for pin in [25u32, 30, 31] {
        let err = arbiter.claim(Driver::Servo, &[pin], 0).unwrap_err();
        assert_eq!(err, ClaimError::NotInPool);
    }

    // Ensure nothing was claimed
    for &pin in &FLEX_PINS {
        assert_eq!(arbiter.has_claim(pin), None);
    }
}

#[test]
fn claim_conflict_preserves_state() {
    let mut arbiter = PinArbiter::new();
    let slot1 = arbiter.claim(Driver::Servo, &[17], 0).unwrap();

    let err = arbiter.claim(Driver::Gpio, &[17], 0).unwrap_err();
    assert_eq!(err, ClaimError::Conflict);

    assert_eq!(arbiter.has_claim(17), Some(slot1));
    assert_eq!(arbiter.owner_of(17), Some(Driver::Servo));
}

#[test]
fn atomic_all_or_nothing_contested_claim() {
    // Scenario from spec:
    // WHEN a driver attempts to claim pins [12, 13, 14] but pin 14 is already claimed
    // THEN the claim returns Err(ClaimError::Conflict) and pins 12 and 13 remain unassigned.
    let mut arbiter = PinArbiter::new();
    let initial_slot = arbiter.claim(Driver::Servo, &[14], 0).unwrap();

    let err = arbiter
        .claim(Driver::Adapter, &[12, 13, 14], 0)
        .unwrap_err();
    assert_eq!(err, ClaimError::Conflict);

    // Pins 12 and 13 MUST remain unassigned
    assert_eq!(arbiter.has_claim(12), None);
    assert_eq!(arbiter.has_claim(13), None);
    assert_eq!(arbiter.owner_of(12), None);
    assert_eq!(arbiter.owner_of(13), None);

    // Pin 14 still belongs to initial claim
    assert_eq!(arbiter.has_claim(14), Some(initial_slot));
    assert_eq!(arbiter.owner_of(14), Some(Driver::Servo));
}

#[test]
fn atomic_all_or_nothing_capability_missing() {
    let mut arbiter = PinArbiter::new();
    // Pin 17 lacks ADC; Pin 27 has ADC.
    let err = arbiter
        .claim(Driver::Servo, &[17, 27], CAP_ADC)
        .unwrap_err();
    assert_eq!(err, ClaimError::CapabilityMissing);

    // Neither pin 17 nor 27 should be claimed
    assert_eq!(arbiter.has_claim(17), None);
    assert_eq!(arbiter.has_claim(27), None);

    // Pin 27 alone with CAP_ADC succeeds
    let slot = arbiter.claim(Driver::Servo, &[27], CAP_ADC).unwrap();
    assert_eq!(arbiter.has_claim(27), Some(slot));
}

#[test]
fn quiesce_notification_on_release() {
    // Scenario from spec:
    // WHEN release(slot) is invoked
    // THEN the slot is returned to the free pool and the registered PinQuiesce callback
    // is invoked for each released pin.
    let mock = MockQuiesce::default();
    let mut arbiter = PinArbiter::with_quiesce(mock);

    let slot = arbiter.claim(Driver::Servo, &[16, 17], 0).unwrap();
    assert_eq!(arbiter.has_claim(16), Some(slot));
    assert_eq!(arbiter.has_claim(17), Some(slot));

    assert!(arbiter.release(slot));

    // Callback invoked for each released pin
    assert_eq!(arbiter.quiesce().quiesced_pins, vec![16, 17]);

    // Slot is returned to the free pool
    assert_eq!(arbiter.has_claim(16), None);
    assert_eq!(arbiter.has_claim(17), None);
    assert_eq!(arbiter.active_claims(), 0);

    // Re-releasing is idempotent no-op
    assert!(!arbiter.release(slot));
    assert_eq!(arbiter.quiesce().quiesced_pins.len(), 2);
}

#[test]
fn registry_capacity_and_handle_progression() {
    let mut arbiter = PinArbiter::new();
    let flex_pins: [u32; 16] = [0, 1, 2, 3, 11, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 26];

    let mut last_handle = SlotId(0);
    for &pin in &flex_pins {
        let slot = arbiter.claim(Driver::Servo, &[pin], 0).unwrap();
        assert!(slot.0 > last_handle.0);
        last_handle = slot;
    }

    assert_eq!(arbiter.active_claims(), MAX_CLAIMS);

    // 17th claim fails because registry is full
    let err = arbiter.claim(Driver::Gpio, &[27], 0).unwrap_err();
    assert_eq!(err, ClaimError::NoFreeSlots);

    // Releasing one claim frees a slot
    assert!(arbiter.release(SlotId(1)));
    assert_eq!(arbiter.active_claims(), MAX_CLAIMS - 1);

    // Now claim succeeds
    let new_slot = arbiter.claim(Driver::Gpio, &[27], 0).unwrap();
    assert!(new_slot.0 > last_handle.0);
}

#[test]
fn clear_transient_preserves_keep_driver() {
    let mock = MockQuiesce::default();
    let mut arbiter = PinArbiter::with_quiesce(mock);

    let bridge = [4u32, 5, 6];
    let prog_slot = arbiter.claim_fixed(Driver::Programmer, &bridge, 0).unwrap();
    let _servo_slot = arbiter.claim(Driver::Servo, &[16, 17], 0).unwrap();
    let _gpio_slot = arbiter.claim(Driver::Gpio, &[18], 0).unwrap();

    assert_eq!(arbiter.active_claims(), 3);

    let released_count = arbiter.clear_transient(Driver::Programmer);
    assert_eq!(released_count, 2);
    assert_eq!(arbiter.active_claims(), 1);

    // Programmer claim still alive
    assert_eq!(arbiter.has_claim(4), Some(prog_slot));
    assert_eq!(arbiter.owner_of(4), Some(Driver::Programmer));

    // Servo and Gpio claims cleared and quiesced
    assert_eq!(arbiter.has_claim(16), None);
    assert_eq!(arbiter.has_claim(18), None);
    assert_eq!(arbiter.quiesce().quiesced_pins, vec![16, 17, 18]);

    // Programmer slot can be released
    assert!(arbiter.release(prog_slot));
    assert_eq!(arbiter.active_claims(), 0);
}

#[test]
fn handle_pins_and_driver_queries() {
    let mut arbiter = PinArbiter::new();
    let pins = [0u32, 1, 2];
    let slot = arbiter.claim(Driver::Capture, &pins, 0).unwrap();

    let retrieved = arbiter.handle_pins(slot).unwrap();
    assert_eq!(&retrieved[..], &pins[..]);

    assert_eq!(arbiter.handle_pins(SlotId(999)), None);
}

#[test]
fn driver_parsing_and_wire_roundtrip() {
    assert_eq!(Driver::from_name("servo"), Some(Driver::Servo));
    assert_eq!(Driver::from_name("programmer"), Some(Driver::Programmer));
    assert_eq!(Driver::from_name("scope"), Some(Driver::Oscilloscope));
    assert_eq!(Driver::from_name("unknown_driver"), None);

    for code in 0..=8 {
        let driver = Driver::from_u8(code).unwrap();
        assert_eq!(driver.enc(), code);
    }
    assert_eq!(Driver::from_u8(9), None);
}

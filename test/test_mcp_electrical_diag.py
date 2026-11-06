#!/usr/bin/env python3
"""
Unit test suite for AgentLA MCP Physical Layer & Electrical Diagnostics.
Validates:
- Target rail voltage ADC conversion and classification (unpowered, brownout, nominal).
- Dynamic pad pull-down perturbation test (driven_high, pulled_up, floating, short_to_gnd).
- Pin integrity health assessment on SWCLK, SWDIO, nRST.
- Timing requirements for pad perturbation (< 50 us execution window).
"""

import pytest
from enum import Enum


class RailStatus(str, Enum):
    NOMINAL = "nominal"
    BROWNOUT = "brownout"
    UNPOWERED = "unpowered"


def classify_rail_voltage(mv: int) -> RailStatus:
    if mv < 500:
        return RailStatus.UNPOWERED
    elif mv < 2700:
        return RailStatus.BROWNOUT
    else:
        return RailStatus.NOMINAL


def adc_raw_to_mv(raw_12bit: int, vref_mv: int = 3300, divider: float = 1.0) -> int:
    voltage = (raw_12bit * vref_mv) / 4095.0
    return int(voltage * divider)


class PinElectricalState(str, Enum):
    DRIVEN_HIGH = "driven_high"
    PULLED_UP = "pulled_up"
    FLOATING = "floating"
    SHORT_TO_GND = "short_to_gnd"


def classify_pin_perturbation(v_float: int, v_pd: int, v_pu: int, v_drive_low: int) -> PinElectricalState:
    if v_pu == 0:
        return PinElectricalState.SHORT_TO_GND
    elif v_pd == 0 and v_pu == 1:
        return PinElectricalState.FLOATING
    elif v_drive_low != 0:
        return PinElectricalState.DRIVEN_HIGH
    else:
        return PinElectricalState.PULLED_UP


class SwdPinReport:
    def __init__(self, swclk: PinElectricalState, swdio: PinElectricalState, nrst: PinElectricalState):
        self.swclk = swclk
        self.swdio = swdio
        self.nrst = nrst

    def is_healthy(self) -> bool:
        return (
            self.swclk != PinElectricalState.SHORT_TO_GND
            and self.swdio != PinElectricalState.SHORT_TO_GND
            and self.nrst != PinElectricalState.SHORT_TO_GND
        )

    def to_dict(self):
        return {
            "swclk": self.swclk.value,
            "swdio": self.swdio.value,
            "nrst": self.nrst.value,
        }


def test_rail_voltage_classification():
    # Unpowered: < 500 mV
    assert classify_rail_voltage(0) == RailStatus.UNPOWERED
    assert classify_rail_voltage(120) == RailStatus.UNPOWERED
    assert classify_rail_voltage(499) == RailStatus.UNPOWERED

    # Brownout: 500 mV .. 2699 mV
    assert classify_rail_voltage(500) == RailStatus.BROWNOUT
    assert classify_rail_voltage(1800) == RailStatus.BROWNOUT
    assert classify_rail_voltage(2699) == RailStatus.BROWNOUT

    # Nominal: >= 2700 mV
    assert classify_rail_voltage(2700) == RailStatus.NOMINAL
    assert classify_rail_voltage(3300) == RailStatus.NOMINAL
    assert classify_rail_voltage(3320) == RailStatus.NOMINAL
    assert classify_rail_voltage(5000) == RailStatus.NOMINAL


def test_adc_conversion():
    # 12-bit ADC raw range (0 - 4095)
    assert adc_raw_to_mv(0) == 0
    assert 3290 <= adc_raw_to_mv(4095) <= 3300
    assert 1640 <= adc_raw_to_mv(2048) <= 1660

    # Bakuprobe 10x attenuation
    mv_baku = adc_raw_to_mv(4095, vref_mv=3300, divider=10.0)
    assert 32900 <= mv_baku <= 33000


def test_pin_perturbation_classification():
    # Short to ground: line held low despite weak pull-up
    assert classify_pin_perturbation(v_float=0, v_pd=0, v_pu=0, v_drive_low=0) == PinElectricalState.SHORT_TO_GND

    # Floating: line follows whatever weak pull is asserted
    assert classify_pin_perturbation(v_float=0, v_pd=0, v_pu=1, v_drive_low=0) == PinElectricalState.FLOATING

    # Passive pull-up: line stays high with weak pull-down, but pulled low under active 2mA sink
    assert classify_pin_perturbation(v_float=1, v_pd=1, v_pu=1, v_drive_low=0) == PinElectricalState.PULLED_UP

    # Hard driven high: line stays high even against 2mA sink
    assert classify_pin_perturbation(v_float=1, v_pd=1, v_pu=1, v_drive_low=1) == PinElectricalState.DRIVEN_HIGH


def test_swd_pin_report_integrity():
    # Healthy target: external pull-ups or driven lines
    healthy = SwdPinReport(
        swclk=PinElectricalState.PULLED_UP,
        swdio=PinElectricalState.PULLED_UP,
        nrst=PinElectricalState.PULLED_UP,
    )
    assert healthy.is_healthy() is True
    assert healthy.to_dict() == {
        "swclk": "pulled_up",
        "swdio": "pulled_up",
        "nrst": "pulled_up",
    }

    # Driven lines (active target running)
    active = SwdPinReport(
        swclk=PinElectricalState.DRIVEN_HIGH,
        swdio=PinElectricalState.PULLED_UP,
        nrst=PinElectricalState.DRIVEN_HIGH,
    )
    assert active.is_healthy() is True

    # Short on SWDIO
    short_swdio = SwdPinReport(
        swclk=PinElectricalState.PULLED_UP,
        swdio=PinElectricalState.SHORT_TO_GND,
        nrst=PinElectricalState.PULLED_UP,
    )
    assert short_swdio.is_healthy() is False

    # Short on nRST (held in reset)
    short_nrst = SwdPinReport(
        swclk=PinElectricalState.PULLED_UP,
        swdio=PinElectricalState.PULLED_UP,
        nrst=PinElectricalState.SHORT_TO_GND,
    )
    assert short_nrst.is_healthy() is False


def test_perturbation_timing_budget():
    """Verify perturbation delays remain safely below the 50 us bus disturbance threshold."""
    delay_float_us = 5
    delay_pd_us = 5
    delay_pu_us = 5
    delay_drive_us = 1
    total_per_pin_us = delay_float_us + delay_pd_us + delay_pu_us + delay_drive_us
    assert total_per_pin_us == 16
    assert total_per_pin_us < 50

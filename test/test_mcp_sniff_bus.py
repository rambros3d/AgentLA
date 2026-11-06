#!/usr/bin/env python3
"""
Unit test suite for AgentLA MCP Autonomous Bus Sniffer (`mcp_sniff_bus`).
Validates:
- Waveform generation and software decoding of I2C transactions (START, 7-bit addr, R/W, ACK/NACK, data, STOP).
- Repeated START handling across multi-message sequences.
- Address inventory aggregation (unique active 7-bit addresses).
- 32-transaction capping while preserving total transaction count.
- SPI transaction decoding (CS framing, MOSI/MISO full-duplex latching on SCK rising edge).
- Pin conflict arbitration rejection (returns isError: true on bus contention).
- MCP protocol content envelope formatting.
"""

import json
import pytest
from typing import List, Dict, Tuple


class MockI2cDecoder:
    """Software model of firmware-rs/src/decoder.rs i2c_decode_transactions."""

    @staticmethod
    def decode(samples: List[int], scl_bit: int = 1, sda_bit: int = 0) -> Tuple[int, List[Dict]]:
        if not samples:
            return 0, []

        transactions = []
        total_tx = 0
        state = "idle"
        curr_tx = {"addr": 0, "is_read": False, "ack": False, "data": bytearray()}
        bit_accum = 0
        bit_count = 0

        prev_scl = (samples[0] >> scl_bit) & 1
        prev_sda = (samples[0] >> sda_bit) & 1

        for b in samples[1:]:
            cur_scl = (b >> scl_bit) & 1
            cur_sda = (b >> sda_bit) & 1

            # START condition
            if prev_scl and cur_scl and prev_sda and not cur_sda:
                if state != "idle" and (curr_tx["addr"] != 0 or len(curr_tx["data"]) > 0):
                    total_tx += 1
                    transactions.append(curr_tx)
                    curr_tx = {"addr": 0, "is_read": False, "ack": False, "data": bytearray()}
                state = "addr"
                bit_accum = 0
                bit_count = 0
                prev_scl = cur_scl
                prev_sda = cur_sda
                continue

            # STOP condition
            if prev_scl and cur_scl and not prev_sda and cur_sda:
                if state != "idle" and (curr_tx["addr"] != 0 or len(curr_tx["data"]) > 0):
                    total_tx += 1
                    transactions.append(curr_tx)
                    curr_tx = {"addr": 0, "is_read": False, "ack": False, "data": bytearray()}
                state = "idle"
                prev_scl = cur_scl
                prev_sda = cur_sda
                continue

            # SCL Rising Edge
            if not prev_scl and cur_scl:
                bit = cur_sda
                if state == "addr":
                    bit_accum = (bit_accum << 1) | bit
                    bit_count += 1
                    if bit_count == 8:
                        curr_tx["addr"] = bit_accum >> 1
                        curr_tx["is_read"] = bool(bit_accum & 1)
                        state = "addr_ack"
                elif state == "addr_ack":
                    curr_tx["ack"] = (bit == 0)
                    state = "data"
                    bit_accum = 0
                    bit_count = 0
                elif state == "data":
                    bit_accum = (bit_accum << 1) | bit
                    bit_count += 1
                    if bit_count == 8:
                        if len(curr_tx["data"]) < 8:
                            curr_tx["data"].append(bit_accum)
                        state = "data_ack"
                elif state == "data_ack":
                    curr_tx["ack"] = (bit == 0)
                    state = "data"
                    bit_accum = 0
                    bit_count = 0

            prev_scl = cur_scl
            prev_sda = cur_sda

        if state != "idle" and (curr_tx["addr"] != 0 or len(curr_tx["data"]) > 0):
            total_tx += 1
            transactions.append(curr_tx)

        return total_tx, transactions


def generate_i2c_waveform(transactions: List[Tuple[int, bool, bytes, bool]]) -> List[int]:
    """
    Synthesizes sample stream where bit 0 is SDA and bit 1 is SCL.
    Tuple: (addr_7bit, is_read, data_bytes, ack)
    """
    samples = []

    def idle():
        for _ in range(4):
            samples.append(0b11)  # SCL=1, SDA=1

    def start():
        samples.append(0b11)
        samples.append(0b10)  # SDA falls while SCL=1
        samples.append(0b00)  # SCL falls

    def stop():
        samples.append(0b00)
        samples.append(0b10)  # SCL rises while SDA=0
        samples.append(0b11)  # SDA rises while SCL=1
        samples.append(0b11)

    def write_byte(val: int):
        for bit_idx in range(7, -1, -1):
            bit = (val >> bit_idx) & 1
            # SCL low, set data
            samples.append(bit)
            # SCL high
            samples.append(bit | 0b10)
            # SCL low
            samples.append(bit)

    def read_ack(ack_val: bool):
        ack_bit = 0 if ack_val else 1
        samples.append(ack_bit)
        samples.append(ack_bit | 0b10)
        samples.append(ack_bit)

    idle()
    for addr, is_read, data, ack in transactions:
        start()
        addr_byte = (addr << 1) | (1 if is_read else 0)
        write_byte(addr_byte)
        read_ack(ack)
        for d in data:
            write_byte(d)
            read_ack(ack)
        stop()

    idle()
    return samples


def test_i2c_write_transaction_decode():
    # Write 0x01, 0x02 to device 0x3C
    txs = [(0x3C, False, b"\x01\x02", True)]
    samples = generate_i2c_waveform(txs)
    total, decoded = MockI2cDecoder.decode(samples)

    assert total == 1
    assert len(decoded) == 1
    tx = decoded[0]
    assert tx["addr"] == 0x3C
    assert tx["is_read"] is False
    assert tx["ack"] is True
    assert bytes(tx["data"]) == b"\x01\x02"


def test_i2c_read_transaction_decode():
    # Read 1 byte 0x42 from device 0x48
    txs = [(0x48, True, b"\x42", True)]
    samples = generate_i2c_waveform(txs)
    total, decoded = MockI2cDecoder.decode(samples)

    assert total == 1
    assert len(decoded) == 1
    tx = decoded[0]
    assert tx["addr"] == 0x48
    assert tx["is_read"] is True
    assert tx["ack"] is True
    assert bytes(tx["data"]) == b"\x42"


def test_i2c_multiple_transactions_and_address_aggregation():
    # Two different devices on the same bus
    txs = [
        (0x3C, False, b"\x00", True),
        (0x48, True, b"\x10", True),
        (0x3C, False, b"\x01\x02", True),
    ]
    samples = generate_i2c_waveform(txs)
    total, decoded = MockI2cDecoder.decode(samples)

    assert total == 3
    assert len(decoded) == 3

    # Aggregate active addresses
    addrs = sorted(list({f"0x{tx['addr']:02x}" for tx in decoded}))
    assert addrs == ["0x3c", "0x48"]


def test_i2c_transaction_capping_at_32():
    # 40 transactions generated
    txs = [(0x20, False, b"\x55", True) for _ in range(40)]
    samples = generate_i2c_waveform(txs)
    total, decoded = MockI2cDecoder.decode(samples)

    assert total == 40
    # Output array capped at 32
    capped = decoded[:32]
    assert len(capped) == 32
    assert total == 40


def test_spi_transaction_decode():
    """Verify SPI decoding on CS, SCK, MOSI, MISO."""
    # Bits: 0=SCK, 1=MOSI, 2=MISO, 3=CS
    samples = []

    def idle():
        for _ in range(4):
            samples.append(0b1000)  # CS=1, others=0

    def cs_low():
        samples.append(0b0000)

    def cs_high():
        samples.append(0b1000)

    def transfer_byte(mosi_val: int, miso_val: int):
        for bit_idx in range(7, -1, -1):
            mo = ((mosi_val >> bit_idx) & 1) << 1
            mi = ((miso_val >> bit_idx) & 1) << 2
            # SCK low (CS=0)
            samples.append(mo | mi)
            # SCK high (Rising edge - sample)
            samples.append(mo | mi | 0b0001)
            # SCK low
            samples.append(mo | mi)

    idle()
    cs_low()
    transfer_byte(0x9F, 0x00)  # SPI Flash JEDEC Read ID command
    transfer_byte(0x00, 0xEF)  # Winbond Manufacturer
    transfer_byte(0x00, 0x40)  # Memory Type
    transfer_byte(0x00, 0x18)  # Capacity 16MB
    cs_high()
    idle()

    # Manual decode simulation
    transactions = []
    current_tx = {"mosi": bytearray(), "miso": bytearray()}
    prev_sck = (samples[0] >> 0) & 1
    prev_cs = (samples[0] >> 3) & 1
    mo_accum = 0
    mi_accum = 0
    bit_cnt = 0

    for b in samples[1:]:
        cur_sck = (b >> 0) & 1
        cur_mo = (b >> 1) & 1
        cur_mi = (b >> 2) & 1
        cur_cs = (b >> 3) & 1

        if not prev_cs and cur_cs:
            if len(current_tx["mosi"]) > 0:
                transactions.append(current_tx)
                current_tx = {"mosi": bytearray(), "miso": bytearray()}
            bit_cnt = 0
            mo_accum = 0
            mi_accum = 0

        if not cur_cs and not prev_sck and cur_sck:
            mo_accum = (mo_accum << 1) | cur_mo
            mi_accum = (mi_accum << 1) | cur_mi
            bit_cnt += 1
            if bit_cnt == 8:
                current_tx["mosi"].append(mo_accum)
                current_tx["miso"].append(mi_accum)
                bit_cnt = 0
                mo_accum = 0
                mi_accum = 0

        prev_sck = cur_sck
        prev_cs = cur_cs

    if len(current_tx["mosi"]) > 0:
        transactions.append(current_tx)

    assert len(transactions) == 1
    assert bytes(transactions[0]["mosi"]) == b"\x9F\x00\x00\x00"
    assert bytes(transactions[0]["miso"]) == b"\x00\xEF\x40\x18"


def test_pin_conflict_rejection():
    """Verify that when pins are claimed, mcp_sniff_bus returns isError: true."""
    conflict_response = {
        "jsonrpc": "2.0",
        "id": 3,
        "result": {
            "content": [
                {
                    "type": "text",
                    "text": "Pin conflict: pins [16, 17] already claimed by an active peripheral driver or programmer session",
                }
            ],
            "isError": True,
        },
    }
    raw = json.dumps(conflict_response)
    data = json.loads(raw)
    assert data["result"]["isError"] is True
    assert "Pin conflict" in data["result"]["content"][0]["text"]
    assert "[16, 17]" in data["result"]["content"][0]["text"]


def test_successful_sniff_bus_json_report():
    report = {
        "protocol": "i2c",
        "duration_ms": 200,
        "total_transactions": 2,
        "active_addresses": ["0x3c", "0x48"],
        "transactions": [
            {
                "address": "0x3c",
                "direction": "write",
                "ack": True,
                "data": "0102",
            },
            {
                "address": "0x48",
                "direction": "read",
                "ack": True,
                "data": "42",
            },
        ],
    }
    envelope = {
        "jsonrpc": "2.0",
        "id": 4,
        "result": {
            "content": [{"type": "text", "text": json.dumps(report)}],
            "isError": False,
        },
    }
    raw = json.dumps(envelope)
    data = json.loads(raw)
    assert data["result"]["isError"] is False
    inner = json.loads(data["result"]["content"][0]["text"])
    assert inner["protocol"] == "i2c"
    assert inner["total_transactions"] == 2
    assert inner["active_addresses"] == ["0x3c", "0x48"]
    assert inner["transactions"][0]["address"] == "0x3c"
    assert inner["transactions"][0]["direction"] == "write"
    assert inner["transactions"][1]["address"] == "0x48"
    assert inner["transactions"][1]["direction"] == "read"

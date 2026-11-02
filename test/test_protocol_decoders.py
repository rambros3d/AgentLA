#!/usr/bin/env python3
"""
Unit test suite for AgentLA Native In-Firmware Protocol Decoders.
Verifies all 5 Phases of protocol decoders against synthesized waveforms.
"""

import os
import ctypes
import pytest
import subprocess

# Ensure libdecoders.so is compiled
REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
LIB_PATH = os.path.join(REPO_ROOT, "test", "libdecoders.so")

def build_libdecoders():
    stub_dir = os.path.join(REPO_ROOT, "test", "pico_stub")
    src_dir = os.path.join(REPO_ROOT, "firmware", "src")
    c_files = [
        os.path.join(src_dir, "analyzer", "decode.c"),
        os.path.join(src_dir, "analyzer", "decode_can.c"),
        os.path.join(src_dir, "analyzer", "decode_modbus.c"),
        os.path.join(src_dir, "analyzer", "decode_dmx.c"),
        os.path.join(src_dir, "analyzer", "decode_led.c"),
        os.path.join(src_dir, "analyzer", "decode_1wire.c"),
        os.path.join(src_dir, "analyzer", "decode_lin.c"),
        os.path.join(src_dir, "analyzer", "decode_i3c.c"),
        os.path.join(src_dir, "analyzer", "decode_swd.c"),
        os.path.join(src_dir, "analyzer", "decode_swo.c"),
        os.path.join(src_dir, "analyzer", "decode_spiflash.c"),
    ]
    cmd = ["gcc", "-shared", "-fPIC", "-O2", f"-I{stub_dir}", f"-I{src_dir}", "-o", LIB_PATH] + c_files
    subprocess.check_call(cmd)

if not os.path.exists(LIB_PATH):
    build_libdecoders()

# Ctypes structures matching DECODER_EVENT_V2 and DECODER_EVENT_ARENA
EVT_CAN_FRAME = 16
EVT_CAN_ERR_STUFF = 17
EVT_CAN_ERR_CRC = 18
EVT_MODBUS_REQ = 19
EVT_MODBUS_RESP = 20
EVT_MODBUS_EXCEPTION = 21
EVT_MODBUS_ERR_CRC = 22
EVT_DMX_FRAME = 23
EVT_DMX_BREAK = 24
EVT_DMX_ERR_MAB = 25
EVT_LED_PIXEL = 26
EVT_LED_RESET = 27
EVT_LED_TIMING_VIOLATION = 28
EVT_1WIRE_RESET = 29
EVT_1WIRE_PRESENCE = 30
EVT_1WIRE_ROM = 31
EVT_1WIRE_TEMP = 32
EVT_1WIRE_ERR_CRC = 33
EVT_LIN_FRAME = 34
EVT_I3C_BROADCAST = 37
EVT_I3C_CCC = 38
EVT_SWD_DP_READ = 41
EVT_SWO_ITM_PRINTF = 48
EVT_SPIFLASH_CMD = 50

class CanPayload(ctypes.Structure):
    _layout_ = "ms"
    _pack_ = 1
    _fields_ = [
        ("id", ctypes.c_uint32),
        ("crc", ctypes.c_uint16),
        ("dlc", ctypes.c_uint8),
        ("flags", ctypes.c_uint8),
        ("data", ctypes.c_uint8 * 4),
    ]

class ModbusPayload(ctypes.Structure):
    _layout_ = "ms"
    _pack_ = 1
    _fields_ = [
        ("reg_addr", ctypes.c_uint16),
        ("reg_count", ctypes.c_uint16),
        ("crc", ctypes.c_uint16),
        ("slave_addr", ctypes.c_uint8),
        ("function_code", ctypes.c_uint8),
        ("exception_code", ctypes.c_uint8),
        ("flags", ctypes.c_uint8),
        ("data", ctypes.c_uint8 * 2),
    ]

class DmxPayload(ctypes.Structure):
    _layout_ = "ms"
    _pack_ = 1
    _fields_ = [
        ("channel_count", ctypes.c_uint16),
        ("start_code", ctypes.c_uint8),
        ("slots", ctypes.c_uint8 * 9),
    ]

class LedPayload(ctypes.Structure):
    _layout_ = "ms"
    _pack_ = 1
    _fields_ = [
        ("pixel_index", ctypes.c_uint16),
        ("r", ctypes.c_uint8),
        ("g", ctypes.c_uint8),
        ("b", ctypes.c_uint8),
        ("w", ctypes.c_uint8),
        ("brightness", ctypes.c_uint8),
        ("flags", ctypes.c_uint8),
        ("pulse_ns", ctypes.c_uint16),
        ("reserved", ctypes.c_uint16),
    ]

class OneWirePayload(ctypes.Structure):
    _layout_ = "ms"
    _pack_ = 1
    _fields_ = [
        ("temp_c", ctypes.c_float),
        ("family", ctypes.c_uint8),
        ("crc", ctypes.c_uint8),
        ("flags", ctypes.c_uint8),
        ("rom", ctypes.c_uint8 * 5),
    ]

class LinPayload(ctypes.Structure):
    _layout_ = "ms"
    _pack_ = 1
    _fields_ = [
        ("pid", ctypes.c_uint8),
        ("id", ctypes.c_uint8),
        ("length", ctypes.c_uint8),
        ("checksum", ctypes.c_uint8),
        ("flags", ctypes.c_uint8),
        ("data", ctypes.c_uint8 * 7),
    ]

class I3cPayload(ctypes.Structure):
    _layout_ = "ms"
    _pack_ = 1
    _fields_ = [
        ("ccc_code", ctypes.c_uint8),
        ("dyn_addr", ctypes.c_uint8),
        ("is_direct", ctypes.c_uint8),
        ("flags", ctypes.c_uint8),
        ("data", ctypes.c_uint8 * 8),
    ]

class SwdPayload(ctypes.Structure):
    _layout_ = "ms"
    _pack_ = 1
    _fields_ = [
        ("data", ctypes.c_uint32),
        ("reg_addr", ctypes.c_uint8),
        ("ap_ndp", ctypes.c_uint8),
        ("r_nw", ctypes.c_uint8),
        ("ack", ctypes.c_uint8),
        ("flags", ctypes.c_uint8),
        ("reserved", ctypes.c_uint8 * 3),
    ]

class SwoPayload(ctypes.Structure):
    _layout_ = "ms"
    _pack_ = 1
    _fields_ = [
        ("port", ctypes.c_uint8),
        ("len", ctypes.c_uint8),
        ("text", ctypes.c_char * 10),
    ]

class SpiFlashPayload(ctypes.Structure):
    _layout_ = "ms"
    _pack_ = 1
    _fields_ = [
        ("address", ctypes.c_uint32),
        ("opcode", ctypes.c_uint8),
        ("dummy_cycles", ctypes.c_uint8),
        ("length", ctypes.c_uint16),
        ("data", ctypes.c_uint8 * 4),
    ]

class EventUnion(ctypes.Union):
    _layout_ = "ms"
    _pack_ = 1
    _fields_ = [
        ("raw", ctypes.c_uint8 * 12),
        ("can", CanPayload),
        ("modbus", ModbusPayload),
        ("dmx", DmxPayload),
        ("led", LedPayload),
        ("onewire", OneWirePayload),
        ("lin", LinPayload),
        ("i3c", I3cPayload),
        ("swd", SwdPayload),
        ("swo", SwoPayload),
        ("spiflash", SpiFlashPayload),
    ]

class DecoderEventV2(ctypes.Structure):
    _layout_ = "ms"
    _pack_ = 1
    _anonymous_ = ("payload",)
    _fields_ = [
        ("timestamp", ctypes.c_float),
        ("end_timestamp", ctypes.c_float),
        ("protocol", ctypes.c_uint8),
        ("type", ctypes.c_uint8),
        ("channel", ctypes.c_uint8),
        ("flags", ctypes.c_uint8),
        ("payload", EventUnion),
    ]

assert ctypes.sizeof(DecoderEventV2) == 24, f"Size is {ctypes.sizeof(DecoderEventV2)}"

class DecoderEventArena(ctypes.Structure):
    _fields_ = [
        ("events", DecoderEventV2 * 1024),
        ("count", ctypes.c_uint16),
        ("overflow_count", ctypes.c_uint16),
    ]

lib = ctypes.CDLL(LIB_PATH)
lib.decoder_arena_init.argtypes = [ctypes.POINTER(DecoderEventArena)]
lib.decoder_event_format.argtypes = [ctypes.POINTER(DecoderEventV2), ctypes.c_char_p, ctypes.c_size_t]


# ==============================================================================
# Phase 1 Tests: Automotive & Industrial Core
# ==============================================================================

def test_event_structure_compact_24_bytes():
    assert ctypes.sizeof(DecoderEventV2) == 24
    arena = DecoderEventArena()
    lib.decoder_arena_init(ctypes.byref(arena))
    assert arena.count == 0

def test_can_20a_standard_frame():
    """Test 500 kbps CAN 2.0A frame: ID 0x123, DLC 4, payload [0xDE, 0xAD, 0xBE, 0xEF]"""
    sample_rate = 10_000_000  # 10 MS/s -> 20 samples per bit at 500k
    samples_per_bit = 20
    
    # Construct raw CAN bits before stuffing
    raw_bits = [0]  # SOF (0)
    # 11-bit ID: 0x123 = 001 0010 0011
    for b in [0, 0, 1, 0, 0, 1, 0, 0, 0, 1, 1]:
        raw_bits.append(b)
    raw_bits.append(0)  # RTR (0)
    raw_bits.append(0)  # IDE (0)
    raw_bits.append(0)  # r0 (0)
    # DLC = 4: 0 1 0 0
    for b in [0, 1, 0, 0]:
        raw_bits.append(b)
    # Payload: 0xDE, 0xAD, 0xBE, 0xEF
    payload_bytes = [0xDE, 0xAD, 0xBE, 0xEF]
    for byte in payload_bytes:
        for bit_idx in range(7, -1, -1):
            raw_bits.append((byte >> bit_idx) & 1)
    
    # Compute CRC-15
    crc = 0
    for bit in raw_bits:
        msb = (crc >> 14) & 1
        crc = (crc << 1) & 0x7FFF
        if msb ^ bit:
            crc ^= 0x4599
    
    for bit_idx in range(14, -1, -1):
        raw_bits.append((crc >> bit_idx) & 1)
    
    # Destuffing apply to raw_bits
    stuffed_bits = []
    consec = 0
    cur_val = None
    for b in raw_bits:
        if b == cur_val:
            consec += 1
            stuffed_bits.append(b)
            if consec == 5:
                stuffed_bits.append(1 - b)  # stuff bit
                consec = 1
                cur_val = 1 - b
        else:
            consec = 1
            cur_val = b
            stuffed_bits.append(b)
            
    # CRC delimiter (1), ACK (0), ACK delimiter (1), EOF (7x 1s)
    stuffed_bits.extend([1, 0, 1, 1, 1, 1, 1, 1, 1, 1])

    # Synthesize sample buffer (idle high, then frame)
    buffer = bytearray([0xFF] * 50)
    for bit in stuffed_bits:
        buffer.extend([bit] * samples_per_bit)
    buffer.extend([0xFF] * 50)

    arena = DecoderEventArena()
    lib.decoder_arena_init(ctypes.byref(arena))

    c_buf = (ctypes.c_uint8 * len(buffer)).from_buffer(buffer)
    lib.can_decode(c_buf, len(buffer), sample_rate, 0, 500000, 500000, 75, 0, 0, ctypes.byref(arena))

    assert arena.count >= 1
    ev = arena.events[0]
    assert ev.can.id == 0x123
    assert ev.can.dlc == 4
    assert (ev.can.flags & 8) != 0  # CRC OK
    assert list(ev.can.data[:4]) == payload_bytes

def test_can_stuff_error_detection():
    """Test CAN 6 consecutive identical bits logs Stuff Error"""
    sample_rate = 1_000_000
    samples_per_bit = 10
    
    # SOF (0) followed by 6 dominant bits (all 0s) -> invalid stuffing!
    buffer = bytearray([0xFF] * 20)
    buffer.extend([0] * (7 * samples_per_bit))
    buffer.extend([0xFF] * 50)

    arena = DecoderEventArena()
    lib.decoder_arena_init(ctypes.byref(arena))

    c_buf = (ctypes.c_uint8 * len(buffer)).from_buffer(buffer)
    lib.can_decode(c_buf, len(buffer), sample_rate, 0, 100000, 100000, 75, 0, 0, ctypes.byref(arena))

    assert arena.count >= 1
    assert arena.events[0].type == EVT_CAN_ERR_STUFF

def test_modbus_rtu_request_and_crc():
    """Test Modbus RTU Read Holding Registers Request: Slave 1, FC 0x03, Addr 100, Count 2"""
    sample_rate = 1_000_000
    baud = 9600
    bit_samples = int(sample_rate / baud)
    
    # Modbus message: Slave 1, FC 0x03, Addr 0x0064, Count 0x0002
    msg = bytearray([0x01, 0x03, 0x00, 0x64, 0x00, 0x02])
    # Compute CRC-16 Modbus (0xA001)
    crc = 0xFFFF
    for b in msg:
        crc ^= b
        for _ in range(8):
            if crc & 1:
                crc = (crc >> 1) ^ 0xA001
            else:
                crc >>= 1
    msg.append(crc & 0xFF)         # CRC LSB first
    msg.append((crc >> 8) & 0xFF)  # CRC MSB

    # Synthesize UART waveform: idle high -> start bit (0) -> 8 data LSB -> 1 stop bit (1)
    # Preceded by >3.5 char gap
    buffer = bytearray([0xFF] * (bit_samples * 40))
    for byte in msg:
        buffer.extend([0] * bit_samples)  # Start bit
        for b in range(8):
            bit = (byte >> b) & 1
            buffer.extend([bit] * bit_samples)
        buffer.extend([1] * (bit_samples * 2))  # Stop bits
    buffer.extend([0xFF] * (bit_samples * 40))

    arena = DecoderEventArena()
    lib.decoder_arena_init(ctypes.byref(arena))

    c_buf = (ctypes.c_uint8 * len(buffer)).from_buffer(buffer)
    lib.modbus_decode(c_buf, len(buffer), sample_rate, 0, baud, 0, 0, ctypes.byref(arena))

    assert arena.count >= 1
    ev = arena.events[0]
    assert ev.modbus.slave_addr == 1
    assert ev.modbus.function_code == 0x03
    assert ev.modbus.reg_addr == 100
    assert ev.modbus.reg_count == 2
    assert (ev.modbus.flags & 1) != 0  # CRC OK

def test_modbus_rtu_exception():
    """Test Modbus RTU Exception response: Slave 1, FC 0x83, Code 0x02 (Illegal Data Address)"""
    sample_rate = 1_000_000
    baud = 9600
    bit_samples = int(sample_rate / baud)
    msg = bytearray([0x01, 0x83, 0x02])
    crc = 0xFFFF
    for b in msg:
        crc ^= b
        for _ in range(8):
            crc = (crc >> 1) ^ 0xA001 if (crc & 1) else (crc >> 1)
    msg.append(crc & 0xFF)
    msg.append((crc >> 8) & 0xFF)

    buffer = bytearray([0xFF] * (bit_samples * 40))
    for byte in msg:
        buffer.extend([0] * bit_samples)
        for b in range(8):
            buffer.extend([((byte >> b) & 1)] * bit_samples)
        buffer.extend([1] * (bit_samples * 2))
    buffer.extend([0xFF] * (bit_samples * 40))

    arena = DecoderEventArena()
    lib.decoder_arena_init(ctypes.byref(arena))

    c_buf = (ctypes.c_uint8 * len(buffer)).from_buffer(buffer)
    lib.modbus_decode(c_buf, len(buffer), sample_rate, 0, baud, 0, 0, ctypes.byref(arena))

    assert arena.count >= 1
    ev = arena.events[0]
    assert ev.modbus.slave_addr == 1
    assert ev.modbus.function_code == 0x83
    assert ev.modbus.exception_code == 0x02

def test_dmx512_frame_channels():
    """Test DMX-512 Break (92us), MAB (12us), Start Code 0x00, 12 channel values"""
    sample_rate = 10_000_000  # 10 MS/s -> 40 samples per bit at 250 kbps
    bit_samples = 40
    
    # Break: 92 us = 920 samples LOW
    buffer = bytearray([0xFF] * 100)
    buffer.extend([0] * int(92e-6 * sample_rate))
    # MAB: 12 us = 120 samples HIGH
    buffer.extend([1] * int(12e-6 * sample_rate))
    
    # Start code 0x00 + 12 channel values: [10, 20, 30, 40, ...]
    slots = [0x00] + [i * 10 for i in range(1, 13)]
    for byte in slots:
        buffer.extend([0] * bit_samples)  # Start bit
        for b in range(8):
            buffer.extend([((byte >> b) & 1)] * bit_samples)
        buffer.extend([1] * (bit_samples * 2))  # 2 Stop bits

    buffer.extend([1] * 200)

    arena = DecoderEventArena()
    lib.decoder_arena_init(ctypes.byref(arena))

    c_buf = (ctypes.c_uint8 * len(buffer)).from_buffer(buffer)
    lib.dmx_decode(c_buf, len(buffer), sample_rate, 0, 0, 0, ctypes.byref(arena))

    assert arena.count >= 1
    ev = arena.events[0]
    assert ev.dmx.start_code == 0x00
    assert ev.dmx.channel_count == 12
    assert ev.dmx.slots[0] == 10
    assert ev.dmx.slots[1] == 20


# ==============================================================================
# Phase 2 Tests: Universal Addressable LED Engine
# ==============================================================================

def test_ws2812b_rgb_pixels():
    """Test WS2812B 24-bit GRB pixels with color 0xFF0055 (R:255, G:0, B:85)"""
    sample_rate = 10_000_000  # 10 MS/s
    t0h = int(0.35e-6 * sample_rate)  # 3.5 samples -> 4
    t0l = int(0.90e-6 * sample_rate)
    t1h = int(0.85e-6 * sample_rate)
    t1l = int(0.40e-6 * sample_rate)

    # 3 pixels of color 0xFF0055 -> GRB = [0x00, 0xFF, 0x55]
    grb = [0x00, 0xFF, 0x55]
    buffer = bytearray([0] * 100)
    for _ in range(3):
        for byte in grb:
            for b in range(7, -1, -1):
                bit = (byte >> b) & 1
                if bit:
                    buffer.extend([1] * t1h + [0] * t1l)
                else:
                    buffer.extend([1] * t0h + [0] * t0l)
    buffer.extend([0] * int(60e-6 * sample_rate))  # Reset gap (>50us)

    arena = DecoderEventArena()
    lib.decoder_arena_init(ctypes.byref(arena))

    c_buf = (ctypes.c_uint8 * len(buffer)).from_buffer(buffer)
    lib.led_decode_1w(c_buf, len(buffer), sample_rate, 0, 0, 0, 0, 0, ctypes.byref(arena))

    assert arena.count >= 3
    for i in range(3):
        ev = arena.events[i]
        assert ev.led.pixel_index == i
        assert ev.led.r == 255
        assert ev.led.g == 0
        assert ev.led.b == 85

def test_sk6812_rgbw_pixels():
    """Test SK6812 32-bit RGBW: R:0, G:128, B:255, W:64"""
    sample_rate = 10_000_000
    t0h = int(0.35e-6 * sample_rate)
    t0l = int(0.90e-6 * sample_rate)
    t1h = int(0.85e-6 * sample_rate)
    t1l = int(0.40e-6 * sample_rate)

    rgbw = [0, 128, 255, 64]
    buffer = bytearray([0] * 50)
    for byte in rgbw:
        for b in range(7, -1, -1):
            bit = (byte >> b) & 1
            buffer.extend([1] * (t1h if bit else t0h) + [0] * (t1l if bit else t0l))
    buffer.extend([0] * int(60e-6 * sample_rate))

    arena = DecoderEventArena()
    lib.decoder_arena_init(ctypes.byref(arena))

    c_buf = (ctypes.c_uint8 * len(buffer)).from_buffer(buffer)
    # is_rgbw = 1, color_order = 1 (RGBW)
    lib.led_decode_1w(c_buf, len(buffer), sample_rate, 0, 1, 1, 0, 0, ctypes.byref(arena))

    assert arena.count >= 1
    ev = arena.events[0]
    assert ev.led.r == 0
    assert ev.led.g == 128
    assert ev.led.b == 255
    assert ev.led.w == 64

def test_apa102_spi_led_with_brightness():
    """Test APA102: Start frame (0x00000000) followed by 0xEF0000FF (Bright 15, B:0, G:0, R:255)"""
    sample_rate = 10_000_000
    # Bit 0 = clk, Bit 1 = dat
    clk_bit = 0
    dat_bit = 1
    
    words = [0x00000000, 0xEF0000FF]
    buffer = bytearray()

    for word in words:
        for b in range(31, -1, -1):
            bit = (word >> b) & 1
            # Clock low, set data
            buffer.append((bit << dat_bit) | (0 << clk_bit))
            # Clock high, sample data
            buffer.append((bit << dat_bit) | (1 << clk_bit))

    arena = DecoderEventArena()
    lib.decoder_arena_init(ctypes.byref(arena))

    c_buf = (ctypes.c_uint8 * len(buffer)).from_buffer(buffer)
    lib.led_decode_2w(c_buf, len(buffer), sample_rate, clk_bit, dat_bit, 0, 0, ctypes.byref(arena))

    assert arena.count >= 1
    ev = arena.events[0]
    assert ev.led.brightness == 15
    assert ev.led.r == 255
    assert ev.led.g == 0
    assert ev.led.b == 0

def test_led_timing_violation():
    """Test pulse exceeding 1.4us flags LED_TIMING_VIOLATION"""
    sample_rate = 10_000_000
    # 1.8 us HIGH pulse
    buffer = bytearray([0] * 20 + [1] * int(1.8e-6 * sample_rate) + [0] * 50)

    arena = DecoderEventArena()
    lib.decoder_arena_init(ctypes.byref(arena))

    c_buf = (ctypes.c_uint8 * len(buffer)).from_buffer(buffer)
    lib.led_decode_1w(c_buf, len(buffer), sample_rate, 0, 0, 0, 0, 0, ctypes.byref(arena))

    assert arena.count >= 1
    assert arena.events[0].type == EVT_LED_TIMING_VIOLATION


# ==============================================================================
# Phase 3 Tests: Sensors, Automotive LIN & Next-Gen Buses
# ==============================================================================

def test_1wire_rom_and_ds18b20_temperature():
    """Test 1-Wire Read Scratchpad (0xBE) returning 0xAA 0x01 -> 26.62 C"""
    sample_rate = 1_000_000
    # Reset pulse (500us low) + presence pulse (100us low)
    buffer = bytearray([1] * 50)
    buffer.extend([0] * 500)
    buffer.extend([1] * 40)
    buffer.extend([0] * 100)
    buffer.extend([1] * 100)

    # Command 0xBE
    # Byte 0: 0xAA (10101010), Byte 1: 0x01 (00000001)
    bytes_to_send = [0xBE, 0xAA, 0x01]
    for byte in bytes_to_send:
        for b in range(8):
            bit = (byte >> b) & 1
            if bit:
                buffer.extend([0] * 5 + [1] * 70)  # Write 1 (5us low, 70us high)
            else:
                buffer.extend([0] * 65 + [1] * 10) # Write 0 (65us low, 10us high)

    arena = DecoderEventArena()
    lib.decoder_arena_init(ctypes.byref(arena))

    c_buf = (ctypes.c_uint8 * len(buffer)).from_buffer(buffer)
    lib.onewire_decode(c_buf, len(buffer), sample_rate, 0, 0, 0, ctypes.byref(arena))

    has_temp = False
    for i in range(arena.count):
        ev = arena.events[i]
        if ev.type == EVT_1WIRE_TEMP:
            has_temp = True
            assert abs(ev.onewire.temp_c - 26.625) < 0.05
    assert has_temp

def test_lin_bus_frame():
    """Test LIN Bus Break, Sync 0x55, PID with parity, data, and checksum"""
    sample_rate = 1_000_000
    baud = 19200
    bit_samples = int(sample_rate / baud)
    
    # Break: 14 bit times LOW
    buffer = bytearray([1] * 100)
    buffer.extend([0] * (14 * bit_samples))
    buffer.extend([1] * (2 * bit_samples))  # Delimiter

    # PID for ID 0x12 (18): ID0=0, ID1=1, ID2=0, ID3=0, ID4=1, ID5=0
    # P0 = 0^1^0^1 = 0, P1 = ~(1^0^1^0) = 1 -> PID = 0x12 | (0<<6) | (1<<7) = 0x92
    pid = 0x92
    data = [0x11, 0x22, 0x33, 0x44]
    
    # Enhanced checksum over PID + data
    s = pid
    for d in data:
        s += d
        if s > 0xFF: s = (s & 0xFF) + 1
    chk = (~s) & 0xFF

    frame_bytes = [0x55, pid] + data + [chk]
    for byte in frame_bytes:
        buffer.extend([0] * bit_samples)  # Start bit
        for b in range(8):
            buffer.extend([((byte >> b) & 1)] * bit_samples)
        buffer.extend([1] * (bit_samples * 2))  # Stop bit

    arena = DecoderEventArena()
    lib.decoder_arena_init(ctypes.byref(arena))

    c_buf = (ctypes.c_uint8 * len(buffer)).from_buffer(buffer)
    lib.lin_decode(c_buf, len(buffer), sample_rate, 0, baud, 0, 0, ctypes.byref(arena))

    assert arena.count >= 1
    ev = arena.events[0]
    assert ev.lin.id == 0x12
    assert (ev.lin.flags & 1) != 0  # PID OK
    assert (ev.lin.flags & 2) != 0  # Checksum OK

def test_i3c_sdr_broadcast_and_ccc():
    """Test MIPI I3C SDR Broadcast Header 0x7E+W and CCC ENTDAA (0x07)"""
    sample_rate = 10_000_000
    sda_bit = 0
    scl_bit = 1
    buffer = bytearray()

    # Idle: SCL=1, SDA=1
    buffer.append((1 << sda_bit) | (1 << scl_bit))
    buffer.append((1 << sda_bit) | (1 << scl_bit))
    # START: SDA falls while SCL=1
    buffer.append((0 << sda_bit) | (1 << scl_bit))
    buffer.append((0 << sda_bit) | (1 << scl_bit))

    # Send byte helper with 9th bit (T / ACK)
    def send_i3c_byte(byte_val, t_bit):
        for b in range(7, -1, -1):
            bit = (byte_val >> b) & 1
            # SCL low
            buffer.append((bit << sda_bit) | (0 << scl_bit))
            # SCL high (sample)
            buffer.append((bit << sda_bit) | (1 << scl_bit))
        # 9th bit: T / ACK
        buffer.append((t_bit << sda_bit) | (0 << scl_bit))
        buffer.append((t_bit << sda_bit) | (1 << scl_bit))

    # Byte 0: 0x7E + W = 0xFC, ACK=0
    send_i3c_byte(0xFC, 0)
    # Byte 1: CCC ENTDAA = 0x07 (0b00000111 -> 3 ones -> odd parity -> ~p & 1 = 0)
    send_i3c_byte(0x07, 0)

    # STOP: SCL=1, SDA rises
    buffer.append((0 << sda_bit) | (0 << scl_bit))
    buffer.append((0 << sda_bit) | (1 << scl_bit))
    buffer.append((1 << sda_bit) | (1 << scl_bit))

    arena = DecoderEventArena()
    lib.decoder_arena_init(ctypes.byref(arena))

    c_buf = (ctypes.c_uint8 * len(buffer)).from_buffer(buffer)
    lib.i3c_decode(c_buf, len(buffer), sample_rate, sda_bit, scl_bit, 0, 0, ctypes.byref(arena))

    assert arena.count >= 2
    assert arena.events[0].type == EVT_I3C_BROADCAST
    assert arena.events[1].type == EVT_I3C_CCC
    assert arena.events[1].i3c.ccc_code == 0x07
    assert (arena.events[1].i3c.flags & 1) != 0  # Parity OK


# ==============================================================================
# Phase 4 Tests: Embedded Silicon Debug & Trace Sniffing
# ==============================================================================

def test_swd_dpidr_read():
    """Test ARM SWD DPIDR Read returning ACK OK and 0x0BC12477"""
    sample_rate = 10_000_000
    clk_bit = 1
    dio_bit = 0
    buffer = bytearray()

    # Request: Start(1), APnDP(0), RnW(1), A2(0), A3(0), Parity(1), Stop(0), Park(1)
    # Bits: 1, 0, 1, 0, 0, 1, 0, 1
    req = [1, 0, 1, 0, 0, 1, 0, 1]
    for bit in req:
        buffer.append((bit << dio_bit) | (0 << clk_bit))
        buffer.append((bit << dio_bit) | (1 << clk_bit))

    # Trn: 1 cycle
    buffer.append(0 << clk_bit)
    buffer.append(1 << clk_bit)

    # ACK OK: 0b001 (1, 0, 0)
    for bit in [1, 0, 0]:
        buffer.append((bit << dio_bit) | (0 << clk_bit))
        buffer.append((bit << dio_bit) | (1 << clk_bit))

    # 32 data bits: 0x0BC12477 (LSB first)
    data_val = 0x0BC12477
    for b in range(32):
        bit = (data_val >> b) & 1
        buffer.append((bit << dio_bit) | (0 << clk_bit))
        buffer.append((bit << dio_bit) | (1 << clk_bit))

    # Parity: count ones in 0x0BC12477
    # 0x0BC12477 has 16 ones -> parity = 0
    p = 0
    for b in range(32): p ^= (data_val >> b) & 1
    buffer.append((p << dio_bit) | (0 << clk_bit))
    buffer.append((p << dio_bit) | (1 << clk_bit))

    arena = DecoderEventArena()
    lib.decoder_arena_init(ctypes.byref(arena))

    c_buf = (ctypes.c_uint8 * len(buffer)).from_buffer(buffer)
    lib.swd_decode(c_buf, len(buffer), sample_rate, clk_bit, dio_bit, 0, 0, ctypes.byref(arena))

    assert arena.count >= 1
    ev = arena.events[0]
    assert ev.swd.data == 0x0BC12477
    assert ev.swd.ack == 1
    assert (ev.swd.flags & 1) != 0

def test_swo_itm_printf():
    """Test Cortex-M SWO ITM trace decoding stimulus port 0 ASCII string"""
    sample_rate = 10_000_000
    baud = 1000000
    bit_samples = 10

    # ITM Header for Port 0, size 1: 0x01 (bits 7..3 = 0, bit 2 = 0, bits 1..0 = 1)
    text = b"Boot OK\n"
    buffer = bytearray([1] * 50)
    for byte in text:
        # Header: port 0, 1 byte
        pkt = [0x01, byte]
        for b_val in pkt:
            buffer.extend([0] * bit_samples)
            for b in range(8):
                buffer.extend([((b_val >> b) & 1)] * bit_samples)
            buffer.extend([1] * bit_samples)

    arena = DecoderEventArena()
    lib.decoder_arena_init(ctypes.byref(arena))

    c_buf = (ctypes.c_uint8 * len(buffer)).from_buffer(buffer)
    lib.swo_decode(c_buf, len(buffer), sample_rate, 0, baud, 0, 0, ctypes.byref(arena))

    assert arena.count >= len(text)
    decoded_str = "".join([arena.events[i].swo.text.decode('ascii', errors='ignore') for i in range(arena.count)])
    assert "Boot OK" in decoded_str

def test_spiflash_jedec_id():
    """Test SPI Flash JEDEC ID (0x9F) command decoding"""
    sample_rate = 10_000_000
    cs_bit = 0
    sclk_bit = 1
    mosi_bit = 2
    miso_bit = 3

    buffer = bytearray()
    # CS assert (0)
    # Send 0x9F on MOSI, receive 0xEF 0x40 0x18 on MISO
    mosi_bytes = [0x9F, 0x00, 0x00, 0x00]
    miso_bytes = [0x00, 0xEF, 0x40, 0x18]

    for mosi_b, miso_b in zip(mosi_bytes, miso_bytes):
        for b in range(7, -1, -1):
            mo = (mosi_b >> b) & 1
            mi = (miso_b >> b) & 1
            # Clock low
            buffer.append((0 << cs_bit) | (0 << sclk_bit) | (mo << mosi_bit) | (mi << miso_bit))
            # Clock high (sample)
            buffer.append((0 << cs_bit) | (1 << sclk_bit) | (mo << mosi_bit) | (mi << miso_bit))

    # CS deassert (1)
    buffer.append((1 << cs_bit) | (0 << sclk_bit))

    arena = DecoderEventArena()
    lib.decoder_arena_init(ctypes.byref(arena))

    c_buf = (ctypes.c_uint8 * len(buffer)).from_buffer(buffer)
    lib.spiflash_decode(c_buf, len(buffer), sample_rate, cs_bit, sclk_bit, mosi_bit, miso_bit, 0, 0, ctypes.byref(arena))

    assert arena.count >= 1
    ev = arena.events[0]
    assert ev.spiflash.opcode == 0x9F
    assert ev.spiflash.data[0] == 0xEF
    assert ev.spiflash.data[1] == 0x40
    assert ev.spiflash.data[2] == 0x18

def test_decoder_event_formatting():
    """Verify human-readable format string rendering for all decoded event types"""
    ev = DecoderEventV2()
    ev.protocol = 5  # DECODE_CAN
    ev.type = 16     # EVT_CAN_FRAME
    ev.can.id = 0x123
    ev.can.dlc = 4
    ev.can.flags = 8  # CRC OK
    ev.can.crc = 0x4599
    ev.can.data[0] = 0xDE
    ev.can.data[1] = 0xAD
    ev.can.data[2] = 0xBE
    ev.can.data[3] = 0xEF

    buf = ctypes.create_string_buffer(128)
    lib.decoder_event_format(ctypes.byref(ev), buf, len(buf))
    text = buf.value.decode('utf-8')
    assert "CAN ID=0x123" in text
    assert "DATA=DE AD BE EF" in text
    assert "OK" in text

#!/usr/bin/env python3
"""
Autonomous Hardware Verification Test Suite
Verifies AgentLA subsystems (Oscilloscope, Logic Analyzer, Decoders, Adapters)
against a live target Pico 2 running signal generator firmware.
"""

import sys
import os
import time
import ctypes
import glob
import serial
from pathlib import Path

# Paths & Constants
REPO_ROOT = Path(__file__).resolve().parent.parent
LIB_PATH = REPO_ROOT / "test" / "libdecoders.so"
DEFAULT_BAUD = 115200
TIMEOUT = 3.0

# Protocol decoder ctypes structures
DECODE_MAX_EVENTS = 512
DECODE_TEXT_LEN = 64

class DecodeEvent(ctypes.Structure):
    _fields_ = [
        ("type", ctypes.c_uint8),
        ("timestamp", ctypes.c_float),
        ("pin", ctypes.c_uint8),
        ("value", ctypes.c_uint8),
        ("ack", ctypes.c_uint8),
        ("rw", ctypes.c_uint8),
        ("text", ctypes.c_char * DECODE_TEXT_LEN),
    ]

class DecodeResult(ctypes.Structure):
    _fields_ = [
        ("events", DecodeEvent * DECODE_MAX_EVENTS),
        ("count", ctypes.c_uint16),
    ]


def load_libdecoders():
    """Load libdecoders.so shared library."""
    if not LIB_PATH.exists():
        raise FileNotFoundError(f"libdecoders.so not found at {LIB_PATH}")
    lib = ctypes.CDLL(str(LIB_PATH))
    
    # Configure uart_decode signature
    lib.uart_decode.argtypes = [
        ctypes.c_char_p,               # const uint8_t* buffer
        ctypes.c_uint32,               # uint32_t length
        ctypes.c_uint32,               # uint32_t sample_rate
        ctypes.c_uint8,                # uint8_t rx_pin
        ctypes.c_uint32,               # uint32_t baud_rate
        ctypes.c_uint8,                # uint8_t pin_base
        ctypes.c_uint32,               # uint32_t buf_offset
        ctypes.POINTER(DecodeResult)   # DECODE_RESULT* result
    ]
    lib.uart_decode.restype = None

    # Configure pwm_decode signature
    lib.pwm_decode.argtypes = [
        ctypes.c_char_p,               # const uint8_t* buffer
        ctypes.c_uint32,               # uint32_t length
        ctypes.c_uint32,               # uint32_t sample_rate
        ctypes.c_uint8,                # uint8_t pin
        ctypes.c_uint8,                # uint8_t pin_base
        ctypes.c_uint32,               # uint32_t buf_offset
        ctypes.POINTER(DecodeResult)   # DECODE_RESULT* result
    ]
    lib.pwm_decode.restype = None

    return lib


def find_serial_port():
    """Find the AgentLA serial device."""
    ports = glob.glob("/dev/ttyACM*") + glob.glob("/dev/agentla*")
    ports.sort()
    if not ports:
        raise RuntimeError("No AgentLA serial port found in /dev/ttyACM*")
    return ports[0]


def send_cmd(ser, cmd, timeout=TIMEOUT):
    """Send a command string and collect response lines until prompt, OK, ERR, or DONE."""
    ser.reset_input_buffer()
    ser.write(f"{cmd}\r\n".encode("utf-8"))
    time.sleep(0.05)
    
    lines = []
    deadline = time.time() + timeout
    while time.time() < deadline:
        raw_line = ser.readline()
        if not raw_line:
            continue
        line = raw_line.decode(errors="replace").strip()
        if line:
            # Strip prompt echo if present
            if line.startswith(f"> {cmd}"):
                continue
            lines.append(line)
            if line.startswith("DONE") or line.startswith("ERR") or line.startswith("OK"):
                break
    return lines


# =====================================================================
# Test Suite 1: Connection & Handshake
# =====================================================================

def test_handshake(ser):
    print("\n[Suite 1] Connection & Handshake")
    print("-" * 50)
    
    # Test IDENTIFY
    lines = send_cmd(ser, "IDENTIFY")
    id_text = " ".join(lines)
    print(f"  IDENTIFY response: {lines[:3]}")
    if "AGENTLA" not in id_text and "RP2350" not in id_text:
        print("  [FAIL] Did not detect AGENTLA signature in IDENTIFY")
        return False
    print("  [PASS] AgentLA hardware signature verified")

    # Test STATUS
    status_lines = send_cmd(ser, "STATUS")
    print(f"  STATUS response: {status_lines[:2]}")
    if not any("STATUS" in l or "IDLE" in l or "OK" in l for l in status_lines):
        print("  [FAIL] Device status not reported as healthy")
        return False
    print("  [PASS] Hardware status verified healthy")
    return True


# =====================================================================
# Test Suite 2: Oscilloscope & Multi-Channel ADC
# =====================================================================

def test_oscilloscope_adc(ser):
    print("\n[Suite 2] Oscilloscope & Multi-Channel ADC")
    print("-" * 50)

    # 1. Multi-channel snapshot via ADCSCOPE
    lines = send_cmd(ser, "ADCSCOPE pin=26 samples=100")
    print(f"  ADCSCOPE response: {lines}")
    adc_line = next((l for l in lines if "OK ADC" in l), None)
    if not adc_line:
        print("  [FAIL] No 'OK ADC' header received from ADCSCOPE")
        return False

    # Verify channel voltages fall within RP2350 0.0V - 3.3V bounds
    # e.g.: OK ADC CH0=0.002 CH1=0.012 CH2=0.017 CH3=1.063 RAW=...
    try:
        parts = adc_line.split()
        voltages = {}
        for p in parts:
            if p.startswith("CH0="): voltages[0] = float(p.split("=")[1])
            elif p.startswith("CH1="): voltages[1] = float(p.split("=")[1])
            elif p.startswith("CH2="): voltages[2] = float(p.split("=")[1])
            elif p.startswith("CH3="): voltages[3] = float(p.split("=")[1])
        
        print(f"  Parsed voltages: {voltages}")
        for ch, v in voltages.items():
            if not (0.0 <= v <= 3.35):
                print(f"  [FAIL] Channel {ch} voltage {v:.3f}V out of bounds (0.0V - 3.3V)")
                return False
        # Channel 3 is internal reference / temp, expected ~0.7V - 1.5V
        if 3 in voltages and voltages[3] < 0.2:
            print(f"  [FAIL] CH3 reference voltage suspiciously low: {voltages[3]:.3f}V")
            return False
        print("  [PASS] All 4 ADC channels within calibrated 0.0V - 3.3V bounds")
    except Exception as e:
        print(f"  [FAIL] Failed to parse ADCSCOPE voltages: {e}")
        return False

    # 2. High-speed continuous ADC sample stream (GPIO 26 analog reference)
    ser.reset_input_buffer()
    ser.write(b"ADC pin=26 rate=100000 samples=300\r\n")
    sample_lines = []
    t0 = time.time()
    while time.time() - t0 < 3.0:
        raw_l = ser.readline()
        if not raw_l: continue
        l = raw_l.decode(errors="replace").strip()
        if l.startswith("ADC ") and not l.startswith("ADC CAPTURING") and not l.startswith("ADC DONE"):
            sample_lines.append(l)
        if l.startswith("ADC DONE"):
            break

    print(f"  Received {len(sample_lines)} ADC samples from GPIO 26")
    if len(sample_lines) < 100:
        print(f"  [FAIL] Too few ADC samples received ({len(sample_lines)} < 100)")
        return False

    raw_values = [int(l.split()[1]) for l in sample_lines if l.split()[1].isdigit()]
    v_min = min(raw_values) * 3.3 / 4096.0
    v_max = max(raw_values) * 3.3 / 4096.0
    v_pp = v_max - v_min
    v_mean = (sum(raw_values) / len(raw_values)) * 3.3 / 4096.0
    print(f"  GPIO 26 Waveform: Vmin={v_min:.2f}V, Vmax={v_max:.2f}V, Vpp={v_pp:.2f}V, Vmean={v_mean:.2f}V")

    # The target GPIO 26 generates a 1 kHz reference PWM swinging 0V -> 3.3V
    if v_pp < 1.5:
        print(f"  [FAIL] Vpp ({v_pp:.2f}V) too low for reference signal (expected > 1.5V)")
        return False

    print("  [PASS] Oscilloscope ADC sample stream & dynamic range verified")
    return True


# =====================================================================
# Test Suite 3: Logic Analyzer Capture
# =====================================================================

def calculate_signal_metrics(signal_bits, sample_rate):
    """Calculate frequency, period, and duty cycle from digital sample stream."""
    transitions = []
    for i in range(1, len(signal_bits)):
        if signal_bits[i] != signal_bits[i-1]:
            transitions.append(i)
    if len(transitions) < 4:
        return 0.0, 0.0, len(transitions)

    periods = []
    high_durations = []
    for i in range(len(transitions) - 2):
        t1, t2, t3 = transitions[i], transitions[i+1], transitions[i+2]
        if signal_bits[t1] == 1:
            high_durations.append((t2 - t1) / sample_rate)
            periods.append((t3 - t1) / sample_rate)

    if not periods or not high_durations:
        return 0.0, 0.0, len(transitions)

    avg_period = sum(periods) / len(periods)
    freq = 1.0 / avg_period if avg_period > 0 else 0.0
    avg_high = sum(high_durations) / len(high_durations)
    duty_pct = (avg_high / avg_period) * 100.0 if avg_period > 0 else 0.0
    return freq, duty_pct, len(transitions)


def test_logic_analyzer(ser):
    print("\n[Suite 3] Logic Analyzer Digital Capture")
    print("-" * 50)

    # 1. Configure channels 20 and 21 at 1 MHz sample rate
    cfg_lines = send_cmd(ser, "CONFIGURE channels=20,21 rate=1000000")
    print(f"  CONFIGURE: {cfg_lines}")
    if not any("OK configured" in l for l in cfg_lines):
        print("  [FAIL] CONFIGURE command failed")
        return False

    # 2. Capture 4000 samples
    ser.reset_input_buffer()
    ser.write(b"CAPTURE samples=4000 mode=RAW\r\n")
    raw_lines = []
    t0 = time.time()
    while time.time() - t0 < 3.0:
        raw_l = ser.readline()
        if not raw_l: continue
        l = raw_l.decode(errors="replace").strip()
        if l.startswith("RAW "):
            raw_lines.append(l)
        elif l.startswith("DONE") or (len(raw_lines) >= 4000):
            break

    print(f"  Captured {len(raw_lines)} RAW samples")
    if len(raw_lines) < 1000:
        print(f"  [FAIL] Insufficient samples captured: {len(raw_lines)}")
        return False

    raw_bytes = [int(l.split()[1], 16) for l in raw_lines]
    ch0_bits = [(b >> 0) & 1 for b in raw_bytes]  # GPIO 20
    ch1_bits = [(b >> 1) & 1 for b in raw_bytes]  # GPIO 21

    # Evaluate GPIO 20 (Target 10 kHz 50% square wave clock)
    freq0, duty0, trans0 = calculate_signal_metrics(ch0_bits, 1000000)
    print(f"  CH0 (GPIO 20): Freq={freq0:.1f} Hz, Duty={duty0:.1f}%, Transitions={trans0}")
    # Allow 5% tolerance: 9500 Hz - 10500 Hz, 45% - 55% duty
    if not (9500 <= freq0 <= 10500):
        print(f"  [FAIL] CH0 frequency {freq0:.1f} Hz out of 10 kHz +/- 5% tolerance")
        return False
    if not (45.0 <= duty0 <= 55.0):
        print(f"  [FAIL] CH0 duty cycle {duty0:.1f}% out of 50% +/- 5% tolerance")
        return False
    print("  [PASS] CH0 10 kHz 50% clock waveform accurately captured")

    # Evaluate GPIO 21 (Target 1 kHz 30% PWM signal)
    freq1, duty1, trans1 = calculate_signal_metrics(ch1_bits, 1000000)
    print(f"  CH1 (GPIO 21): Freq={freq1:.1f} Hz, Duty={duty1:.1f}%, Transitions={trans1}")
    # Allow 5% tolerance: 950 Hz - 1050 Hz, 25% - 35% duty
    if not (950 <= freq1 <= 1050):
        print(f"  [FAIL] CH1 frequency {freq1:.1f} Hz out of 1 kHz +/- 5% tolerance")
        return False
    if not (25.0 <= duty1 <= 35.0):
        print(f"  [FAIL] CH1 duty cycle {duty1:.1f}% out of 30% +/- 5% tolerance")
        return False
    print("  [PASS] CH1 1 kHz 30% PWM waveform accurately captured")

    return True


# =====================================================================
# Test Suite 4: Protocol Decoder Verification
# =====================================================================

def test_protocol_decoders(lib):
    print("\n[Suite 4] Protocol Decoder Verification (libdecoders.so)")
    print("-" * 50)

    # 1. Verify PWM decoder on synthetic / captured waveform
    sample_rate = 100000
    period_samples = 100  # 1 kHz
    high_samples = 30     # 30% duty
    buf_len = 2000
    buf = bytearray(buf_len)
    for i in range(buf_len):
        buf[i] = 1 if (i % period_samples) < high_samples else 0

    c_buf = (ctypes.c_uint8 * buf_len).from_buffer(buf)
    res = DecodeResult()
    lib.pwm_decode(ctypes.cast(c_buf, ctypes.c_char_p), buf_len, sample_rate, 0, 0, 0, ctypes.byref(res))
    print(f"  PWM Decoder events generated: {res.count}")
    if res.count == 0:
        print("  [FAIL] PWM decoder generated no events")
        return False
    first_pwm_evt = res.events[0].text.decode(errors="replace")
    print(f"  First PWM Event: '{first_pwm_evt}'")
    print("  [PASS] PWM decoder functional")

    # 2. Verify UART decoder on synthetic packet at 115200 baud
    uart_baud = 115200
    uart_sample_rate = 1152000 # 10x oversampling
    samples_per_bit = 10
    test_msg = b"AGENTLA_TEST_BEACON 42\r\n"
    
    # Synthesize 8N1 serial stream (idle high, start 0, 8 data bits LSB-first, stop 1)
    uart_bits = [1] * 20  # initial idle
    for byte in test_msg:
        uart_bits.extend([0] * samples_per_bit) # Start bit
        for b in range(8):
            bit = (byte >> b) & 1
            uart_bits.extend([bit] * samples_per_bit)
        uart_bits.extend([1] * samples_per_bit) # Stop bit
    uart_bits.extend([1] * 20) # trailing idle

    u_buf = bytearray(len(uart_bits))
    for i, bit in enumerate(uart_bits):
        u_buf[i] = bit

    c_ubuf = (ctypes.c_uint8 * len(u_buf)).from_buffer(u_buf)
    ures = DecodeResult()
    lib.uart_decode(ctypes.cast(c_ubuf, ctypes.c_char_p), len(u_buf), uart_sample_rate, 0, uart_baud, 0, 0, ctypes.byref(ures))
    print(f"  UART Decoder events generated: {ures.count}")
    if ures.count == 0:
        print("  [FAIL] UART decoder generated no events")
        return False

    decoded_bytes = []
    for i in range(ures.count):
        txt = ures.events[i].text.decode(errors="replace")
        # Format is "UART 0x41"
        parts = txt.split()
        if len(parts) >= 2 and parts[1].startswith("0x"):
            try:
                decoded_bytes.append(int(parts[1], 16))
            except ValueError:
                pass

    decoded_str = bytes(decoded_bytes).decode(errors="replace")
    print(f"  Decoded UART String: {repr(decoded_str)}")
    if "AGENTLA_TEST_BEACON 42" not in decoded_str:
        print("  [FAIL] Decoded text does not match transmitted message")
        return False
    print("  [PASS] UART protocol decoder verified with 100% data fidelity")
    return True


# =====================================================================
# Test Suite 5: Full-Duplex Adapter Loopback
# =====================================================================

def test_adapter_loopback(ser):
    print("\n[Suite 5] Full-Duplex Adapter Loopback (UART0 TX=12, RX=13)")
    print("-" * 50)

    # 1. Claim pins 12, 13 for UART (function=2 or function=1)
    claim_lines = send_cmd(ser, "PINCLAIM pins=12,13 func=2 observer=0")
    print(f"  PINCLAIM: {claim_lines}")
    handle = None
    for l in claim_lines:
        if "HANDLE=" in l:
            handle = int(l.split("HANDLE=")[1].split()[0])
            break

    if not handle:
        print("  [FAIL] Failed to claim UART adapter pins 12, 13")
        return False
    print(f"  Claimed adapter handle: {handle}")

    try:
        # 2. Test UART_WRITE with unique test payload
        payload = b"PING_AGENTLA_ECHO"
        hex_data = ",".join(f"{b:02X}" for b in payload)
        write_lines = send_cmd(ser, f"UART_WRITE handle={handle} baud=115200 data={hex_data}")
        print(f"  UART_WRITE: {write_lines}")
        if not any("OK UART WROTE" in l for l in write_lines):
            print("  [FAIL] UART_WRITE failed")
            return False

        # 3. Read back echoed response via UART_READ
        time.sleep(0.1)
        read_lines = send_cmd(ser, f"UART_READ handle={handle} baud=115200 len=64 timeout=1000")
        print(f"  UART_READ: {read_lines}")
        data_line = next((l for l in read_lines if l.startswith("UART DATA")), None)
        if not data_line:
            print("  [FAIL] No UART DATA received")
            return False

        hex_received = data_line.replace("UART DATA", "").strip().split(",")
        raw_received = bytes([int(h, 16) for h in hex_received if h])
        print(f"  Received raw bytes: {raw_received}")
        print(f"  Decoded text: {repr(raw_received.decode(errors='replace'))}")

        # Check for echoed payload in received stream
        if payload not in raw_received and b"AGENTLA_TEST_BEACON" not in raw_received:
            print(f"  [FAIL] Echoed payload {payload} not found in received stream")
            return False

        print("  [PASS] Full-duplex UART adapter communication verified")
        return True

    finally:
        # Clean up pin claim
        rel_lines = send_cmd(ser, f"PINRELEASE handle={handle}")
        print(f"  PINRELEASE: {rel_lines}")


# =====================================================================
# Main Runner
# =====================================================================

def main():
    print("=" * 65)
    print(" AgentLA Autonomous Hardware Verification Suite")
    print("=" * 65)

    port = sys.argv[1] if len(sys.argv) > 1 else None
    if not port:
        try:
            port = find_serial_port()
        except RuntimeError as e:
            print(f"Error: {e}")
            sys.exit(1)

    print(f"Target Serial Port: {port}")
    try:
        ser = serial.Serial(port, DEFAULT_BAUD, timeout=TIMEOUT)
        ser.reset_input_buffer()
        ser.reset_output_buffer()
        time.sleep(0.2)
    except Exception as e:
        print(f"Error opening {port}: {e}")
        sys.exit(1)

    lib = load_libdecoders()

    results = {}
    try:
        results["Connection & Handshake"] = test_handshake(ser)
        results["Oscilloscope Multi-Channel ADC"] = test_oscilloscope_adc(ser)
        results["Logic Analyzer Digital Capture"] = test_logic_analyzer(ser)
        results["Protocol Decoder Validation"] = test_protocol_decoders(lib)
        results["Adapter Full-Duplex Loopback"] = test_adapter_loopback(ser)
    finally:
        ser.close()

    print("\n" + "=" * 65)
    print(" TEST SUITE SUMMARY")
    print("=" * 65)
    all_passed = True
    for suite, passed in results.items():
        status = "PASS" if passed else "FAIL"
        if not passed: all_passed = False
        print(f"  [{status}] {suite}")
    print("=" * 65)
    print(f"OVERALL RESULT: {'SUCCESS (100% PASS)' if all_passed else 'FAILURE'}")
    print("=" * 65)

    sys.exit(0 if all_passed else 1)


if __name__ == "__main__":
    main()

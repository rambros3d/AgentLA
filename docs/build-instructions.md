# AgentLA Firmware Build Instructions

## Prerequisites

### Required Software

1. **CMake** (3.13 or later)
   ```bash
   # Ubuntu/Debian
   sudo apt install cmake

   # macOS (Homebrew)
   brew install cmake

   # Windows: Download from https://cmake.org/download/
   ```

2. **ARM GCC Toolchain**
   ```bash
   # Ubuntu/Debian
   sudo apt install gcc-arm-none-eabi

   # macOS (Homebrew)
   brew install --cask gcc-arm-embedded

   # Windows: Download from https://developer.arm.com/downloads/-/gnu-rm
   ```

3. **Python 3** (for picotool and UF2 conversion)
   ```bash
   # Usually pre-installed on Linux/macOS
   python3 --version
   ```

4. **Pico SDK** (downloaded automatically by CMake)
   - The build system uses `pico_sdk_import.cmake` to fetch the SDK automatically

### Optional Tools

- **VS Code** with Cortex-Debug extension for debugging
- **OpenOCD** or **Picoprobe** for SWD debugging
- **picotool** for UF2 manipulation (built automatically during build)

## Project Structure

```
AgentLA/
├── CMakeLists.txt              # Main CMake build file
├── agentla_build_settings.cmake # Build configuration
├── pico_sdk_import.cmake       # Pico SDK import script
├── firmware/
│   ├── src/
│   │   ├── main.c              # Main firmware entry point│       │   ├── main.c              # Main firmware entry point
│       │   ├── analyzer/           # Signal capture & decode
│       │   ├── adapter/            # I2C/UART adapter
│       │   ├── programmer/         # SWD/CH32 programming
│       │   ├── board_settings.h    # Board configuration
│       │   └── structs.h           # Data structures
│       └── pio/
│           ├── capture.pio         # PIO capture programs
│           ├── swd.pio             # SWD protocol
│           ├── swio.pio            # WCH SWIO protocol
│           ├── wch_sdi.pio         # WCH 1-wire SDI
│           └── loopback.pio        # Loopback test
├── sdk/                        # Python SDK (future)
├── docs/                       # Documentation
├── test/                       # Test scripts
└── build/                      # Build output (generated)
```

## Build Configuration

### Build Settings

Edit `agentla_build_settings.cmake` to configure build options:

```cmake
# Board: BOARD_PICO_2 (RP2350)
set(BOARD_TYPE "BOARD_PICO_2")

# Set to 1 to enable 400MHz overclock (extreme overvoltage!)
set(TURBO_MODE 0)

# Uncomment to debug
set(DEBUG_BUILD 1)
```

**Options:**
- `TURBO_MODE 0`: Normal 200MHz operation (recommended)
- `TURBO_MODE 1`: 400MHz overclock (requires voltage modification, may damage hardware)
- `DEBUG_BUILD 1`: Debug build with symbols, no optimization
- `DEBUG_BUILD 0`: Release build with optimization

### Board Selection

The firmware targets the **Raspberry Pi Pico 2** (RP2350) by default. This is set in `CMakeLists.txt`:

```cmake
set(PICO_BOARD pico2 CACHE STRING "Board type")
```

**Important:** This firmware is specifically designed for RP2350 and will NOT work on RP2040 (Pico 1).

## Build Steps

### 1. Clone/Setup Project

```bash
# If starting fresh
git clone <repository-url>
cd AgentLA

# If already cloned
cd AgentLA
```

### 2. Create Build Directory

```bash
mkdir -p build
cd build
```

### 3. Configure CMake

```bash
# Standard configuration
cmake ..

# Or with debug build
cmake -DDEBUG_BUILD=1 ..

# Or with turbo mode (use with caution!)
cmake -DTURBO_MODE=1 ..
```

**First build will automatically:**
- Download Pico SDK from GitHub
- Download and build picotool
- Download and build pioasm (PIO assembler)
- Set up all dependencies

This may take 5-10 minutes on first run.

### 4. Build Firmware

```bash
# From build directory
make -j$(nproc)
```

Or from project root:
```bash
cmake --build build -j$(nproc)
```

### 5. Build Output Files

After successful build, check `build/` directory for:

| File | Description |
|------|-------------|
| `AgentLA.uf2` | UF2 file for drag-and-drop flashing |
| `AgentLA.bin` | Binary firmware image |
| `AgentLA.hex` | Intel HEX format |
| `AgentLA.elf` | ELF with debug symbols |
| `AgentLA.dis` | Disassembly listing |

## Flashing Firmware

### Method 1: UF2 Drag-and-Drop (Recommended)

1. **Enter BOOTSEL mode:**
   - Hold BOOTSEL button on Pico 2
   - While holding, connect USB cable
   - Release button after connecting
   - Pico appears as USB mass storage device (RPI-RP2)

2. **Copy UF2 file:**
   ```bash
   # Linux
   cp build/AgentLA.uf2 /media/$USER/RPI-RP2/

   # macOS
   cp build/AgentLA.uf2 /Volumes/RPI-RP2/

   # Windows (PowerShell)
   Copy-Item build\AgentLA.uf2 D:\
   ```

3. **Device automatically reboots** into firmware

### Method 2: picotool

If you have picotool installed:

```bash
# Put device in BOOTSEL mode first
picotool load build/AgentLA.uf2
picotool reboot
```

### Method 3: OpenOCD (Debug Probe)

For development/debugging with SWD probe:

```bash
# Using picoprobe/Pico DAP
openocd -f interface/cmsis-dap.cfg -f target/rp2350.cfg \
  -c "program build/AgentLA.elf verify reset exit"
```

## Verifying Installation

1. Connect AgentLA to computer via USB
2. Open serial terminal (115200 baud, 8N1)
   - Linux: `screen /dev/ttyACM0 115200` or `minicom -D /dev/ttyACM0`
   - macOS: `screen /dev/cu.usbmodem* 115200`
   - Windows: PuTTY or Device Manager COM port
3. Type: `IDENTIFY`
4. Expected response:
   ```
   > IDENTIFY
   AGENTLA v1.0 RP2350 24CH 100MHZ
   FREQ:100000000
   BLASTFREQ:200000000
   BUFFER:393216
   CHANNELS:24
   ```

## Troubleshooting Build

### CMake Errors

**Error:** `Could not find PICO_SDK_PATH`
- Solution: Let CMake download SDK automatically, or set:
  ```bash
  export PICO_SDK_PATH=/path/to/pico-sdk
  ```

**Error:** `arm-none-eabi-gcc not found`
- Solution: Install ARM toolchain and ensure it's in PATH:
  ```bash
  export PATH=$PATH:/path/to/gcc-arm/bin
  ```

### Build Fails

**Error:** `pioasm not found`
- Solution: Build dependencies first:
  ```bash
  cd build
  cmake ..
  make pioasm
  make -j$(nproc)
  ```

**Error:** `pico_sdk_import.cmake not found`
- Solution: Ensure internet connection for first build (downloads SDK)

### Flashing Issues

**Device not appearing as USB storage:**
- Try different USB cable (data + power)
- Try different USB port
- Check if BOOTSEL button is working

**UF2 copy fails:**
- Ensure device is in BOOTSEL mode (RPI-RP2 drive visible)
- Try copying smaller chunks
- Format drive as FAT32 if corrupted

## Development Build Options

### Debug Build

For debugging with GDB:

```bash
cmake -DDEBUG_BUILD=1 ..
make -j$(nproc)
```

Debug features:
- No optimization (-O0)
- Full debug symbols
- UART stdio enabled
- USB stdio enabled

### Release Build

For production:

```bash
cmake -DDEBUG_BUILD=0 ..
make -j$(nproc)
```

Release features:
- Optimized (-Os)
- No debug symbols
- Binary type: copy_to_ram (faster execution)

### Turbo Mode

**WARNING:** 400MHz overclock requires voltage modification!

```bash
cmake -DTURBO_MODE=1 ..
make -j$(nproc)
```

Turbo mode:
- 400MHz system clock (vs 200MHz normal)
- Requires VREG voltage increase to 1.3V
- May reduce hardware lifespan
- Use at your own risk

## Cleaning Build

```bash
cd build
make clean
# Or remove entire build directory
rm -rf build
```

## Advanced: Custom Board Support

To add support for a custom RP2350 board:

1. Edit `firmware/src/agentla_board_settings.h`:
   ```c
   #define BOARD_NAME "Custom Board"
   #define INPUT_PIN_BASE 0  // Your pin base
   #define LED_IO 25         // Your LED pin
   ```

2. Edit `CMakeLists.txt`:
   ```cmake
   set(PICO_BOARD my_board)  # Or keep pico2 for compatible boards
   ```

3. Update pin mapping in `agentla_capture.c` if needed

## Resources

- [RP2350 Datasheet](https://datasheets.raspberrypi.com/rp2350/rp2350-datasheet.pdf)
- [Pico C/C++ SDK](https://datasheets.raspberrypi.com/pico/c-sdk/)
- [Pico 2 Getting Started](https://datasheets.raspberrypi.com/pico/getting-started-with-pico.pdf)
- [PIO Programming](https://datasheets.raspberrypi.com/rp2350/rp2350-programmers-model.pdf)
- [AgentLA Source](../firmware/)

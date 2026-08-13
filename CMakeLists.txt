cmake_minimum_required(VERSION 3.13)

set(CMAKE_C_STANDARD 11)
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

include(${CMAKE_CURRENT_LIST_DIR}/agentla_build_settings.cmake)

set(PICO_BOARD pico2 CACHE STRING "Board type")

if(TURBO_MODE)
    message(WARNING "WARNING!! Turbo mode enabled!")
    add_compile_definitions(TURBO_MODE)
endif()

include(pico_sdk_import.cmake)

project(AgentLA C CXX ASM)

# Build metadata
string(TIMESTAMP BUILD_TIMESTAMP "%Y-%m-%d %H:%M:%S")
add_compile_definitions(BUILD_TIMESTAMP="${BUILD_TIMESTAMP}")
# Get git hash at configure time
execute_process(
    COMMAND git log -1 --format=%h
    WORKING_DIRECTORY ${CMAKE_SOURCE_DIR}
    OUTPUT_VARIABLE GIT_HASH
    OUTPUT_STRIP_TRAILING_WHITESPACE
    ERROR_QUIET
    RESULT_VARIABLE GIT_RESULT
)
if(NOT GIT_RESULT EQUAL 0)
    set(GIT_HASH "unknown")
endif()
add_compile_definitions(GIT_HASH="${GIT_HASH}")

pico_sdk_init()

FILE(GLOB_RECURSE CSources firmware/src/*.c)
ADD_EXECUTABLE(AgentLA ${CSources})

pico_generate_pio_header(${PROJECT_NAME}
    ${CMAKE_CURRENT_LIST_DIR}/firmware/pio/capture.pio
)

pico_generate_pio_header(${PROJECT_NAME}
    ${CMAKE_CURRENT_LIST_DIR}/firmware/src/programmer/debugprobe/swd_probe.pio
)

pico_generate_pio_header(${PROJECT_NAME}
    ${CMAKE_CURRENT_LIST_DIR}/firmware/pio/wch_sdi.pio
)

pico_generate_pio_header(${PROJECT_NAME}
    ${CMAKE_CURRENT_LIST_DIR}/firmware/pio/loopback.pio
)

pico_generate_pio_header(${PROJECT_NAME}
    ${CMAKE_CURRENT_LIST_DIR}/firmware/pio/swio.pio
)

pico_generate_pio_header(${PROJECT_NAME}
    ${CMAKE_CURRENT_LIST_DIR}/firmware/src/programmer/rvswd/rvswd.pio
)

pico_set_program_name(AgentLA "AgentLA")
pico_set_program_version(AgentLA "1.0")
add_compile_definitions(FIRMWARE_VERSION="v1.0")

pico_enable_stdio_uart(AgentLA 0)
# pico_enable_stdio_usb(AgentLA 1)  # Disabled - using TinyUSB directly for CMSIS-DAP + CDC

if(NOT DEBUG_BUILD)
    # pico_set_binary_type(AgentLA copy_to_ram)  # Disabled: firmware too large for RAM (561KB > 512KB SRAM)
endif()

target_link_libraries(AgentLA
    pico_stdlib
    tinyusb_device
    tinyusb_board
    hardware_dma
    hardware_pio
    hardware_clocks
    hardware_flash
    hardware_exception
    hardware_adc
    hardware_vreg
    pico_multicore
    pico_base_headers
    cmsis_core
    hardware_i2c
    hardware_uart
)

pico_add_extra_outputs(AgentLA)

target_include_directories(AgentLA PRIVATE
    ${CMAKE_CURRENT_LIST_DIR}/firmware/src
    ${CMAKE_CURRENT_LIST_DIR}/firmware/pio
)

target_compile_definitions(AgentLA PUBLIC
    USBD_MANUFACTURER="AgentLA"
    USBD_PRODUCT="AGENTLA"
    USBD_VID=0x1209
    USBD_PID=0x414C
)
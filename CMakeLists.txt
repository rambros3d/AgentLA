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

pico_sdk_init()

FILE(GLOB CSources firmware/src/*.c)
ADD_EXECUTABLE(AgentLA ${CSources})

pico_generate_pio_header(${PROJECT_NAME}
    ${CMAKE_CURRENT_LIST_DIR}/firmware/pio/agentla_capture.pio
)

pico_set_program_name(AgentLA "AgentLA")
pico_set_program_version(AgentLA "1.0")
add_compile_definitions(FIRMWARE_VERSION="v1.0")

pico_enable_stdio_uart(AgentLA 1)
pico_enable_stdio_usb(AgentLA 1)

if(NOT DEBUG_BUILD)
    pico_set_binary_type(AgentLA copy_to_ram)
endif()

target_link_libraries(AgentLA
    pico_stdlib
    hardware_dma
    hardware_pio
    hardware_clocks
    hardware_flash
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
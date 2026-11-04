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

pico_generate_pio_header(${PROJECT_NAME}
    ${CMAKE_CURRENT_LIST_DIR}/firmware/pio/i3c.pio
)

pico_generate_pio_header(${PROJECT_NAME}
    ${CMAKE_CURRENT_LIST_DIR}/firmware/pio/neopixel.pio
)

pico_set_program_name(AgentLA "AgentLA")
pico_set_program_version(AgentLA "1.0")
add_compile_definitions(FIRMWARE_VERSION="v1.0")

pico_enable_stdio_uart(AgentLA 0)
# pico_enable_stdio_usb(AgentLA 1)  # Disabled - using TinyUSB directly for CMSIS-DAP + CDC

if(NOT DEBUG_BUILD)
    # pico_set_binary_type(AgentLA copy_to_ram)  # Disabled: firmware too large for RAM (561KB > 512KB SRAM)
endif()

target_compile_definitions(AgentLA PRIVATE
    PICO_ENABLE_USB_RESET_VIA_VENDOR_INTERFACE=1
    PICO_USB_RESET_SUPPORT_MS_OS_20_DESCRIPTOR=0
    PICO_USB_RESET_INCLUDE_DEFAULT_APP_DRIVER_CB=1
)

target_link_libraries(AgentLA
    pico_stdlib
    tinyusb_device
    tinyusb_board
    pico_lwip
    pico_lwip_http
    pico_httpd_content
    pico_rand
    hardware_dma
    hardware_pio
    hardware_clocks
    hardware_flash
    hardware_exception
    hardware_adc
    hardware_pwm
    hardware_vreg
    pico_multicore
    pico_base_headers
    cmsis_core
    hardware_i2c
    hardware_uart
)

# HTTP content embedding - Dynamic Board Assets & WebUI
find_package(Python3 REQUIRED COMPONENTS Interpreter)
set(BOARD_ID "pico2" CACHE STRING "Target board identifier (pico2, bakuprobe)")
set(WEB_STAGED_DIR "${CMAKE_CURRENT_BINARY_DIR}/web_staged")
set(GENERATED_BOARD_HEADER "${CMAKE_CURRENT_SOURCE_DIR}/firmware/src/generated/board_config.h")

add_custom_command(
    OUTPUT 
        ${GENERATED_BOARD_HEADER}
        ${WEB_STAGED_DIR}/board.json
        ${WEB_STAGED_DIR}/logo.svg
        ${WEB_STAGED_DIR}/pinout.svg
        ${WEB_STAGED_DIR}/favicon.svg
        ${WEB_STAGED_DIR}/index.html
        ${WEB_STAGED_DIR}/app.js
        ${WEB_STAGED_DIR}/style.css
        ${WEB_STAGED_DIR}/wallpaper.svg
        ${WEB_STAGED_DIR}/mcp.json
        ${WEB_STAGED_DIR}/ok.json
        ${WEB_STAGED_DIR}/apps.html
        ${WEB_STAGED_DIR}/agentla-bridge.js
        ${WEB_STAGED_DIR}/themes.css
        ${WEB_STAGED_DIR}/themes.json
    COMMAND ${CMAKE_COMMAND} -E make_directory ${WEB_STAGED_DIR}
    COMMAND ${Python3_EXECUTABLE} ${CMAKE_CURRENT_SOURCE_DIR}/tools/compile_themes.py
        --themes-dir ${CMAKE_CURRENT_SOURCE_DIR}/themes
        --output-css ${CMAKE_CURRENT_SOURCE_DIR}/firmware/html/themes.css
        --output-json ${CMAKE_CURRENT_SOURCE_DIR}/firmware/html/themes.json
    COMMAND ${Python3_EXECUTABLE} ${CMAKE_CURRENT_SOURCE_DIR}/tools/gen_board_assets.py
        --board ${BOARD_ID}
        --boards-dir ${CMAKE_CURRENT_SOURCE_DIR}/boards
        --base-html-dir ${CMAKE_CURRENT_SOURCE_DIR}/firmware/html
        --output-header ${GENERATED_BOARD_HEADER}
        --output-web-dir ${WEB_STAGED_DIR}
    DEPENDS
        ${CMAKE_CURRENT_SOURCE_DIR}/tools/compile_themes.py
        ${CMAKE_CURRENT_SOURCE_DIR}/themes/emerald.css
        ${CMAKE_CURRENT_SOURCE_DIR}/themes/forest.css
        ${CMAKE_CURRENT_SOURCE_DIR}/themes/caramellatte.css
        ${CMAKE_CURRENT_SOURCE_DIR}/tools/gen_board_assets.py
        ${CMAKE_CURRENT_SOURCE_DIR}/boards/${BOARD_ID}/board.json
        ${CMAKE_CURRENT_SOURCE_DIR}/boards/${BOARD_ID}/logo.svg
        ${CMAKE_CURRENT_SOURCE_DIR}/boards/${BOARD_ID}/favicon.svg
        ${CMAKE_CURRENT_SOURCE_DIR}/boards/${BOARD_ID}/pinout.svg
        ${CMAKE_CURRENT_SOURCE_DIR}/firmware/html/index.html
        ${CMAKE_CURRENT_SOURCE_DIR}/firmware/html/apps.html
        ${CMAKE_CURRENT_SOURCE_DIR}/firmware/html/agentla-bridge.js
        ${CMAKE_CURRENT_SOURCE_DIR}/firmware/html/app.js
        ${CMAKE_CURRENT_SOURCE_DIR}/firmware/html/style.css
        ${CMAKE_CURRENT_SOURCE_DIR}/firmware/html/wallpaper.svg
        ${CMAKE_CURRENT_SOURCE_DIR}/firmware/html/mcp.json
        ${CMAKE_CURRENT_SOURCE_DIR}/firmware/html/ok.json
    COMMENT "Generating board configuration and staging web assets for board '${BOARD_ID}'"
    VERBATIM
)

pico_add_library(pico_httpd_content NOFLAG)
pico_set_lwip_httpd_content(pico_httpd_content INTERFACE
    ${WEB_STAGED_DIR}/index.html
    ${WEB_STAGED_DIR}/apps.html
    ${WEB_STAGED_DIR}/agentla-bridge.js
    ${WEB_STAGED_DIR}/app.js
    ${WEB_STAGED_DIR}/style.css
    ${WEB_STAGED_DIR}/wallpaper.svg
    ${WEB_STAGED_DIR}/mcp.json
    ${WEB_STAGED_DIR}/ok.json
    ${WEB_STAGED_DIR}/board.json
    ${WEB_STAGED_DIR}/logo.svg
    ${WEB_STAGED_DIR}/favicon.svg
    ${WEB_STAGED_DIR}/pinout.svg
    ${WEB_STAGED_DIR}/themes.css
    ${WEB_STAGED_DIR}/themes.json
)
pico_add_extra_outputs(AgentLA)


target_include_directories(AgentLA PRIVATE
    ${CMAKE_CURRENT_LIST_DIR}/firmware/src
    ${CMAKE_CURRENT_LIST_DIR}/firmware/src/generated
    ${CMAKE_CURRENT_LIST_DIR}/firmware/pio
    /home/sun/sandbox/fedora/.pico-sdk/src/rp2_common/pico_lwip/include
    /home/sun/sandbox/fedora/.pico-sdk/lib/tinyusb/lib/networking
)

target_compile_definitions(AgentLA PUBLIC
    USBD_MANUFACTURER="AgentLA"
    USBD_PRODUCT="AGENTLA"
    USBD_VID=0x1209
    USBD_PID=0x414C
)


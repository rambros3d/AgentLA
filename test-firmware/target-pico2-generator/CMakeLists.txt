cmake_minimum_required(VERSION 3.13)

set(PICO_BOARD pico2 CACHE STRING "Board type")
set(PICO_PLATFORM rp2350-arm-s CACHE STRING "Platform type")

include(/home/sun/sandbox/fedora/.pico-sdk/pico_sdk_init.cmake)

project(target_pico2_generator C CXX ASM)
set(CMAKE_C_STANDARD 11)
set(CMAKE_CXX_STANDARD 17)

pico_sdk_init()

add_executable(target_pico2_generator
    main.c
)

target_link_libraries(target_pico2_generator
    pico_stdlib
    hardware_pwm
    hardware_uart
    hardware_gpio
    hardware_i2c
    hardware_clocks
    hardware_pio
    pico_i2c_slave
)

pico_add_extra_outputs(target_pico2_generator)

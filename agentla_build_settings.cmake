# AgentLA Build Settings
# Board: BOARD_PICO_2 (RP2350)
set(BOARD_TYPE "BOARD_PICO_2")

# Custom Board Profile: Set to 1 to target the custom BakuProbe PCB (RP2350B QFN-80)
option(BAKUPROBE_BOARD "Build firmware for BakuProbe custom RP2350B PCB" OFF)
option(SILINGPROBE_BOARD "Deprecated alias for BAKUPROBE_BOARD" OFF)
if(BAKUPROBE_BOARD OR SILINGPROBE_BOARD OR AGENTPROBE_BOARD)
    add_compile_definitions(BAKUPROBE_BOARD=1)
    add_compile_definitions(SILINGPROBE_BOARD=1)
endif()

# Set to 1 to enable 400MHz overclock (extreme overvoltage!)
set(TURBO_MODE 0)

# Uncomment to debug
set(DEBUG_BUILD 1)

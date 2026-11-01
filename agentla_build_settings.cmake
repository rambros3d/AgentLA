# AgentLA Build Settings
# Board: BOARD_PICO_2 (RP2350)
set(BOARD_TYPE "BOARD_PICO_2")

# Custom Board Profile: Set to 1 to target the custom AgentProbe PCB (RP2350B QFN-80)
option(AGENTPROBE_BOARD "Build firmware for AgentProbe custom RP2350B PCB" OFF)
if(AGENTPROBE_BOARD)
    add_compile_definitions(AGENTPROBE_BOARD=1)
endif()

# Set to 1 to enable 400MHz overclock (extreme overvoltage!)
set(TURBO_MODE 0)

# Uncomment to debug
set(DEBUG_BUILD 1)

## Context
The Rust branch already has pin-owner, REST shell, and bridge REST endpoints; what is missing is bus-driving behavior on the flex pins.

## Goals / Non-Goals
Goals: I2C 100k ACK writes on 16/17; SLCAN ASCII command routing; 4-8 px NeoPixel on a released flexible pin; MIPI I3C SDR CCC read/write.
Non-goals: advanced bus arbitration/internal starvation.

## Decisions
1. Fixed flex-pin pool mapping; task settles all state under one static up.
2. REST JSON return shapes unchanged from Phase4.
3. No external deps; PIO required for WS2812. Keep bus protocols under adapter.

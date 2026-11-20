# Design: BareFOC Studio Identity & Calibration Wizard Update

## UI Layout & Component Architecture

1. **Header Identity Strip**:
   - Above the sub-toolbar or inside the connection badge:
   - Silicon UID pill (`text-xs font-mono`, e.g. `UID: 340028001751323337373334`).
   - Board Name input (`input input-bordered input-xs font-mono`, 15-char max).
   - Board Node ID input (`input input-bordered input-xs font-mono w-14`, 0..127).

2. **Motor Selector Tabs & Joint Identity**:
   - Motor tabs `M0..M3` display the custom joint name (e.g. `M0: shoulder_pitch`, `M1: shoulder_roll`) when available.
   - Sub-panel displays editable Joint Address (`input-xs`, 0..127 clamped) and Joint Name (15 chars) with an `Apply Identity` button writing to `REG_JOINTx_ADDR` and `REG_JOINTx_NAME`.

3. **MultiBus Register Table**:
   - Update `MultiBusReg` dictionary in JavaScript with:
     ```js
     const MultiBusReg = {
       // Identity
       HARDWARE_UID: 0x04,
       BOARD_NODE_ID: 0x10,
       BOARD_NAME: 0x11,
       JOINT_BASE: 0x20, // 16B per joint: ADDR @ +0x00, NAME @ +0x01
       
       // 4-Axis Motor Bank (0x60..0xDF, 32B per motor)
       MOTOR_BANK_BASE: 0x60,
       MOTOR_POLE_PAIRS_OFFSET: 0x00,
       MOTOR_PHASE_R_OFFSET: 0x02,
       MOTOR_PHASE_L_OFFSET: 0x06,
       MOTOR_KV_OFFSET: 0x0A,
       MOTOR_ZERO_ELEC_OFFSET: 0x0E,
       MOTOR_SENSOR_DIR_OFFSET: 0x12,
       MOTOR_SENSOR_TYPE_OFFSET: 0x13,
       MOTOR_VOLT_ALIGN_OFFSET: 0x14,
       MOTOR_HOMING_OFFSET: 0x18,
       
       // Calibration
       CALIB_CMD: 0xB0,
       // ...
     };
     ```

4. **Automated Calibration Wizard Strip**:
   - Status badge shows `ARMED` (success) or `DISARMED_SAFE` (warning/neutral).
   - Electrical Zero Angle readout displays measured radians / degrees, or `Uncalibrated`.
   - Button `1. Run ALIGN`: writes `0x01` to `0xB0`, monitors progress, and on completion updates badge to `Armed & Saved (Flash)`.
   - Button `Set Origin (ZERO)`: writes `0x04` to `0xB0` to zero mechanical position.

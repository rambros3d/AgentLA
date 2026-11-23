adapter driver cmsis-dap
cmsis_dap_vid_pid 0x1209 0x414d
transport select swd
adapter speed 1000
set CPUTAPID 0x6ba02477
source [find target/stm32u5x.cfg]

init
halt

# 1. Un-gate DBGMCU and freeze watchdogs
mww 0x44024004 0x00000006
mww 0x44024008 0x00001800

# 2. Switch to Secure AP access
stm32x5x_ahb_ap_secure_access

# 3. Load patched algorithm to 0x20000000
load_image /tmp/algo_patched.bin 0x20000000 bin

# 4. Load full bootloader to 0x20000400
load_image /home/sun/Filelink/Firmwares/bareBoot/artifacts/bareboot-stm32h503-i3c-i2c.bin 0x20000400 bin

# 5. Erase Sector 0 (cmd = 1, arg1 = 0x08000000)
reg r0 1
reg r1 0x08000000
reg r2 0
reg r3 0
reg sp 0x20006000
reg lr 0x20001ffd
reg pc 0x20000000
reg xpsr 0x01000000
mww 0x20001ffc 0xbe00be00
resume
wait_halt 5000
set res_erase [reg r0]
echo "Erase result: $res_erase"

# 6. Program bootloader into Sector 0 (cmd = 2, arg1 = 0x08000000, arg2 = 0x20000400, arg3 = 3952)
reg r0 2
reg r1 0x08000000
reg r2 0x20000400
reg r3 3952
reg sp 0x20006000
reg lr 0x20001ffd
reg pc 0x20000000
reg xpsr 0x01000000
resume
wait_halt 5000
set res_prog [reg r0]
echo "Program result: $res_prog"

# 7. Read back first 16 bytes of flash
mdw 0x08000000 4

shutdown

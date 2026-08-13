.section .text
.global _start
.equ DONE_BASE, 0x20001000
.equ KEYR,     0x40022004
.equ OBKEYR,   0x40022008
.equ STATR,    0x4002200C
.equ CTLR,     0x40022010
.equ MODEKEYR, 0x40022024
.equ OB_BASE,  0x1FFFF800
.equ KEY1,     0x45670123
.equ KEY2,     0xCDEF89AB

_start:
    csrci mstatus, 8        # clear MIE
    csrci mie, 0            # disable all interrupt sources

    la t0, DONE_BASE
    sw zero, 0(t0)          # done_flag = 0
    li t1, 1
    sw t1, 4(t0)            # status = 1 (started)

    # ---- unlock flash (KEYR, OBKEYR, MODEKEYR) ----
    la t2, KEYR
    li t3, KEY1
    sw t3, 0(t2)
    li t3, KEY2
    sw t3, 0(t2)
    la t2, OBKEYR
    li t3, KEY1
    sw t3, 0(t2)
    li t3, KEY2
    sw t3, 0(t2)
    la t2, MODEKEYR
    li t3, KEY1
    sw t3, 0(t2)
    li t3, KEY2
    sw t3, 0(t2)

    # ---- verify unlock: (CTLR & 0x8080) == 0 ----
    la t2, CTLR
    lw t3, 0(t2)
    li t4, 0x8080
    and t3, t3, t4
    bnez t3, fail

    # ---- option byte erase ----
    li t3, 0x20             # OPTER
    sw t3, 0(t2)
    li t3, 0x60             # OPTER | STRT
    sw t3, 0(t2)
    call wait_flash          # t4: 0=ok 1=timeout
    bnez t4, fail

    # ---- program option bytes (RDPR first, triggers auto mass erase) ----
    li t3, 0x10             # OPTPG
    sw t3, 0(t2)
    la t5, OB_BASE
    li t3, 0x5AA5
    sh t3, 0(t5)            # RDPR
    call wait_flash

    # re-unlock OBKEYR before subsequent option writes
    la t2, OBKEYR
    li t3, KEY1
    sw t3, 0(t2)
    li t3, KEY2
    sw t3, 0(t2)
    la t2, CTLR
    li t3, 0x10             # OPTPG
    sw t3, 0(t2)

    li t3, 0xC03F
    sh t3, 2(t5)            # USER
    call wait_flash
    li t3, 0xFF00
    sh t3, 4(t5)            # WRPR0
    call wait_flash
    li t3, 0xFF00
    sh t3, 6(t5)            # WRPR1
    call wait_flash
    li t3, 0x00FF
    sh t3, 8(t5)            # WRPR2
    call wait_flash
    li t3, 0x00FF
    sh t3, 10(t5)           # WRPR3
    call wait_flash
    li t3, 0x00FF
    sh t3, 12(t5)           # DATA0
    call wait_flash
    li t3, 0x00FF
    sh t3, 14(t5)           # DATA1
    call wait_flash

    # clear OPTPG
    li t3, 0
    sw t3, 0(t2)

    li t1, 2
    sw t1, 0(t0)            # done = 2 (success)
    ebreak                  # halt the core so debugger can detect completion
    j forever

fail:
    li t1, 0xFF
    sw t1, 0(t0)            # done = 0xFF
    ebreak
    j forever

wait_flash:
    la t1, STATR
    li t6, 40000000         # timeout count (~30s at 8MHz)
wf_loop:
    lw t4, 0(t1)
    andi t4, t4, 3          # BSY | WRBSY
    beqz t4, wf_ok
    addi t6, t6, -1
    bnez t6, wf_loop
    li t4, 1
    ret
wf_ok:
    li t4, 0
    ret

forever:
    j forever
# stm32f1-swd-flash-fixes

STM32F1 flash driver fixes (page erase, halfword programming, error handling) and Cortex-M SWD connect sequence fixes, based on BMP reference comparison. Root cause: FLASH ERASE runs mass erase instead of page erase; FLASH WRITE uses 32-bit accesses on F1 which requires halfwords; write loop never checks PGERR; SWD connect tries dormant sequence before JTAG-to-SWD for SWJ-DP targets.

Status (Aug 11, 2026): waveform debugging via the probe's own logic analyzer found and fixed two additional defects (analyzer pin-base bug, SWD CAPTURE timing race). One **critical open issue** gates all flash work: PIO1 SM0 appears stuck generating a uniform ~39 kHz clock on SWCLK while never driving SWDIO (IDCODE read error 5), suspected to be a FIFO/autopull desync in swd_write_bits()/swd_transfer().

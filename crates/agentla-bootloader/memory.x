MEMORY {
    FLASH : ORIGIN = 0x10000000, LENGTH = 8K
    RAM   : ORIGIN = 0x20000000, LENGTH = 16K
}

SECTIONS {
    .start_block 0x10000138 : {
        KEEP(*(.start_block));
    } > FLASH
} INSERT BEFORE .text;

_stext = 0x10000200;

OUTPUT_ARCH(aarch64)
ENTRY(_start)
BASE_ADDRESS = 0xFFFF000000080000;


SECTIONS
{
    . = BASE_ADDRESS;
    skernel = .;

    stext = .;
    .text : {
        KEEP(*(.text.entry))
        *(.text .text.*)
    }

    . = ALIGN(4K);
    sinit = .;
    .init : {
        PROVIDE(early_init = .);
        KEEP(*(.init.early))
        PROVIDE(rest_init = .);
        KEEP(*(.init.rest))
        PROVIDE(init = .);
        KEEP(*(.init))
    }
    PROVIDE(einit = .);

    . = ALIGN(4K);
    etext = .;
    srodata = .;
    .rodata : {
        *(.rodata .rodata.*)
        *(.srodata .srodata.*)
    }

    . = ALIGN(4K);
    erodata = .;
    sdata = .;
    .data : {
        *(.data .data.*)
        *(.sdata .sdata.*)
    }

    . = ALIGN(4K);
    edata = .;
    .bss : {
        *(.bss.stack)
        sbss = .;
        *(.bss .bss.*)
        *(.sbss .sbss.*)
    }

    . = ALIGN(4K);
    ebss = .;
    ekernel = .;

    /DISCARD/ : {
        *(.eh_frame)
    }
}
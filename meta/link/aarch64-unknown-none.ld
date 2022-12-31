/* Partially taken from https://lowenware.com/blog/aarch64-bare-metal-program-in-rust/ */
ENTRY(_start)
MEMORY {
       RAM : ORIGIN = 0x40100000, LENGTH = 0x10000000
}
SECTIONS {
         . = 0x40100000;
         .text.boot : { *(.text.boot) } > RAM
         .text : { *(.text) }
         .data : { *(.data) }
         .rodata : { *(.rodata) }
         .bss : { *(.bss COMMON) }

         . = ALIGN(8);
         . += 0x4000;
         LD_STACK_PTR = .;
}

/* Partially taken from https://lowenware.com/blog/aarch64-bare-metal-program-in-rust/ */
ENTRY(_start)
EXTERN(PROGRAM_START PROGRAM_END PROGRAM_SIZE)
SECTIONS {
         . = 0x40100000;
         PROGRAM_START = .;
         .text.boot : { *(.text.boot) }
         .text : { *(.text) }
         .data : { *(.data) }
         .rodata : { *(.rodata) }
         .bss : { *(.bss COMMON) }

         . = ALIGN(8);
         . += 0x4000;
         LD_STACK_PTR = .;
         PROGRAM_END = .;
         PROGRAM_SIZE = PROGRAM_END - PROGRAM_START;
}

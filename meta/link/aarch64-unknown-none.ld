ENTRY(_start)
EXTERN(entry)
SECTIONS {
         . = 0x40000000;
         .boot . : { *(.boot) }
         .text : { *(.text) }
         .data : { *(.data) }
         .bss : { *(.bss COMMON) }
         . = ALIGN(8);
         . += 0x1000;
         stack_top = .;
}

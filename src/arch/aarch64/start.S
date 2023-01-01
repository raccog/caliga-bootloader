// Taken from https://lowenware.com/blog/aarch64-bare-metal-program-in-rust/
.globl _start
.extern LD_STACK_PTR

.section ".text.boot"

_start:
    ldr x30, =LD_STACK_PTR
    mov sp, x30
    ldr x9, =0x09000000
    mov x10, #65
    str x10, [x9]
    bl qemu_entry
    b .

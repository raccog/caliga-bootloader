#!/bin/bash

# This does load the elf image properly (confirmed with the qemu monitor), the program does not correctly output to the
# virtual PL011 UART device. I want to debug the program with GDB to figure out why. Also, for some reason, the PC
# of the VM ends up at 0x200, so there might be an instruction thats branching to the wrong address.

# Run elf image with generic loader https://qemu.readthedocs.io/en/latest/system/generic-loader.html#setting-a-cpu-s-program-counter
# Starts at address 0x40100000 to avoid collisions with the dtb. Not sure why, but the dtb is loaded at 0x40000000-0x40100000
qemu-system-aarch64 -s -machine virt -cpu cortex-a57 -device loader,file=target/aarch64-unknown-none/debug/caliga-qemu-aarch64,addr=0x40100000,cpu-num=0 -S -serial stdio


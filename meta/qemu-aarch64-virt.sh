#!/bin/bash

set -eu

# Run elf image with generic loader https://qemu.readthedocs.io/en/latest/system/generic-loader.html#setting-a-cpu-s-program-counter
# Starts at address 0x40100000 to avoid collisions with the dtb. Not sure why, but the dtb is loaded at 0x40000000-0x40100000
qemu-system-aarch64 \
    -machine virt \
    -cpu cortex-a57 \
    -device loader,file=target/aarch64-unknown-none/debug/caliga-aarch64-qemu,addr=0x40100000,cpu-num=0 \
    -nographic $QEMU_EXTRA_ARGS

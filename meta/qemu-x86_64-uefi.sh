#!/bin/bash

# Arguments:
#   $1 - The ovmf image path
#   $2 - The disk image path

qemu-system-x86_64 \
    -drive file=$1,if=pflash,format=raw,readonly=on \
    -drive file=$2,format=raw \
    -cpu qemu64 \
    -net none \
    -serial stdio

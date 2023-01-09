#!/bin/bash

set -eu

qemu-system-x86_64 \
    -drive file=$OVMF_DST,if=pflash,format=raw,readonly=on \
    -drive file=$DISK_IMG,format=raw \
    -cpu qemu64 \
    -net none \
    -serial stdio $QEMU_EXTRA_ARGS

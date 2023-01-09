#!/bin/bash

set -eu

echo "Caching UEFI firmware"

# All known ovmf locations
OVMF_LOCATIONS="/usr/share/edk2/x64/OVMF.fd
    /usr/share/edk2-ovmf/OVMF.fd
    /usr/share/ovmf/OVMF.fd"

# Search for OVMF firmware in known locations
for ovmf in $OVMF_LOCATIONS; do
    if [[ -f "$ovmf" ]]; then
        # Cache firmware if it's found
        cp -v "$ovmf" "$OVMF_DST"
        exit 0
    fi
done

echo "Could not find UEFI firmware to cache... Compiling instead" >&2

exit 1

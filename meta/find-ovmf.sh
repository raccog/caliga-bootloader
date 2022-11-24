#!/bin/bash

# Try to use environment variable
if [[ -f "$OVMF" ]]; then
    echo "$OVMF"
    exit 0
fi

OVMF_LOCATIONS="/usr/share/ovmf/x64/OVMF.fd /usr/share/ovmf/OVMF.fd"

# Search for OVMF firmware in known locations
# TODO: Maybe use the `find` command if OVMF cannot be found
for ovmf in $OVMF_LOCATIONS; do
    if [[ -f "$ovmf" ]]; then
        echo "$ovmf"
        exit 0
    fi
done

echo "Error: Could not find OVMF firmware. Please install with your package manager or export the variable 'OVMF' as the OVMF file path" >&2

exit 1

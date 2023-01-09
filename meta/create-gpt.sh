#!/bin/bash

set -eu

if which parted; then
    # Use parted on Linux systems
	dd if=/dev/zero of=$DISK_IMG bs=1M count=66
	parted -s "$DISK_IMG" mklabel gpt
	parted -s "$DISK_IMG" mkpart ESP fat32 2048s 100%
	parted -s "$DISK_IMG" set 1 esp on
elif which hdiutil; then
    # Use hdiutil on MacOS systems
    #
    # This is kind of excessive, as it creates a FAT32 file system that is
    # eventually overwritten. But this is the best method I've found on MacOS
    # to create a GPT partition table in a normal file. I also don't like how
    # the image needs to have a .dmg extension to be created by hdiutil.
    #
    # Maybe search for a better partitioning method?
    DMG="$DISK_IMG.dmg"
    if [[ -f "$DMG" ]]; then
        rm "$DMG"
    fi
    hdiutil create -size 66m -fs FAT32 -volname ESP -layout GPTSPUD "$DMG"
    mv "$DMG" "$DISK_IMG"
else
    echo "ERROR: Neither 'parted' or 'hdiutil' are installed. Cannot create a gpt partition image with meta/create-gpt.sh" >&2
    exit 2
fi

exit 0

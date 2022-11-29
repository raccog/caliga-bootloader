#!/bin/bash

if [[ -z "$1" ]]; then
    echo "ERROR: meta/create-gpt.sh was called with an empty filename" >&2
    exit 1
fi

if which parted; then
    # Use parted on Linux systems
	dd if=/dev/zero of=$1 bs=1M count=66
	parted -s "$1" mklabel gpt
	parted -s "$1" mkpart ESP fat32 2048s 100%
	parted -s "$1" set 1 esp on
elif which hdiutil; then
    # Use hdiutil on MacOS systems
    #
    # This is kind of excessive, as it creates a FAT32 file system that is
    # eventually overwritten. But this is the best method I've found on MacOS
    # to create a GPT partition table in a normal file. I also don't like how
    # the image needs to have a .dmg extension to be created by hdiutil.
    #
    # Maybe search for a better partitioning method?
    DMG="$1.dmg"
    if [[ -f "$DMG" ]]; then
        rm "$DMG"
    fi
    hdiutil create -size 66m -fs FAT32 -volname ESP -layout GPTSPUD "$DMG"
    mv "$DMG" "$1"
else
    echo "ERROR: Neither 'parted' or 'gpt' are installed. Cannot create a gpt partition with meta/create-gpt.sh" >&2
    exit 2
fi

exit 0

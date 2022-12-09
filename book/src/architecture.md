# Architecture

My main goal for this boot loader is mostly personal. I am pretty much just doing this because I enjoy learning about and trying to redesign the boot process for modern operating systems.

That being said, I currently have a few high-level goals for this boot loader (for my own use):

* Implement Multiboot2 and load Linux kernels
* Mess around with designing a custom boot protocol for my future hobby operating systems (if I ever get that far :P)

The chapter called [How Does the Boot Loader Work?](./how-does-it-work.md) explains more of my thoughts on this boot loader's design decisions.

## Chosen CPU Targets

I have chosen a list of targets; mostly based on what computers I have lying around my house.

Here is a list of targets I hope to have this boot loader running on:

* x86-64 (any UEFI firmware)
* aarch64 (RPi3/4)

This list will most likely change in the future, but it's a good starting point. With 2 targets, I will be forced to design the boot loader to be cross-platform without needing to support too many platforms from the start.

## Architecture-Specific Parts

In this boot loader, the architecture-specific parts will run as much architecture-dependent code at the beginning of the boot process. These parts will craft architecture-common data structures and interfaces that can be passed to the common part of the boot loader.

Some examples of code that needs to run in this part are:

* Get memory map
* Read kernel/initramfs from disk
* Setup terminal device (UART, VGA, etc.)
* Setup CPU timer

## Architecture-Common Part

Everything in the common part needs to be portable for every supported architecture. This mostly includes data structure APIs that are architecture-independent.

Some examples of code that needs to run in this part are:

* ELF parser
* Ustar archive reader
* Module loader


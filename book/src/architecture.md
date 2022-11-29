# Architecture

The main goal of this boot loader is to setup a common environment for my custom OS on multiple CPU targets. NOTE: The custom OS mentioned does not exist yet :) ...

ANOTHER NOTE: This documentation is mostly just a scratch-pad for my ideas on how to structure this boot loader. As such, it is designed with the intention of continuously re-organizing and re-writing this information.

## Chosen CPU Targets

I have chosen a list of targets based on what computers I have lying around my house.

Here is a list of targets I hope to have this boot loader running on:

* x86-64 (UEFI)
* aarch64 (RPi3/4, UEFI)
* aarch32 (rp2040?)

## How Does the Boot Loader Work?

NOTE: Currently, (as of Nov 28, 2022) the ideas discussed in this section are not implemented yet.

In a high level, the boot loader's single job is to start running an operating system when the computer is turned on. (TODO: expand this sentence?)

However, this single job easily becomes complex when multiple CPU architectures and firmware types are involved. (TODO: expand on firmware types?)

A common solution to this problem is to separate the boot loader into two parts; one architecture-specific and the other common to all architectures. Many boot loaders use this model, some inspirations for this project are GRUB, Limine, and Redox OS.

### Architecture-Specific Part

In this boot loader, the architecture-specific part will run as much architecture-dependent code at the beginning of the boot process. This part will craft architecture-common data structures that can be passed to the common part of the boot loader.

Some examples of code that needs to run in this part are:

* Get memory map
* Read kernel/initramfs from disk
* Setup terminal device (UART, VGA, etc.)
* Setup CPU timer

### Architecture-Common Part

Everything in the common part needs to be portable for every supported architecture. This mostly includes data structure APIs that are architecture-independent.

Some examples of code that needs to run in this part are:

* ELF parser
* Ustar archive reader
* Module loader


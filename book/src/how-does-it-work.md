# How Does the Boot Loader Work?

NOTE: Currently, (as of Dec 9, 2022) the ideas discussed in this section are not implemented yet. 

These notes are from my perspective, having a little experience with working on boot loaders. I don't have more than a year of experience on this topic, so take all the information here with a large grain of salt. Or maybe a boulder of salt :)

## Common Tasks

In a high-level, the boot loader's job is to start running an operating system when the computer is turned on. This high-level job is composed of many low-level tasks, for example:

* Set up a memory map
* Set up a stack
* Load kernel into memory
* Set up terminal device for output (UART, VGA, etc.)
* Set up devices that are needed at boot-time

There are lots of other tasks not listed here that a boot loader needs to do before it can pass control to the loaded kernel.

## Compile-time vs Run-time Decisions

Many of these tasks are architecture or firmware-dependent; they work differently (or not at all) depending on the architecture, firmware, extensions, and connected devices of the computer. This makes the boot loader's job much more difficult, as it needs to dynamically respond to each of these possibilities.

Some of these architecture-dependent parts can be extracted into compile-time decisions. For example, if you know that you want the boot loader to be set up to only run on a Raspberry Pi 4, you can remove any UEFI-related code at compile-time, as you know it will never be needed.

However, some decisions need to be made dynamically. One example of this is the output terminal. Which output to use can usually be chosen in the config file, which is read at run-time. Also, the output devices connected to the computer can always vary. How will the boot loader respond when the user chose a serial terminal in the config, but there is no serial device connected to the compuer? These questions are everywhere within the dynamic decisions a boot loader needs to make, and the answers are often unclear.

## Conclusions

In my opinion, this makes the design of a boot loader very creative and enjoyable. Some of these questions can be answered by looking to the boot loaders of the past, but their answers to these questions might not be as good as they could be. I find it fun to think about how the design choices made in existing boot loaders are great the way they are and also how they could possibly be improved. That is the main point of this boot loader; to explore these questions and have fun doing so.

One solution other boot loaders use is to separate the boot loader code into multiple parts; one part common to all architectures and the other parts are architecture/firmware-dependent and can be removed at compile-time. Many cross-platform boot loaders use this model; some inspirations for this project are GRUB, Limine, and the Redox OS boot loader.


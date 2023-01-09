# The Caliga Bootloader

I am designing a multi-architecture bootloader to use in my hobby OS projects.

Currently working on the bootstrap process for QEMU Aarch64 and QEMU x86_64 UEFI.

## Build

For now, this section is very brief, but I plan to expand it so that it can be really easy to build the bootloader and test it out.

Note: Don't use the `-j` argument when running make. As cargo handles most of the build process, allowing Make to use multiple threads does not improve the build speed.

### Build and Run With Qemu

To quickly test out the boot loader's Aarch64 implementation, run the following:

``` shell
make qemu
```

### Only Build

If you just want to build and not run in qemu, remove 'qemu' from the arguments:

``` shell
make
```

### Switch Target Architecture

Note: It may be difficult to run for `x86_64-unknown-uefi` because the build system currently assumes that OVMF will be used from the default Arch Linux path (`/usr/share/ovmf/x64/OVMF.fd`). I will soon change this so that it will instead be automatically downloaded.

If you want to run a different architecture, export it's target triple under the variable `RUST_TARGET`:

``` shell
export RUST_TARGET=x86_64-unknown-uefi
make qemu
```

If you only want to change the architecture for a single run, use the following command:

``` shell
RUST_TARGET=x86_64-unknown-uefi make qemu
```

## Supported Targets

The only target triples currently supported are:

* `aarch64-unknown-none`
* `x86_64-unknown-uefi`

## Goals

Here's a checklist of some goals for this bootloader. Each goal is for both x86_64 and Aarch64:

- [x] Entry point
- [x] Panic handler
- [x] Default text output device (uart for aarch64, efi_text_output for x86_64)
- [x] Default logger
- [x] Global allocator

## x86_64 Status

Some current parts of the x86_64 bootloader utilize the [uefi_services crate](https://docs.rs/uefi-services/latest/uefi_services/).

I plan to wean off the features of this crate by slowly re-implementing them myself. This includes the following:

- [ ] Entry point
- [x] Panic handler
- [ ] Default text output device
- [ ] Default logger
- [ ] Global allocator

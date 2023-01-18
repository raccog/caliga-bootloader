RUST_TARGET ?= aarch64-unknown-none

CARGO_BUILD_ARGS := --target=meta/target-specs/$(RUST_TARGET).json -Zbuild-std=core,alloc,compiler_builtins

# 1 for debug build, 0 for release build
DEBUG ?= 1
ifeq ($(DEBUG), 1)
	BUILD_TYPE = debug
else
	BUILD_TYPE = release
	CARGO_BUILD_ARGS += --release
endif

export TOOLCHAIN_BUILD_DIR := build-toolchain
export TARGET_BUILD_DIR := target/$(RUST_TARGET)/$(BUILD_TYPE)

export QEMU_EXTRA_ARGS ?=

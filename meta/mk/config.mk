TARGET_TRIPLE ?= x86_64-unknown-uefi

CARGO_BUILD_ARGS :=

# 1 for debug build, 0 for release build
DEBUG ?= 1
ifeq ($(DEBUG), 1)
	BUILD_TYPE = debug
else
	BUILD_TYPE = release
	CARGO_BUILD_ARGS += --release
endif


TARGET_BUILD_DIR := target/$(TARGET_TRIPLE)/$(BUILD_TYPE)

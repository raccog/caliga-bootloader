include meta/mk/config.mk
include meta/mk/$(TARGET_TRIPLE).mk

all: $(BOOTLOADER)

$(BOOTLOADER): bootloader

.PHONY: bootloader
bootloader:
	cargo build $(CARGO_BUILD_ARGS)

.PHONY: clean
clean:
	cargo clean

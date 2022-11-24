include meta/mk/config.mk
include meta/mk/$(TARGET_TRIPLE).mk

all: $(BOOTLOADER)

$(BOOTLOADER):
	cargo build $(CARGO_BUILD_ARGS)

.PHONY: clean
clean:
	cargo clean

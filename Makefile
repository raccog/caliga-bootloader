include meta/mk/config.mk
include meta/mk/$(TARGET_TRIPLE).mk

all: $(BOOTLOADER)

$(BOOTLOADER): FORCE
	cargo build $(CARGO_BUILD_ARGS)
FORCE:

.PHONY: clean
clean:
	cargo clean

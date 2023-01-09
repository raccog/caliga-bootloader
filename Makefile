include meta/mk/pre-config.mk
include meta/mk/$(RUST_TARGET).mk
include meta/mk/post-config.mk

all: $(BOOTLOADER)
.DEFAULT_GOAL := all

$(BOOTLOADER): FORCE
	cargo build $(CARGO_BUILD_ARGS)
FORCE:

.PHONY: clean
clean:
	cargo clean
	rm -rf build-external

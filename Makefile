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

# TODO: Add more checks to ensure that this doesn't remove any important files
.PHONY: distclean
distclean: clean
	rm -rf --preserve-root $(TOOLCHAIN_BUILD_DIR)

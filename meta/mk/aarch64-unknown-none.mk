BINARY = caliga-aarch64-qemu

BOOTLOADER := $(TARGET_BUILD_DIR)/$(BINARY)

.PHONY: qemu
qemu: $(BOOTLOADER)
	./meta/qemu-aarch64-virt.sh

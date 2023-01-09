BINARY = caliga-aarch64-qemu

BOOTLOADER := $(TARGET_BUILD_DIR)/$(BINARY)

.PHONY: qemu
qemu: $(BOOTLOADER)
	QEMU_EXTRA_ARGS="$(QEMU_EXTRA_ARGS)" ./meta/qemu-aarch64-virt.sh

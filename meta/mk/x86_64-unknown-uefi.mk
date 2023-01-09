BINARY = caliga-x86_64-uefi

BOOTLOADER := $(TARGET_BUILD_DIR)/$(BINARY).efi
ESP_IMG := $(TARGET_BUILD_DIR)/esp.img
export DISK_IMG := $(TARGET_BUILD_DIR)/disk.img

export OVMF_DST := $(TOOLCHAIN_BUILD_DIR)/OVMF.fd

CARGO_BUILD_ARGS += --features="uefi"

$(ESP_IMG): $(BOOTLOADER)
	dd if=/dev/zero of=$@ bs=1M count=64
	mkfs.vfat -F 32 $@
	mmd -D s -i $@ '::/EFI'
	mmd -D s -i $@ '::/EFI/BOOT'
	mcopy -D o -i $@ $< '::/EFI/BOOT/BOOTX64.EFI'
	mcopy -D o -i $@ tmp_config.txt '::/config.txt'

$(DISK_IMG): $(ESP_IMG)
	./meta/create-gpt.sh
	dd if=$< of=$@ bs=1M seek=1 count=64 conv=notrunc

# TODO: Move OVMF installation into a separate toolchain build script
$(OVMF_DST):
	mkdir -p $(TOOLCHAIN_BUILD_DIR)
	./meta/ovmf-cache.sh || ./meta/ovmf-compile.sh

.PHONY: qemu
qemu: $(DISK_IMG) $(OVMF_DST)
	./meta/qemu-x86_64-uefi.sh

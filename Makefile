ESP_IMG := target/ESP.img
DISK_IMG := target/DISK.img

BUILD_DIR := target/x86_64-unknown-uefi/debug
BOOTLOADER := $(BUILD_DIR)/bootloader.efi

# TODO: Let user choose OVMF dir (download if it doesn't exist?)
OVMF_DIR := /usr/share/edk2-ovmf
OVMF_TARGETS := target/OVMF_CODE.fd target/OVMF_VARS.fd

.PHONY: run-efi
run-efi: build-efi $(OVMF_TARGETS)
	qemu-system-x86_64 \
	    -drive file=target/OVMF_CODE.fd,if=pflash,format=raw,unit=0,readonly=on \
	    -drive file=target/OVMF_VARS.fd,if=pflash,format=raw,unit=1 \
	    -drive file=$(DISK_IMG),format=raw \
	    -cpu qemu64 \
	    -net none \
	    -serial stdio

.PHONY: build-efi
build-efi: build-bootloader $(DISK_IMG)

.PHONY: build-bootloader
build-bootloader:
	cargo build

.PHONY: clean
clean:
	cargo clean

.PHONY: help
help:
	@echo 'Commands:'
	@echo '  build-efi - Generate a GPT image that includes an ESP with'
	@echo '              the UEFI bootloader. (runs `cargo build` and other'
	@echo '		     commands to generate the disk image)'
	@echo '  run-efi   - Build and run the UEFI bootloader in QEMU. (runs'
	@echo '              the `build-efi` makefile target)'
	@echo '  clean     - Cleans the `target` directory. (runs `cargo clean`)'

$(ESP_IMG): $(BOOTLOADER)
	dd if=/dev/zero of=$(ESP_IMG) bs=1M count=64
	mkfs.vfat -F 32 $(ESP_IMG)
	mmd -D s -i $(ESP_IMG) '::/EFI'
	mmd -D s -i $(ESP_IMG) '::/EFI/BOOT'
	mcopy -D o -i $(ESP_IMG) $(BOOTLOADER) '::/EFI/BOOT/BOOTX64.EFI'

$(DISK_IMG): $(ESP_IMG)
	dd if=/dev/zero of=$(DISK_IMG) bs=1M count=66
	parted -s $(DISK_IMG) mklabel gpt
	parted -s $(DISK_IMG) mkpart ESP fat32 2048s 100%
	parted -s $(DISK_IMG) set 1 esp on
	dd if=$(ESP_IMG) of=$(DISK_IMG) bs=1M seek=1 count=64 conv=notrunc

target/OVMF_%.fd: $(OVMF_DIR)/OVMF_%.fd
	cp $< $@

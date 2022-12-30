BOOTLOADER := $(TARGET_BUILD_DIR)/caliga.efi
ESP_IMG := $(TARGET_BUILD_DIR)/esp.img
DISK_IMG := $(TARGET_BUILD_DIR)/disk.img

OVMF_DST := $(TARGET_BUILD_DIR)/ovmf.fd

$(ESP_IMG): $(BOOTLOADER)
	dd if=/dev/zero of=$@ bs=1M count=64
	mkfs.vfat -F 32 $@
	mmd -D s -i $@ '::/EFI'
	mmd -D s -i $@ '::/EFI/BOOT'
	mcopy -D o -i $@ $< '::/EFI/BOOT/BOOTX64.EFI'
	mcopy -D o -i $@ tmp_config.txt '::/config.txt'

$(DISK_IMG): $(ESP_IMG)
	./meta/create-gpt.sh $@
	dd if=$< of=$@ bs=1M seek=1 count=64 conv=notrunc

$(OVMF_DST):
	export OVMF
	cp $(shell ./meta/find-ovmf.sh) $@

qemu: $(DISK_IMG) $(OVMF_DST)
	qemu-system-x86_64 \
	    -drive file=$(OVMF_DST),if=pflash,format=raw,readonly=on \
	    -drive file=$(DISK_IMG),format=raw \
	    -cpu qemu64 \
	    -net none \
	    -serial stdio

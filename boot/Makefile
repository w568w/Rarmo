# Support arithmetics in makefiles
include gmtt.mk

# Calculate some values for building the images
sector_size := 512
boot_offset := 2048
n_sectors := $(call mul,256,1024)
n_boot_sectors := $(call mul,128,1024)
filesystem_offset = $(call add,$(boot_offset),$(n_boot_sectors))
n_filesystem_blocks = $(call sub,$(n_sectors),$(filesystem_offset))
boot_line := $(boot_offset), $(call div,$(call mul,$(n_boot_sectors),$(sector_size)),1024)K, c,
filesystem_line := $(filesystem_offset), $(call div,$(call mul,$(n_filesystem_blocks),$(sector_size)),1024)K, L,

BOOT_FILES := $(wildcard *.bin) config.txt $(wildcard *.linux) $(wildcard *.dat) $(wildcard *.broadcom) $(wildcard *.elf)

USER_FILES := $(wildcard ../user/*)

boot.img : $(kernel_bin) $(BOOT_FILES)
	$(DD) if=/dev/zero of=$@ seek=$(call sub,$(n_boot_sectors),1) bs=$(sector_size) count=1
	$(MKFS_VFAT) -F 32 -s 1 $@
	$(foreach file,$^,$(MCOPY) -i $@ $(file) ::$(notdir $(file)) $(DELIMITER_CHAR))

mkfs: fs/main.c
	$(SYSTEM_CC) -o $@ -static $^

fs.img: mkfs
	./mkfs $@ $(USER_FILES)

# Pipeline is not supported on Windows Powershell, so we have to use a temporary file.
sd.img: boot.img fs.img $(BOOT_FILES)
	$(DD) if=/dev/zero of=$@ seek=$(call sub,$(n_sectors),1) bs=$(sector_size) count=1
	$(PRINTF) "$(boot_line)\n$(filesystem_line)\n" > op.txt
	$(SFDISK) $@ < op.txt
	$(DD) if=boot.img of=$@ seek=$(boot_offset) conv=notrunc
	$(DD) if=fs.img of=$@ seek=$(filesystem_offset) conv=notrunc

.PHONY:clean
clean:
	-$(RM) sd.img
	-$(RM) boot.img
	-$(RM) fs.img
	-$(RM) op.txt
	-$(RM) mkfs
	-$(RM) mkfs.exe

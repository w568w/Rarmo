# -----------------
# START OF MAKEFILE
# -----------------
# This is the Makefile for the project.
# This Makefile is host-os-friendly! It should work on Windows, Linux and Mac OS with correct configurations.
#
# Read README.md first before you start to explore.
# -----------------
# You need to install the following packages first:
#
# Rust (You should install the nightly version, not stable)
# - https://rustup.rs/
# MSYS2 (If you are on Windows)
# - https://www.msys2.org/
# mtools (If you are on Windows, the executable has been included in the repo.)
# - apt install mtools (If you are on Debian)
# - homebrew install mtools (If you are on Mac OS)
# ARM cross compiler for `aarch64-none-elf` (You can grab one provided by ARM or Linaro. See below:)
# - https://developer.arm.com/Tools%20and%20Software/GNU%20Toolchain (Choose `AArch64 bare-metal target`)
# - https://releases.linaro.org/components/toolchain/binaries/ (Choose the latest gcc version and pick the toolchain under `aarch64-elf`)
# make
# - pacman -S mingw-w64-x86_64-make (In MSYS2, if you are on Windows)
# - apt install make (If you are on Debian)
# - brew install make (If you are on Mac OS)
# dosfstools
# - pacman -S dosfstools (In MSYS2, if you are on Windows)
# - apt install dosfstools (If you are on Debian)
# - brew install dosfstools (If you are on Mac OS)
# gdb-multiarch
# - pacman -S mingw-w64-x86_64-gdb-multiarch (In MSYS2, if you are on Windows)
# - apt install gdb-multiarch (If you are on Debian)
# - brew tap eblot/armeabi; brew install arm-none-eabi-gdb (If you are on Mac OS)
# jq (If you are on Windows, the executable has been included in the repo. You can also grab one from https://stedolan.github.io/jq/download/)
# - apt install jq (If you are on Debian)
# - brew install jq (If you are on Mac OS)
# Add the path to mingw32-make.exe to PATH environment variable (If you are on Windows)
# rustup target add aarch64-unknown-none
# rustup component add llvm-tools-preview
# cargo install cargo-binutils
# -----------------

mkfile_path := $(abspath $(lastword $(MAKEFILE_LIST)))
mkfile_dir := $(dir $(mkfile_path))


# -----------------
# User defined variables
# In most cases, you need to check and modify these variables one by one.
# -----------------
PROJECT_NAME := rarmo
TARGET := aarch64-unknown-none
DEFAULT_MODE := debug
QEMU_EXECUTABLE := qemu-system-aarch64
QEMU_DEBUGGING_PORT := 1234
QEMU_DEVICE := raspi3b
GCC_ROOT := D:/Flutter/gcc-linaro-7.5.0-2019.12-i686-mingw32_aarch64-elf/gcc-linaro-7.5.0-2019.12-i686-mingw32_aarch64-elf/bin/
GCC_PREFIX := aarch64-elf-

# You only need to modify the paths below if you are using Windows.
# In Linux, you can just leave them as they are or delete these lines safely.
MSYS2_ROOT := D:/Flutter/msys64/
QEMU_ROOT := D:/Program Files/qemu/

# The variables below are automatically determined, and in most cases you do not need to edit them (if you are on Windows).
#
# If some tools cannot be found, you can modify them manually.
ifeq ($(OS), Windows_NT)
	SYSBIN := $(MSYS2_ROOT)usr/bin/
	export RM := del
	export MCOPY := $(mkfile_dir)boot/mtools/mcopy
	export COPY := copy
	export JQ := $(mkfile_dir)misc/jq
	export DELIMITER_CHAR := &
	export FixPath = $(subst /,\,$1)
	GDB := $(MSYS2_ROOT)mingw64/bin/gdb-multiarch
else
	QEMU_ROOT :=
	SYSBIN :=
	export RM := rm
	export MCOPY := mcopy
	export COPY := cp
	export JQ := jq
	export DELIMITER_CHAR := ;
	export FixPath = $(subst \,/,$1)
	GDB := gdb-multiarch
endif
# Configure the path of other executables. You can modify them manually if you want.
export DD := $(SYSBIN)dd
export SFDISK := $(SYSBIN)sfdisk
export PRINTF := $(SYSBIN)printf
export MKFS_VFAT := $(SYSBIN)mkfs.vfat
export QEMU := $(QEMU_ROOT)$(QEMU_EXECUTABLE)
export CC := $(GCC_ROOT)$(GCC_PREFIX)gcc
export SYSTEM_CC := gcc
ARCH_S_FILES := $(wildcard src/aarch64/*.S) $(wildcard src/*.S)
ARCH_ASM_FILES := $(patsubst %.S,%.asm,$(ARCH_S_FILES))
# -----------------

# -----------------
# Variables with automatic values
# -----------------
rust_build_path := $(mkfile_dir)target/$(TARGET)/$(DEFAULT_MODE)
artifact_prefix := $(rust_build_path)/$(PROJECT_NAME)
export kernel_bin := $(artifact_prefix).bin
qemu_flags := -machine $(QEMU_DEVICE) \
					  -nographic \
                      -drive "file=boot/sd.img,if=sd,format=raw" \
                      -kernel "$(kernel_bin)" \
                      -serial "null" \
                      -serial "mon:stdio"
rust_build_mode_arg := --$(DEFAULT_MODE)
ifeq ($(DEFAULT_MODE),debug)
	# We don't need to pass this flag for debug builds.
    rust_build_mode_arg =
endif
# -----------------

.PHONY:all
all: $(kernel_bin)

src/%.asm: src/%.S
	$(CC) -S $< > $@

src/aarch64/%.asm: src/aarch64/%.S
	$(CC) -S $< > $@

$(artifact_prefix): $(ARCH_ASM_FILES)
	cargo build --target $(TARGET) $(rust_build_mode_arg)

$(kernel_bin): $(artifact_prefix)
	rust-objcopy --strip-all $< -O binary $@

boot/sd.img: $(kernel_bin)
	$(MAKE) -C boot $(notdir $@)

.PHONY:run
run: boot/sd.img
	$(QEMU) $(qemu_flags)

TMP_FILE := $(file < test_files_filtered.txt)
.PHONY:inner_test
inner_test: $(ARCH_ASM_FILES) test_files_filtered.txt $(TMP_FILE)
	$(COPY) $(TMP_FILE) $(call FixPath,$(artifact_prefix))
	$(MAKE) run

.PHONY:test
test: $(ARCH_ASM_FILES)
	cargo test --target $(TARGET) --no-run --message-format json > test_files.txt
	$(JQ) -r "select(.profile.test == true) | .filenames[]" < test_files.txt > test_files_filtered.txt
	$(MAKE) inner_test

.PHONY:qemu-debug
qemu-debug: boot/sd.img
	$(QEMU) $(qemu_flags) -nographic \
		-S -gdb tcp::$(QEMU_DEBUGGING_PORT)

.PHONY:debug
debug: $(artifact_prefix)
	$(GDB) --nx --quiet \
	   -ex "set architecture aarch64" \
	   -ex "file $(artifact_prefix)" \
	   -ex "target remote :$(QEMU_DEBUGGING_PORT)"

.PHONY:clean
clean:
	-$(RM) test_files.txt
	-$(RM) test_files_filtered.txt
	-$(RM) $(call FixPath,src/entry.asm)
	-$(foreach file,$(ARCH_ASM_FILES),$(RM) $(call FixPath, $(file)) $(DELIMITER_CHAR))
	-$(MAKE) -C boot clean
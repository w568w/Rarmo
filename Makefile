# -----------------
# START OF MAKEFILE
# -----------------
# This is the Makefile for the project.
# This Makefile is host-os-friendly! It should work on Windows, Linux and Mac OS X with correct configurations.
#
# Read README.md first before you start to explore.
# -----------------
# You need to install the following packages first:
#
# Rust (You should install the nightly version, not stable)
# MSYS2 (If you are on Windows)
# mtools (If you are on Linux)
# ARM cross compiler for `aarch64-none-elf` (You can grab one provided by ARM or Linaro. See below:)
# - https://developer.arm.com/Tools%20and%20Software/GNU%20Toolchain (Choose `AArch64 bare-metal target`)
# - https://releases.linaro.org/components/toolchain/binaries/ (Choose the latest gcc version and pick the toolchain under `aarch64-elf`)
# make
# - pacman -S mingw-w64-x86_64-make (In MSYS2, if you are on Windows)
# - apt install make (If you are on Debian)
# gdb-multiarch
# - pacman -S mingw-w64-x86_64-gdb-multiarch (In MSYS2, if you are on Windows)
# - apt install gdb-multiarch (If you are on Debian)
# Add the path to mingw32-make.exe to PATH environment variable (if you are on Windows)
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
DEFAULT_MODE := release
QEMU_EXECUTABLE := qemu-system-aarch64
QEMU_DEBUGGING_PORT := 1234
# You only need to modify the paths below if you are using Windows.
# In Linux, you can just leave them as they are or delete these lines safely.
MSYS2_ROOT := D:/Flutter/msys64/
QEMU_ROOT := D:/Program Files/qemu/
GCC_ROOT := D:/Flutter/gcc-linaro-7.5.0-2019.12-i686-mingw32_aarch64-elf/gcc-linaro-7.5.0-2019.12-i686-mingw32_aarch64-elf/bin/

# The variables below are automatically determined, and in most cases you do not need to edit them (if you are on Windows).
#
# If some tools cannot be found, you can modify them manually.
ifeq ($(OS), Windows_NT)
	SYSBIN := $(MSYS2_ROOT)usr/bin/
	export RM := del
	export MCOPY := $(mkfile_dir)boot/mtools/mcopy
	export DELIMITER_CHAR := &
	GDB := $(MSYS2_ROOT)mingw64/bin/gdb-multiarch
else
	QEMU_ROOT :=
	GCC_ROOT :=
	SYSBIN :=
	export RM := rm
	export MCOPY := mcopy
	export DELIMITER_CHAR := ;
	GDB := gdb-multiarch
endif
# Configure the path of other executables. You can modify them manually if you want.
export DD := $(SYSBIN)dd
export SFDISK := $(SYSBIN)sfdisk
export PRINTF := $(SYSBIN)printf
export MKFS_VFAT := $(SYSBIN)mkfs.vfat
export QEMU := $(QEMU_ROOT)$(QEMU_EXECUTABLE)
export CC := $(GCC_ROOT)aarch64-elf-gcc
# -----------------

# -----------------
# Variables with automatic values
# -----------------
rust_build_path := $(mkfile_dir)target/$(TARGET)/$(DEFAULT_MODE)
artifact_prefix := $(rust_build_path)/$(PROJECT_NAME)
export kernel_bin := $(artifact_prefix).bin
qemu_flags := -machine raspi3b \
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

src/entry.asm: src/entry.S
	$(CC) -S $< > $@

$(artifact_prefix): src/entry.asm
	cargo build --target $(TARGET) $(rust_build_mode_arg)

$(kernel_bin): $(artifact_prefix)
	rust-objcopy --strip-all $< -O binary $@

boot/sd.img: $(kernel_bin)
	$(MAKE) -C boot $(notdir $@)

.PHONY:run
run: boot/sd.img
	$(QEMU) $(qemu_flags)

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
	-cargo clean
	-cd src && $(RM) entry.asm && cd ..
	-$(MAKE) -C boot clean
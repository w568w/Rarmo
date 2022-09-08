# -----------------
# START OF MAKEFILE
# -----------------
# This is the makefile for the project.
# It is used to compile the project.
# -----------------
# You need to install the following packages first:
# Rust
# MSYS2 (If you are on Windows)
# mtools (If you are on Linux)
# pacman -S mingw-w64-x86_64-gdb-multiarch (In MSYS2, if you are on Windows)
# rustup add component rust-src
# rustup target add aarch64-unknown-none --toolchain nightly
# rustup component add llvm-tools-preview
# cargo install cargo-binutils
# -----------------

mkfile_path := $(abspath $(lastword $(MAKEFILE_LIST)))
mkfile_dir := $(dir $(mkfile_path))

# -----------------
# User defined variables
# In most cases, you need to check and modify these variables one by one.
# -----------------
PROJECT_NAME := Rarmo
TARGET := aarch64-unknown-none
DEFAULT_MODE := debug
QEMU_EXECUTABLE := qemu-system-aarch64

# You only need to modify the paths below if you are using Windows.
MSYS2_ROOT := D:/Flutter/msys64
QEMU_ROOT := D:/Program Files/qemu

ifeq ($(OS), Windows_NT)
	SYSBIN := $(MSYS2_ROOT)/usr/bin/
	export RM := del
	export MCOPY := $(mkfile_dir)boot/mtools/mcopy
	GDB := $(MSYS2_ROOT)/mingw64/bin/gdb-multiarch
else
	QEMU_ROOT :=
	SYSBIN :=
	export RM := rm
	export MCOPY := mcopy
	GDB := gdb-multiarch
endif
# Configure the path of other executables.
export DD := $(SYSBIN)dd
export SFDISK := $(SYSBIN)sfdisk
export PRINTF := $(SYSBIN)printf
export MKFS_VFAT := $(SYSBIN)mkfs.vfat
export QEMU := $(QEMU_ROOT)/$(QEMU_EXECUTABLE)
# -----------------

# -----------------
# Auto generated variables
# -----------------
rust_build_path := $(mkfile_dir)target/$(TARGET)/$(DEFAULT_MODE)
artifact_prefix := $(rust_build_path)/$(PROJECT_NAME)
export kernel_bin := $(artifact_prefix).bin
qemu_flags := -machine raspi3b \
                      -drive "file=boot/sd.img,if=sd,format=raw" \
                      -kernel "$(kernel_bin)"
rust_build_mode_arg := -$(DEFAULT_MODE)
ifeq ($(DEFAULT_MODE),debug)
	# We don't need to pass this flag for debug builds.
    rust_build_mode_arg =
endif
# -----------------

.PHONY:all
all: $(kernel_bin)

$(artifact_prefix):
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
		-S -gdb tcp::1234

.PHONY:debug
debug: boot/sd.img
	$(GDB) --nx --quiet \
	   -ex "set architecture aarch64" \
	   -ex "file ${artifact_prefix}" \
	   -ex "target remote :1234"

.PHONY:clean
clean:
	-cargo clean
	-$(MAKE) -C boot clean
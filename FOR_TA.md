# A quick guide for teach assistants
Rarmo is able to run on the school-provided server with some simple settings. 

This guide is a "TL; DR" version of README written for TAs to help them set up the environment on a Kunpeng server quickly.

1. Clone the repository: `git clone https://github.com/w568w/Rarmo.git`.
2. Download the latest version of ARM GCC toolchain from [here](https://developer.arm.com/Tools%20and%20Software/GNU%20Toolchain) and extract it. **Note: it is a MUST to choose `AArch64 ELF bare-metal target (aarch64-none-elf)` under `AArch64 Linux hosted cross toolchains`. If it cannot be found, look for an older version.**
3. Install `gdb-multiarch` and `mtools`: `apt install gdb-multiarch mtools`.
4. Install Rust: run the installer from [rustup](https://rustup.rs/). **Note: it is also a MUST to choose the `nightly` version manually at the first step.**
5. Install jq: `apt install jq`.
6. Modify the `Makefile` to fit your environment. In most cases, you need to change `GCC_ROOT :=` **in the `else` .. `endif` block** to your path of the extracted toolchain, and `export CC := ...` to `export CC := $(GCC_ROOT)aarch64-none-elf-gcc`.
6. (continue. ) also, change `-machine` in `qemu_flags := ...` to `raspi3`, if QEMU reports something like `'raspi3b' is not a valid machine type`.
7. A simple `make test` should be able to build and run Rarmo now. If it fails or gets stuck at compilation, try running `make clean` and `make test` again.
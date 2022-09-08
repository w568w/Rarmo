# Rarmo

A simple operating system in Rust.

Rarmo stands for `Rusty ARM Operating system` or `Rarmo: A Rust-Made Operating system`.

It is a simple operating system written in Rust for `aarch64`, and a hobby project to learn more about operating systems
and Rust.

At this moment, it does not have a GUI or even a simple shell, but it can boot on Raspberry PI 3 / 3B and run some simple codes in Rust.

## How to build

### Prerequisites

First of all, you need to install some tools to build Rarmo.

You can find the list of tools in [Makefile](Makefile).

Rarmo should be able to be built on Linux, macOS and Windows with [MSYS2](https://www.msys2.org/).

### Build

1. Clone this repository with `git clone https://github.com/w568w/Rarmo.git`;
2. Modify the `Makefile` to fit your environment;
3. Run `make all` to build Rarmo; here are some useful targets:
    - `make all`: build Rarmo;
    - `make run`: build Rarmo and run it in QEMU;
    - `make clean`: clean the build;
    - `make qemu-debug`: build Rarmo and run QEMU in debug mode (no display, but you can use GDB to debug);
    - `make debug`: build Rarmo and start a connected GDB for debugging;

## References
Some codes come from the following projects / articles / labs:

- [Writing an OS in Rust](https://os.phil-opp.com/)
- [rCore-Tutorial-Book-v3](https://rcore-os.github.io/rCore-Tutorial-Book-v3/)
- [Fudan OS2021 Fall, lab1](https://github.com/FDUCSLG/OS-2021Fall-Fudan/tree/lab1)

## Acknowledgements
I would like to thank the authors of the above ones for their great work. 

Also, [Huawei](https://www.huawei.com/)'s [Intelligent Center Plan](https://www.huawei.com/us/corporate-information/openness-collaboration-and-shared-success) with [Fudan University](https://news.fudan.edu.cn/2020/0929/c5a106516/page.htm) provided a great opportunity to learn more about Kunpeng processors and operating systems.

## License
Rarmo is licensed under the [MIT License](https://opensource.org/licenses/MIT).
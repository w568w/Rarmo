/* Check a useful tutorial at https://developer.arm.com/documentation/dui0473/m/writing-arm-assembly-language. */

.section .text.entry
.globl _start
_start:
ldr x9, =boot_stack_top
mov sp, x9
ldr x9, =main
br x9

.section .bss.stack
.globl boot_stack
boot_stack:
.space 4096 * 16
.globl boot_stack_top
boot_stack_top:
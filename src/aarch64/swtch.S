// Do kernel-mode context switch
// x0 (first parameter): new context ptr
// x1 (second parameter): addr to save old context ptr
//
// This function will block until switching back to this process.
#define pushp(a, b) stp a, b, [sp, #-0x10]!
#define popp(a, b) ldp a, b, [sp], #0x10

.globl swtch
swtch:
// Save callee-saved registers (x19-x28)
pushp(x19, x20)
pushp(x21, x22)
pushp(x23, x24)
pushp(x25, x26)
pushp(x27, x28)

// Save frame register (fp, or x29) and link register (lr, or x30)
pushp(x29, x30)

// Let [x1] be stack pointer (sp)
mov x19, sp
str x19, [x1]

// Switch to the new stack
mov sp, x0

// Restore stack pointer and link register (lr, or x30)
popp(x29, x30)
popp(x27, x28)
popp(x25, x26)
popp(x23, x24)
popp(x21, x22)
popp(x19, x20)

// Return to the new caller
ret
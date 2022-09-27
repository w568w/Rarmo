// Do kernel-mode context switch
// x0 (first parameter): new context ptr
// x1 (second parameter): addr to save old context ptr
//
// This function will block until switching back to this process.
#define pushp(a, b) stp a, b, [sp, #-0x10]!
#define pushq(a, b) stp a, b, [sp, #-0x20]!
#define popp(a, b) ldp a, b, [sp], #0x10
#define popq(a, b) ldp a, b, [sp], #0x20

.globl swtch
swtch:
// Save frame register (fp, or x29) and link register (lr, or x30)
pushp(x29, x30)

// Save callee-saved registers (x19-x28)
pushp(x27, x28)
pushp(x25, x26)
pushp(x23, x24)
pushp(x21, x22)
pushp(x19, x20)

// Save argument registers (x0-x7)
pushp(x6, x7)
pushp(x4, x5)
pushp(x2, x3)
pushp(x0, x1)

// Save float register (q0-q31)
pushq(q30, q31)
pushq(q28, q29)
pushq(q26, q27)
pushq(q24, q25)
pushq(q22, q23)
pushq(q20, q21)
pushq(q18, q19)
pushq(q16, q17)
pushq(q14, q15)
pushq(q12, q13)
pushq(q10, q11)
pushq(q8, q9)
pushq(q6, q7)
pushq(q4, q5)
pushq(q2, q3)
pushq(q0, q1)


// Let [x1] be stack pointer (sp)
mov x19, sp
str x19, [x1]

// Switch to the new stack
mov sp, x0

// Restore everything
popq(q0, q1)
popq(q2, q3)
popq(q4, q5)
popq(q6, q7)
popq(q8, q9)
popq(q10, q11)
popq(q12, q13)
popq(q14, q15)
popq(q16, q17)
popq(q18, q19)
popq(q20, q21)
popq(q22, q23)
popq(q24, q25)
popq(q26, q27)
popq(q28, q29)
popq(q30, q31)

popp(x0, x1)
popp(x2, x3)
popp(x4, x5)
popp(x6, x7)

popp(x19, x20)
popp(x21, x22)
popp(x23, x24)
popp(x25, x26)
popp(x27, x28)
popp(x29, x30)

// Return to the new caller
ret
#define pushp(a, b) stp a, b, [sp, #-0x10]!
#define pushq(a, b) stp a, b, [sp, #-0x20]!
#define popp(a, b) ldp a, b, [sp], #0x10
#define popq(a, b) ldp a, b, [sp], #0x20

/* `exception_vector.S` send all traps here. */
.global trap_entry
trap_entry:
// Save the context to stack.

// Save all general purpose registers.
pushp(x30, x31)
pushp(x28, x29)
pushp(x26, x27)
pushp(x24, x25)
pushp(x22, x23)
pushp(x20, x21)
pushp(x18, x19)
pushp(x16, x17)
pushp(x14, x15)
pushp(x12, x13)
pushp(x10, x11)
pushp(x8, x9)
pushp(x6, x7)
pushp(x4, x5)
pushp(x2, x3)
pushp(x0, x1)
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

// Save SPSR (Saved Program Status Register) and ELR (Exception Link Register)
mrs x0, SPSR_EL1
mrs x1, ELR_EL1
pushp(x0, x1)

// Save the current stack pointer.
mov x0, sp

// Call the trap handler.
bl trap_global_handler


.global trap_return
trap_return:
// Restore the context from stack.
popp(x0, x1)
msr SPSR_EL1, x0
msr ELR_EL1, x1

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
popp(x8, x9)
popp(x10, x11)
popp(x12, x13)
popp(x14, x15)
popp(x16, x17)
popp(x18, x19)
popp(x20, x21)
popp(x22, x23)
popp(x24, x25)
popp(x26, x27)
popp(x28, x29)
popp(x30, x31)

// Return to the next instruction.
eret


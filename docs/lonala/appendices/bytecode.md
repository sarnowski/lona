# Appendix B: Bytecode Reference
This appendix documents the virtual machine instruction set. This is an implementation detail for contributors.

## Instruction Encoding

Instructions are 32-bit values with the following formats:

| Format | Layout | Description |
|--------|--------|-------------|
| iABC | `[op:8][A:8][B:8][C:8]` | Three register operands |
| iABx | `[op:8][A:8][Bx:16]` | Register + 16-bit index |
| iAsBx | `[op:8][A:8][sBx:16]` | Register + signed offset |

## Instruction Set

| Opcode | Name | Format | Description |
|--------|------|--------|-------------|
| 0 | Move | iABC | `R[A] = R[B]` |
| 1 | LoadK | iABx | `R[A] = K[Bx]` |
| 2 | LoadNil | iABC | `R[A]..R[A+B] = nil` |
| 3 | LoadTrue | iABC | `R[A] = true` |
| 4 | LoadFalse | iABC | `R[A] = false` |
| 5 | GetGlobal | iABx | `R[A] = globals[K[Bx]]` |
| 6 | SetGlobal | iABx | `globals[K[Bx]] = R[A]` |
| 7 | Add | iABC | `R[A] = RK[B] + RK[C]` |
| 8 | Sub | iABC | `R[A] = RK[B] - RK[C]` |
| 9 | Mul | iABC | `R[A] = RK[B] * RK[C]` |
| 10 | Div | iABC | `R[A] = RK[B] / RK[C]` |
| 11 | Mod | iABC | `R[A] = RK[B] % RK[C]` |
| 12 | Neg | iABC | `R[A] = -R[B]` |
| 13 | Eq | iABC | `R[A] = RK[B] == RK[C]` |
| 14 | Lt | iABC | `R[A] = RK[B] < RK[C]` |
| 15 | Le | iABC | `R[A] = RK[B] <= RK[C]` |
| 16 | Gt | iABC | `R[A] = RK[B] > RK[C]` |
| 17 | Ge | iABC | `R[A] = RK[B] >= RK[C]` |
| 18 | Not | iABC | `R[A] = not R[B]` |
| 19 | Jump | iAsBx | `PC += sBx` |
| 20 | JumpIf | iAsBx | `if R[A] then PC += sBx` |
| 21 | JumpIfNot | iAsBx | `if not R[A] then PC += sBx` |
| 22 | Call | iABC | `R[A..A+C-1] = R[A](R[A+1..A+B])` |
| 23 | TailCall | iABC | `return R[A](R[A+1..A+B])` |
| 24 | Return | iABC | `return R[A..A+B-1]` |

## RK Encoding

Operands B and C in arithmetic/comparison instructions use RK encoding:
- If bit 7 is clear (0-127): Refers to register R[n]
- If bit 7 is set (128-255): Refers to constant K[n-128]

---


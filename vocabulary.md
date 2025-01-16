dc token vocabulary
===================

# Implementations
1. `cherry`: the original version by Lorinda Cherry (as described by 7th Edition Unix man page, 1985)
2. `gnu`
3. `bsd`: by Otto Moerbeek
5. `gavin`: Gavin Howard's version, present on macOS since Ventura

Currently, dc4 implements the GNU set.

# Tokens

| Token | dc4 Action Name   | Implementations           | Notes |
| ----- | ----------------- | ------------------------- | ----- |
|       |                   |
| `!`   | `ShellExec`       | cherry, gnu, bsd, dc4     | but not OpenBSD; dc4 parses it but only for purposes of raising an error
| `!<`  | `Ge`              | bsd, gnu, gavin
| `!<xey` | unimplemented   | bsd, gavin                | execute x if true, y if false
| `!=`  | `Ne`              | bsd, gnu, gavin
| `!=xey` | unimplemented   | bsd, gavin                | execute x if true, y if false
| `!>`  | `Le`              | bsd, gnu, gavin
| `!>xey` | unimplemented   | bsd, gavin                | execute x if true, y if false
| `"`   | unimplemented     | gavin                     | "Pops a value off of the stack, which is used as an exclusive upper bound on the integer that will be generated. [...] If the bound is larger than DC_RAND_MAX, the higher bound is honored by generating several pseudo-random integers, multiplying them by appropriate powers of DC_RAND_MAX+1, and adding them together."
| `#`   | comment           | bsd, gnu
| `$`   | unimplemented     | gavin                     | "The top value is popped off the stack and copied, and the copy is truncated and pushed onto the stack."
| `%`   | `Rem`             | cherry, bsd, gnu, gavin
| `&`   |
| `'`   | unimplemented     | gavin                     | "Generates an integer between 0 and DC_RAND_MAX, inclusive."
| `(`   | unimplemented     | bsd, gavin                | "The top two numbers are popped from the stack and compared. A one is pushed if the top of the stack is less than the second number on the stack. A zero is pushed otherwise. This is a non-portable extension."
| `)`   | unimplemented     | gavin                     | "The top two values are popped off of the stack, they are compared, and a 1 is pushed if the first is greater than the second, or 0 otherwise. This is a non-portable extension."
| `*`   | `Mul`             | cherry, bsd, gnu, gavin
| `+`   | `Add`             | cherry, bsd, gnu, gavin
| `,`   | unimplemented     | gavin                     | "Pushes the depth of the execution stack onto the stack. This is a non-portable extension."
| `-`   | `Sub`             | cherry, bsd, gnu, gavin
| `.`   | number            |
| `/`   | `Div`             | cherry, bsd, gnu, gavin
| `0`..`9` | number |
| `:`   | `StoreRegArray`   | cherry, bsd, gnu, gavin
| `;`   | `LoadRegArray`    | cherry, bsd, gnu, gavin
| `<`   | `Lt`              | cherry, bsd, gnu, gavin
| `<xey` | unimplemented    | bsd, gavin                | execute x if true, y if false
| `=`   | `Eq`              | cherry, bsd, gnu, gavin
| `=xey` | unimplemented    | bsd, gavin                | execute x if true, y if false
| `>`   | `Gt`              | cherry, bsd, gnu, gavin
| `>xey` | unimplemented    | bsd, gavin                | execute x if true, y if false
| `?`   | `Input`           | cherry, bsd, gnu, gavin
| `@`   | `Version`         | dc4                       | this is the only dc4-specific token
| `@`   | unimplemented     | gavin                     | "The top two values are popped off the stack, and the precision of the second is set to the value of the first, whether by truncation or extension. The first value popped off of the stack must be an integer and non-negative."
| `A`..`F` | number         |
| `G`   | unimplemented     | bsd, gavin                | "The top two numbers are popped from the stack and compared. A one is pushed if the top of the stack is equal to the second number on the stack. A zero is pushed otherwise. This is a non-portable extension."
| `H`   | unimplemented     | gavin                     | "The top two values are popped off the stack, and the second is shifted left (radix shifted right) to the value of the first. The first value popped off of the stack must be an integer and non-negative. This is a non-portable extension."
| `I`   | `LoadInputRadix`  | bsd, gnu, gavin
| `J`   | unimplemented     | bsd                       | "Pop the top value from the stack. The recursion level is popped by that value and, following that, the input is skipped until the first occurrence of the M operator. The J operator is a non-portable extension, used by the bc(1) command."
| `J`   | unimplemented     | gavin                     | "Pushes the current value of seed onto the main stack."
| `K`   | `LoadPrecision`   | bsd, gnu, gavin
| `LX`  | `PopRegStack`     | bsd, gnu, gavin
| `M`   | unimplemented     | bsd                       | "Mark used by the J operator. The M operator is a non-portable extension, used by the bc(1) command."
| `M`   | unimplemented     | gavin                     | "The top two values are popped off of the stack.  If they are both non-zero, a 1 is pushed onto the stack.  If either of them is zero, or both of them are, then a 0 is pushed onto the stack. This is like the && operator in bc(1), and it is not a short-circuit operator. This is a non-portable extension."
| `N`   | unimplemented     | bsd, gavin                | "The top of the stack is replaced by one if the top of the stack is equal to zero. If the top of the stack is unequal to zero, it is replaced by zero. This is a non-portable extension."
| `O`   | `LoadOutputRadix` | cherry, bsd, gnu, gavin
| `P`   | `PrintBytesPop`   | bsd, gnu, gavin
| `Q`   | `QuitLevels`      | bsd, gnu, gavin
| `R`   | unimplemented     | bsd, gavin                | "The top of the stack is removed (popped). This is a non-portable extension."
| `R`   | unimplemented     | gnu                       | incompatible with BSD. Pop an integer N, and rotate the top N stack items in place. Added to GNU dc in v1.4 (bc 1.07, released in 2017).
| `SX`  | `PushRegStack`    | bsd, gnu, gavin
| `T`   | unimplemented     | gavin                     | "Pushes the maximum allowable value of ibase onto the main stack. This is a non-portable extension."
| `U`   | unimplemented     | gavin                     | "Pushes the maximum allowable value of obase onto the main stack. This is a non-portable extension."
| `V`   | unimplemented     | gavin                     | "Pushes the maximum allowable value of scale onto the main stack. This is a non-portable extension."
| `W`   | unimplemented     | gavin                     | "Pushes the maximum (inclusive) integer that can be generated with the ’ pseudo-random number generator command. This is a non-portable extension."
| `X`   | `NumFrxDigits`    | cherry, bsd, gnu, gavin
| `Yr`  | unimplemented     | gavin                     | "Pushes the length of the array r onto the stack. This is a non-portable extension."
| `Z`   | `NumDigits`       | cherry, bsd, gnu, gavin
| `[`   | string            |
| `` \ `` | string            |
| `]`   | string            |
| `^`   | `Exp`             | cherry, bsd, gnu, gavin
| `_`   | number            |                           | gavin also uses it ouside a number context: "Otherwise, the top value on the stack is popped and copied, and the copy is negated and pushed onto the stack.  This behavior without a number is a non-portable extension."
| `` ` `` |
| `a`   | `Asciify`         | bsd, gnu, gavin
| `b`   | unimplemented     | gavin                     | "The top value is popped off the stack, and if it is zero, it is pushed back onto the stack.  Otherwise, its absolute value is pushed onto the stack. This is a non-portable extension."
| `c`   | `ClearStack`      | cherry, bsd, gnu, gavin
| `d`   | `Dup`             | cherry, bsd, gnu, gavin
| `e`   | unimplemented     | bsd                       | "Equivalent to p, except that the output is written to the standard error stream. This is a non-portable extension."
| `f`   | `PrintStack`      | cherry, bsd, gnu, gavin
| `gX`  | unimplemented     | gavin                     | gl, gx, gz: get some global settings values
| `h`   | unimplemented     | gavin                     | "The top two values are popped off the stack, and the second is shifted right (radix shifted left) to the value of the first. The first value popped off of the stack must be an integer and non-negative."
| `i`   | `SetInputRadix`   | cherry, bsd, gnu, gavin
| `j`   | unimplemented     | gavin                     | "Pops the value off of the top of the stack and uses it to set seed."
| `k`   | `SetPrecision`    | cherry, bsd, gnu, gavin
| `lX`  | `Load`            | cherry, bsd, gnu, gavin
| `m`   | unimplemented     | gavin                     | "The top two values are popped off of the stack.  If at least one of them is non-zero, a 1 is pushed onto the stack.  If both of them are zero, then a 0 is pushed onto the stack. This is like the || operator in bc(1), and it is not a short-circuit operator. This is a non-portable extension."
| `n`   | `PrintNoNewlinePop` | bsd, gnu, gavin         | "The top value on the stack is popped and printed without a newline. This is a non-portable extension."
| `o`   | `SetOutputRadix`  | cherry, bsd, gnu, gavin
| `p`   | `Print`           | cherry, bsd, gnu, gavin
| `q`   | `Quit`            | cherry, bsd, gnu, gavin
| `r`   | `Swap`            | bsd, gnu, gavin           | "The top two values on the stack are reversed (swapped). This is a non-portable extension."
| `sX`  | `Store`           | cherry, bsd, gnu, gavin
| `t`   | unimplemented     | gavin                     | "Pops one value off of the stack.  If the value is a string, this pushes 1 onto the stack.  Otherwise (if it is a number), it pushes 0. This is a non-portable extension."
| `u`   | unimplemented     | gavin                     | "Pops one value off of the stack.  If the value is a number, this pushes 1 onto the stack.  Otherwise (if it is a string), it pushes 0. This is a non-portable extension."
| `v`   | `Sqrt`            | cherry, bsd, gnu, gavin
| `w`   |
| `x`   | `ExecuteMacro`    | cherry, bsd, gnu, gavin
| `yr`  | unimplemented     | gavin                     | "Pushes the current stack depth of the register r onto the main stack. This is a non-portable extension."
| `z`   | `StackDepth`      | cherry, bsd, gnu, gavin
| `{`   | unimplemented     | bsd, gavin                | "The top two numbers are popped from the stack and compared. A one is pushed if the top of stack is less than or equal to the second number on the stack. A zero is pushed otherwise. This is a non-portable extension."
| `\|`  | `Modexp`          | gnu, gavin 
| `}`   | unimplemented     | gavin                     | "The top two values are popped off of the stack, they are compared, and a 1 is pushed if the first is greater than or equal to the second, or 0 otherwise. This is a non-portable extension."
| `~`   | `DivRem`          | bsd, gnu, gavin

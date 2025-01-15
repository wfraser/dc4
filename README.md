# dc4

A reimplementation of the Unix `dc` program in Rust.

# What is `dc`?

dc stands for "desk calculator". It is an arbitrary-precision
reverse-polish-notation calculator with a turing-complete set of very terse
commands. It was originally introduced in the earliest AT&T Unix versions, and
even predates the C programming language. dc was invented by Robert Morris and
Lorinda Cherry.

dc can even be thought of as a strange, very limited, programming language. Its
ability to define and conditionally execute "macros", or reusable bits of
input, allow it to perform loops and conditional branches. The syntax consists
of numbers and strings (surrounded by square brackets `[` and `]`), which are
pushed onto a stack when typed; and a collection of one- and two-character
commands which manipulate the stack or perform I/O.

The full syntax is well-described by the GNU dc man page. Run `man 1 dc` or
view [an online version](https://linux.die.net/man/1/dc).

`dc4` is a reimplementation of `dc` compatible with GNU dc, which is currently
the most widespread version, and which contains significant extensions over the
traditional Unix version. The source code of GNU dc was not used to develop
dc4, but its documentation and its observed behavior were used extensively.

Performance is on par with, or slightly better than, GNU dc; though the binary
size is quite a bit bigger, mostly due to the numeric library being statically
linked instead of dynamically linked.

# Differences from GNU dc

Some behaviors have been intentionally changed from GNU dc:

- running shell commands with the '!' command is not supported

Any other differences (other than cases where GNU dc crashes and dc4 does not)
should be considered a bug.

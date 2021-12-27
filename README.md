# Unix-v6-like shell written in Rust

This repository contains a Unix-v6-like shell implemented in Rust.

I wrote this code to exercise with Rust. I do not plan on maintaining
this code base or on adding additional features. It's likely the
case that this code behaves differently from the original v6 shell.

## Design

The design is quite traditional for a compiler-like tool:

1. there is a lexer (see [src/lexer.rs](src/lexer.rs));

2. the lexer feeds a parser (see [src/parser.rs](src/parser.rs)),
which produces a _parse tree_;

3. we transform and validate the parse tree for easier execution
(see [src/translator.rs](src/translator.rs));

4. we interpret the transformed output (see [src/interp.rs](src/interp.rs))
to execute shell commands.

When we encounter commands between `(` and `)` we execute them in
a subshell. We pass code to the subshell by serializing the specific
portion of the parse tree using [src/serializer.rs](src/serializer.rs).

The `-c COMMANDS` command allows a shell (or a sub-shell) to
execute a sequence of commands.

The `-x` command line argument turns verbose mode on.

## License

See [mit-pdos/xv6-riscv's sh.c](
https://github.com/mit-pdos/xv6-riscv/blob/riscv/user/sh.c) for the
source code that I started from. The implementation diverged
quite quickly but there are still some ideas of the original code. For
this reason, the copyright is the original copyright plus my own copyright.

```
SPDX-License-Identifier: MIT
```

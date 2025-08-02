# `brainlift`

An interpreter and compiler for the [brainfuck](https://brainfuck.org/brainfuck.html) language.

The compiler uses the [Cranelift](https://cranelift.dev/) compiler backend.

## Usage

Run a program using the interpreter with:
```sh
$ brainlift run examples/helloworld.b
```

Compile a program with:
```sh
$ brainlift compile examples/helloworld.b -o helloworld.o
```
This results in an object file, which still needs to be linked with libc to get a final executable. The `brainlift` compiler uses libc for io and memory-management.\
We can simply use `gcc` as a linker:

```
$ gcc helloworld.o -o helloworld
```

### Configuration

- The size of the array is configurable with the `--array-size` flag, defaulting to the recommended minimum size of 30000. Each cell is one byte.
- Brainfuck leaves the handling of EOF up to implementors. `brainlift` makes this configurable with the `--eof-behaviour` flag. This can either be `ignore` which leaves the current cell unchanged, or `zero` which zeroes the current cell.


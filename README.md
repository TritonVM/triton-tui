# Triton TUI

Terminal User Interface to run and debug programs written for [Triton VM](https://triton-vm.org).

<img alt="Example run of Triton TUI" src="./examples/triton-tui.gif" width="800" />

You might also be interested in the [Triton CLI](https://github.com/TritonVM/triton-cli), which lets
you generate and verify proofs of a program's correct execution.

## Getting Started

Triton TUI tries to be helpful. 🙂 List possible (and required) arguments with `triton-tui --help`.
In the TUI, press `h` to access the help screen.

The [example program](examples/program.tasm), serving as the tutorial, can be run with

```sh
triton-tui examples/program.tasm
```

## Installation

From [crates.io](https://crates.io/crates/triton-tui):

```sh
cargo install triton-tui
```

## Shell Completion

Grab your completion file for
[bash](completions/triton-tui.bash),
[zsh](completions/triton-tui.zsh),
[powershell](completions/triton-tui.powershell),
[fish](completions/triton-tui.fish),
or [elvish](completions/triton-tui.elvish).

Installation depends on your system and shell. 🙇

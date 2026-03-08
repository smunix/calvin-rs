# Calvin-rs

Calvin-rs is a complete, from-scratch rewrite of the hobbes project in Rust. It is a functional programming language with a focus on structured data processing and analysis, inspired by ML-family languages and designed for modern data-intensive applications.

This project is a redesign and reimplementation of the original C++ project, with the goal of creating a more robust, safe, and modern version of the language. The core features of hobbes have been preserved, while the implementation has been updated to leverage the strengths of the Rust programming language.

## Features

*   **Static Typing with Inference:** A powerful Hindley-Milner based type system with type inference, unification, and constraint resolution.
*   **Expression Language:** A rich expression language including literals, variables, let bindings, function application, pattern matching, records, variants, and arrays.
*   **Interpreter:** A tree-walking interpreter for direct evaluation of Calvin expressions.
*   **Structured Storage:** A memory-mapped file-based storage system for structured data, similar to the hobbes fregion/storage system.
*   **Networking and IPC:** TCP and Unix domain socket networking for remote expression evaluation (Net REPL).
*   **Interactive REPL:** An interactive read-eval-print loop (REPL) for evaluating Calvin expressions.
*   **Structured Data Recorder:** A utility for recording structured data produced by applications into Calvin storage files.

## Getting Started

To build the project, you will need to have Rust and Cargo installed. You can then build the project using the following command:

```sh
cargo build
```

This will create two binaries in the `target/debug` directory: `hi` (the interactive interpreter) and `hog` (the structured data recorder).

### Interactive Interpreter (`hi`)

You can start the interactive interpreter by running the `hi` binary:

```sh
./target/debug/hi
```

This will start the REPL, where you can evaluate Calvin expressions:

```
> 1 + 2
3
> let x = 42 in x
42
> :q
Bye!
```

### Structured Data Recorder (`hog`)

The `hog` utility is used to record structured data to files. You can start the data recorder by running the `hog` binary:

```sh
./target/debug/hog --group my_data
```

This will start the data recorder and create a storage group named `my_data` in the current directory.

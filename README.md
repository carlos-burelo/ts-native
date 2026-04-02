# TSN — TypeScript Native

A statically-typed, compiled language with TypeScript-inspired syntax that runs on a native bytecode VM.

```tsn
async function main() {
    const names: str[] = ["Alice", "Bob", "Charlie"]
    for (const name of names) {
        print("Hello, " + name + "!")
    }
}

await main()
````

---

## Features

* **Full pipeline** — lex → parse → type-check → compile → execute
* **Stack-based bytecode VM** with vtable dispatch and reference-counting GC
* **Rich type system** — generics (multi-phase), union/intersection types, nullable types, `never`/`dynamic`
* **Object-oriented** — classes, interfaces, abstract classes, single inheritance, extension methods
* **Async/await** — cooperative multitasking scheduler, `Future<T>`, async generators
* **Pattern matching** — exhaustive `match` with union/enum coverage checks
* **Standard library** — 16+ modules: `std:fs`, `std:http`, `std:crypto`, `std:json`, and more
* **Language Server** — hover, go-to-definition, completions, diagnostics, semantic tokens
* **VS Code extension** — full IDE integration

---

## Architecture

TSN follows a full compilation pipeline:

```
source → lexer → parser → AST → type checker → IR → bytecode → VM
```

* **Lexer** — token stream with contextual keywords and structured literals
* **Parser** — recursive descent with operator precedence and pattern parsing
* **Checker** — multi-phase type inference, symbol resolution, and flow analysis
* **Compiler** — AST lowering into typed IR and bytecode emission
* **VM** — stack-based execution with async scheduler and intrinsic bindings

---

## Design Highlights

* **Multi-phase generics resolution**
  Handles constraint solving, inference, and specialization across multiple passes.

* **Flow-sensitive type narrowing**
  Types are refined across control flow branches (`if`, `match`, guards).

* **Exhaustiveness checking**
  Pattern matching ensures full coverage for unions and enums.

* **Integrated async runtime**
  Cooperative scheduler with `Future<T>` and async generators.

* **Reference-counting memory model**
  Deterministic cleanup integrated with runtime and async execution.

---

## Quick Start

**Requirements**: Rust 1.75+ and Cargo.

```sh
git clone https://github.com/carlos-burelo/ts-native
cd ts-native

# Windows:
powershell -ExecutionPolicy Bypass -File .\scripts\install.ps1

# Linux/macOS:
chmod +x ./scripts/install.sh
./scripts/install.sh
```

Run a file:

```sh
tsn hello.tsn
```

See [Installation](docs/INSTALL.md) and [Getting Started](docs/GETTING_STARTED.md) for a full introduction.
The installer also deploys `tsn-lsp`, enabling automatic detection in the VS Code extension.

---

## Language Overview

### Variables and Types

```tsn
const name: str = "Alice"
let count: int = 0
const pi = 3.14
const price = 9.99d
```

### Classes and Inheritance

```tsn
class Animal {
    name: str
    constructor(name: str) { this.name = name }
    speak(): str { return this.name + " makes a sound" }
}

class Dog extends Animal {
    override speak(): str { return this.name + " barks!" }
}

const dog = new Dog("Rex")
print(dog.speak())
```

### Generics

```tsn
class Box<T> {
    value: T
    constructor(v: T) { this.value = v }
    get(): T { return this.value }
}

const box = new Box<int>(42)
print(box.get())
```

### Async / Await

```tsn
import { sleep } from "std:async"

async function fetchData(id: int): str {
    await sleep(100)
    return "data:" + id
}

const result = await fetchData(1)
print(result)
```

### Pattern Matching

```tsn
const label = match status {
    200 => "OK",
    404 => "Not Found",
    500 => "Server Error",
    _   => "Unknown"
}
```

### Extension Methods

```tsn
extension StringUtils on str {
    shout(): str { return this + "!!!" }
    wordCount(): int { return this.split(" ").length }
}

print("hello world".shout())
print("hello world".wordCount())
```

---

## CLI

```sh
tsn file.tsn                    # run
tsn --debug=lex file.tsn        # tokens
tsn --debug=parse file.tsn      # AST
tsn --debug=check file.tsn      # types
tsn --debug=disasm file.tsn     # bytecode
tsn bench file.tsn              # benchmark pipeline
tsn doctor                      # verify environment
```

---

## Project Structure

```
crates/
  tsn-core/       — AST, opcodes, tokens, source model
  tsn-lexer/      — Tokenizer
  tsn-parser/     — Parser → AST
  tsn-checker/    — Type system + binder
  tsn-compiler/   — IR + bytecode emission
  tsn-vm/         — VM + runtime + scheduler
  tsn-lsp/        — Language Server (LSP)
  tsn-cli/        — CLI interface

tsn-stdlib/       — Standard library (TSN source)
docs/             — Documentation
extension/        — VS Code extension
examples/         — Sample programs
```

---

## Development

TSN was developed as a complete system over ~2 months, focusing on:

* end-to-end compiler pipeline design
* type system architecture and inference
* runtime and async execution model
* developer tooling (CLI + LSP integration)

---

## Documentation

* [Getting Started](docs/GETTING_STARTED.md)
* [Installation](docs/INSTALL.md)
* [Language Specification](docs/TSN-SPEC.md)
* [Standard Library](docs/STDLIB.md)
* [Roadmap](docs/ROADMAP.md)
* [Contributing](CONTRIBUTING.md)
* [Security Policy](SECURITY.md)

---

## Status

**v0.9 (Pre-release)**

* Language semantics: stable
* Compiler + VM: stable
* Type system: mostly complete
* Standard library: in progress
* JIT: experimental

See the [Roadmap](docs/ROADMAP.md) for v1.0 goals.

---

## License

[TSN Source Available License v1.0](LICENSE) — free for personal and non-commercial use.
For commercial licensing, contact the author.

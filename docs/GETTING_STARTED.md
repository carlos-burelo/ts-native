# Getting Started with TSN

TSN (TypeScript Native) is a statically-typed, compiled language with TypeScript-inspired syntax
that compiles to native bytecode. It features a fast single-pass compiler, a stack-based VM,
and a built-in language server.

## Installation

### From Source

**Requirements**: Rust 1.75+ and Cargo.

```sh
git clone https://github.com/[your-username]/ts-native
cd ts-native
cargo build --release
```

Add the binary to your PATH:

```sh
# Linux/macOS
export PATH="$PATH:$(pwd)/target/release"

# Windows (PowerShell)
$env:PATH += ";$(pwd)\target\release"
```

### Verify Installation

```sh
tsn --version
```

---

## Your First Program

Create a file `hello.tsn`:

```tsn
const message: str = "Hello, TSN!"
print(message)
```

Run it:

```sh
tsn hello.tsn
```

Output:
```
Hello, TSN!
```

---

## Basic Syntax

### Variables

```tsn
const name: str = "Alice"         // immutable
let count: int = 0                // mutable
count = count + 1

// Type inference
const pi = 3.14                   // inferred as float
const isActive = true             // inferred as bool
```

### Types

| Type      | Description                          | Example                  |
|-----------|--------------------------------------|--------------------------|
| `int`     | Integer (64-bit)                     | `42`, `-7`               |
| `float`   | Floating point (64-bit)              | `3.14`, `-0.5`           |
| `decimal` | Arbitrary precision decimal          | `1.5d`, `99.99d`         |
| `str`     | UTF-8 string                         | `"hello"`, `'world'`     |
| `char`    | Unicode scalar value                 | `'a'`, `'\n'`            |
| `bool`    | Boolean                              | `true`, `false`          |
| `T[]`     | Array of T                           | `int[]`, `str[]`         |
| `T\|null` | Nullable type                        | `str\|null`              |

### Functions

```tsn
function add(a: int, b: int): int {
    return a + b
}

// Arrow function
const double = (x: int): int => x * 2

// Named arguments
function greet(name: str, age: int): str {
    return name + " is " + age
}
const msg = greet(name: "Bob", age: 30)
```

### Control Flow

```tsn
// if / else
if x > 0 {
    print("positive")
} else if x < 0 {
    print("negative")
} else {
    print("zero")
}

// while
let i = 0
while i < 10 {
    i = i + 1
}

// for
for let j = 0; j < 5; j = j + 1 {
    print(j)
}

// for-of
const nums: int[] = [1, 2, 3]
for n of nums {
    print(n)
}
```

### Match

```tsn
const label = match n {
    1 => "one",
    2 => "two",
    3 => "three",
    _ => "other"
}
```

Match is exhaustive — the compiler will error if a case is missing.

---

## Classes

```tsn
class Animal {
    name: str
    age: int

    constructor(name: str, age: int) {
        this.name = name
        this.age = age
    }

    greet(): str {
        return "Hi, I'm " + this.name
    }
}

class Dog extends Animal {
    breed: str

    constructor(name: str, breed: str) {
        super(name, 0)
        this.breed = breed
    }

    bark(): str {
        return this.name + " says woof!"
    }
}

const dog = new Dog("Rex", "Labrador")
print(dog.bark())   // Rex says woof!
print(dog.greet())  // Hi, I'm Rex  (inherited)
```

---

## Generics

```tsn
function identity<T>(value: T): T {
    return value
}

class Box<T> {
    value: T

    constructor(v: T) {
        this.value = v
    }

    get(): T {
        return this.value
    }
}

const box = new Box<int>(42)
print(box.get())  // 42
```

---

## Async / Await

```tsn
import { sleep } from "std:async"

async function fetchUser(id: int): str {
    await sleep(100)
    return "User " + id
}

async function main() {
    const user = await fetchUser(1)
    print(user)
}

await main()
```

---

## Extension Methods

```tsn
extension StringUtils on str {
    shout(): str {
        return this + "!!!"
    }

    wordCount(): int {
        return this.split(" ").length
    }
}

const s = "hello world"
print(s.shout())       // hello world!!!
print(s.wordCount())   // 2
```

---

## Importing Modules

```tsn
// Standard library
import { Math } from "std:math"
import { readFile, writeFile } from "std:fs"
import { JSON } from "std:json"

// Local files
import { MyClass } from "./my-module"
import { helper } from "../utils/helpers"

// Usage
const data = await readFile("data.txt")
const parsed = JSON.parse(data)
print(Math.sqrt(16.0))  // 4
```

---

## Error Handling

```tsn
import { Error } from "std:error"

function divide(a: int, b: int): float {
    if b === 0 {
        throw new Error("Division by zero")
    }
    return a / b
}

try {
    const result = divide(10, 0)
} catch (e) {
    print("Error: " + e.message)
}
```

---

## Resource Management

```tsn
import { Disposable } from "std:dispose"

class Connection implements Disposable {
    constructor() { print("Connected") }
    dispose() { print("Disconnected") }
}

// Automatically disposed when block exits
using conn = new Connection()
// ... use conn ...
// "Disconnected" printed automatically
```

---

## CLI Reference

```sh
# Run a file
tsn file.tsn

# Debug pipeline stages
tsn --debug=lex file.tsn     # token stream
tsn --debug=parse file.tsn   # AST
tsn --debug=check file.tsn   # type info
tsn --debug=disasm file.tsn  # bytecode
tsn --debug=lsp file.tsn     # LSP analysis

# Benchmark
tsn bench file.tsn
```

---

## Language Server (LSP)

TSN includes a built-in language server for IDE integration.

```sh
tsn-lsp
```

### VS Code Integration

Install the TSN extension from the `extension/` directory:

```sh
cd extension
npm install
npm run package
code --install-extension tsn-*.vsix
```

Features:
- Syntax highlighting
- Hover documentation
- Go-to-definition
- Auto-completion (members, scope, imports)
- Inline diagnostics (type errors)
- Semantic token coloring

---

## Next Steps

- [Language Specification](./TSN-SPEC.md) — Full grammar and type system
- [Standard Library Reference](./STDLIB.md) — All built-in modules
- [Roadmap](./ROADMAP.md) — What's coming next

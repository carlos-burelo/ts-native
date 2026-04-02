# TSN Roadmap

This document tracks what is implemented, what is in progress, and what is planned for future versions.

---

## Current Version: 0.4 (Alpha)

**Status**: Active development — language semantics and compiler are functional.
Core features are stable. Standard library is growing.

---

## What's Implemented (v0.4)

### Compiler & Runtime
- [x] Full pipeline: lex → parse → type-check → compile → execute
- [x] Stack-based bytecode VM
- [x] Vtable dispatch for virtual method calls
- [x] Garbage collection (reference counting)
- [x] Async task scheduler (cooperative multitasking)
- [x] Native Rust FFI for stdlib modules

### Type System
- [x] Primitive types: `int`, `float`, `decimal`, `char`, `str`, `bool`
- [x] Nullable types (`T | null`) with nullish coalescing (`??`)
- [x] Union types
- [x] Intersection types (collapse to `never` for incompatible primitives)
- [x] `unknown` as top type
- [x] `never` as bottom type
- [x] Generics — 5 phases:
  - [x] Explicit type arguments (`identity<int>(x)`)
  - [x] Type inference from arguments
  - [x] Member access on generic types (`box.value` where `box: Box<T>`)
  - [x] Constraint validation (`T extends Comparable`)
  - [x] Generic inheritance (`class PriorityQueue<T> extends Queue<T>`)
- [x] Method-level type parameters (`then<R>(fn: T => R): Future<R>`)
- [x] Type aliases
- [x] `newtype` / nominal types
- [x] Type predicates (`x is T`)

### Object-Oriented
- [x] Classes with fields, constructors, methods
- [x] Single inheritance (`extends`)
- [x] Interface declaration and `implements`
- [x] Abstract classes and methods
- [x] `private`, `protected`, `public` visibility
- [x] `static` members and methods
- [x] `readonly` fields
- [x] `override` keyword (required for inherited methods)
- [x] Getters (`get`) and setters (`set`)
- [x] Structural subtyping for object literals
- [x] Extension methods on any type

### Control Flow
- [x] `if / else if / else`
- [x] `while`, `for`, `do-while`
- [x] `for-of`, `for-await`
- [x] `match` with exhaustiveness checking
- [x] `try / catch / finally`
- [x] `throw`
- [x] `break`, `continue`
- [x] `return`

### Functions
- [x] Named functions with return type annotation
- [x] Arrow functions (`=>`)
- [x] Closures (captures by reference)
- [x] Higher-order functions
- [x] Optional parameters
- [x] Default parameters
- [x] Spread parameters (`...args: T[]`)
- [x] Named arguments at call site
- [x] `async` functions returning `Future<T>`
- [x] Generator functions (`function*`) — sync
- [x] Async generator functions (`async function*`)

### Pattern Matching & Destructuring
- [x] `match` expression
- [x] Wildcard `_` in match
- [x] Array destructuring (`const [a, b] = arr`)
- [x] Object destructuring (`const { x, y } = point`)
- [x] `_` placeholder in destructuring (skip elements)
- [x] Default values in destructuring

### Operators
- [x] Arithmetic: `+`, `-`, `*`, `/`, `%`
- [x] Comparison: `===`, `!==`, `<`, `>`, `<=`, `>=`
- [x] Logical: `&&`, `||`, `!`
- [x] Bitwise: `&`, `|`, `^`, `~`, `<<`, `>>`
- [x] Nullish coalescing: `??`
- [x] Optional chaining: `?.`
- [x] Pipeline: `|>` (Hack-style with `_` placeholder)
- [x] `instanceof`
- [x] `typeof`
- [x] `in`
- [x] `new`

### Modules & Imports
- [x] Named exports (`export function`, `export class`, etc.)
- [x] Named imports (`import { X } from "..."`)
- [x] Default export/import
- [x] Relative module resolution (`./file`, `../dir/file`)
- [x] Standard library import (`std:module`)
- [x] `namespace` declarations

### Resource Management
- [x] `using` statement (sync `Disposable`)
- [x] `await using` (async `AsyncDisposable`)

### Standard Library
- [x] `std:async` — spawn, sleep
- [x] `std:collections` — Range
- [x] `std:console` — log, warn, error, info
- [x] `std:crypto` — sha256, sha512, hmac, uuid, base64, randomBytes
- [x] `std:dispose` — Disposable, AsyncDisposable
- [x] `std:fs` — readFile, writeFile, exists, mkdir, readDir, remove, copy, rename
- [x] `std:http` — get, post, server
- [x] `std:io` — readLine, readAll, write, flush
- [x] `std:json` — JSON.parse, JSON.stringify
- [x] `std:math` — Math.* namespace
- [x] `std:path` — Path.* namespace
- [x] `std:reflect` — metadata API
- [x] `std:result` — Result<T, E>, ok, err
- [x] `std:sys` — Sys.platform, args, env, exit
- [x] `std:test` — assert, assertEqual, fail
- [x] `std:time` — Time.now, millis, toISOString

### Developer Experience
- [x] Language Server Protocol (LSP)
  - [x] Hover documentation (symbols, members, `this`, params)
  - [x] Go-to-definition
  - [x] Auto-completion (scope, members, `this.`, imports)
  - [x] Semantic token coloring
  - [x] Inline error diagnostics
  - [x] Workspace symbol search
- [x] VS Code extension
- [x] CLI debug flags (`--debug=lex/parse/check/disasm/lsp`)
- [x] Benchmarking tool

---

## In Progress

### Type System
- [ ] Mapped types (`{ [K in keyof T]: U }`)
- [ ] Conditional types (`T extends U ? A : B`)
- [ ] Template literal types (`` `prefix-${T}` ``)
- [ ] Variance annotations (`in T`, `out T`)
- [ ] `keyof T` operator
- [ ] `this` type in inheritance

### Language Features
- [ ] ADTs / Sum types (tagged unions as first-class citizens)
- [ ] `record` type (immutable value types)
- [ ] `?` error propagation operator (short-circuit `Result` chains)
- [ ] Default interface methods
- [ ] `const` parameters
- [ ] Decorators

### Standard Library
- [ ] `std:temporal` — Temporal API (advanced date/time)
- [ ] `std:net` — TCP/UDP sockets
- [ ] `std:stream` — Lazy stream API for generators
- [ ] `std:db` — Database client abstraction

### Developer Experience
- [ ] Watch mode (`tsn --watch file.tsn`)
- [ ] Source maps
- [ ] Incremental compilation
- [ ] Project configuration file (`tsn.toml`)
- [ ] Package manager integration

---

## Planned (v1.0 Goals)

### Language Cleanup (Breaking Changes)
These changes will make the language more consistent before v1.0:

- [ ] **Remove `var`**: Only `let` and `const` — `var` adds no value in a compiled language
- [ ] **Remove `==`**: Keep only `===` — a statically typed language needs no coercive equality
- [ ] **Remove `delete` operator**: Breaks static analysis; use `Map.delete()` instead
- [ ] **Remove `void` operator**: Has no use in TSN's type system
- [ ] **Numeric type hierarchy**: Define clear coercion rules between `int`, `float`, `decimal`, `bigint`
- [ ] **Pipeline unification**: One semantics for `|>`, not two

### Tooling
- [ ] Native binary compilation (LLVM or Cranelift backend)
- [ ] Debug adapter protocol (DAP) for step-through debugging
- [ ] Formatter (`tsn fmt`)
- [ ] Linter (`tsn lint`) with configurable rules
- [ ] Documentation generator (`tsn doc`)

### Ecosystem
- [ ] Package registry
- [ ] `std:orm` — Type-safe database ORM
- [ ] `std:ui` — Terminal UI components
- [ ] WASM compilation target

---

## Version History

| Version | Date       | Highlights                                                  |
|---------|------------|-------------------------------------------------------------|
| 0.4     | 2026-03    | Vtable dispatch, full generics (5 phases), decimal type, extension methods, LSP quality |
| 0.3     | 2025       | Async/await, generators, match exhaustiveness, namespaces   |
| 0.2     | 2025       | Classes, inheritance, interfaces, type checker              |
| 0.1     | 2024       | Initial compiler: primitives, functions, basic types        |

---

## Design Principles (guiding v1.0)

1. **One semantics, many contexts** — a keyword means the same thing everywhere
2. **Explicit > implicit** — async boundaries, resource lifetimes, side effects must be visible
3. **Types are documentation** — the type system must be precise enough to trust
4. **No JavaScript baggage** — remove features inherited from JS that don't fit a compiled language
5. **Lean stdlib** — 15–20 focused modules; no kitchen-sink API

---

*For questions, feature requests, or contributions, open an issue on GitHub.*

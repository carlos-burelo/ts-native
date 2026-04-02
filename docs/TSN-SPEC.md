# TSN Language Specification
**Version**: 0.4 (Draft — Design Corrections & Implementation Plan)
**Date**: 2026-03-22

---

## Table of Contents

1. [Design Principles](#1-design-principles)
2. [Type System](#2-type-system)
   - 2.1 Primitive Types
   - 2.2 Composite Types
   - 2.3 Nominal vs Structural Typing
   - 2.4 Nullable Normalization (invariant)
   - 2.5 Intersection Types
   - 2.6 Literal Types & Widening
3. [Enums](#3-enums)
4. [Type Aliases & Newtypes](#4-type-aliases--newtypes)
5. [The `_` Placeholder](#5-the-_-placeholder)
6. [Named Arguments](#6-named-arguments)
7. [Async & Future\<T\>](#7-async--futuret)
8. [Match Expressions & Exhaustiveness](#8-match-expressions--exhaustiveness)
9. [Type Predicates](#9-type-predicates-x-is-t)
10. [Variance Annotations](#10-variance-annotations)
11. [never as Bottom Type](#11-never-as-bottom-type)
12. [this Type in Inheritance](#12-this-type-in-inheritance)
13. [Advanced Type System](#13-advanced-type-system)
    - 13.1 keyof T
    - 13.2 Mapped Types
    - 13.3 Conditional Types
    - 13.4 Template Literal Types
14. [Stdlib: Json Type](#14-stdlib-json-type)
15. [Resource Management: using & Disposable](#15-resource-management-using--disposable)
16. [char: Unicode Semantics](#16-char-unicode-semantics)
17. [Implementation Roadmap](#17-implementation-roadmap)

---

## 1. Design Principles

These principles guide all language decisions. When in doubt, refer here.

**1. Una semántica, varios contextos**
A symbol or keyword must have the same fundamental semantics in every context where it appears. If `_` means "discard" in destructuring, it must mean "discard" everywhere.

**2. Explicit over implicit, where it matters**
Type inference is good. Behavior inference (side effects, resource management, async boundaries) should be explicit. The programmer must be able to read the code and understand what happens without running it.

**3. Types are verifiable documentation**
A type system that accepts `unknown[]` in critical code is a type system that lies when you need it most. Prefer precise types with APIs that ease the transition from imprecise ones.

**4. The compiler tells you what to do**
A good error message doesn't just say "this is wrong" — it says "here is what you should write instead". Error ergonomics are part of the language design.

**5. Soundness is a goal, not an excuse**
TypeScript chose deliberate unsoundness for pragmatism. TSN can be more sound, but must do so without sacrificing expressiveness. If the type system rejects semantically correct code, the type system has a bug.

**6. No silent failures**
A feature that exists in the parser but is silently ignored in the compiler is worse than not having the feature — it creates false expectations. Every syntactic construct must either work or produce a compile error.

---

## 2. Type System

### 2.1 Primitive Types

| Type      | Description                                | Literal Syntax     |
|-----------|--------------------------------------------|--------------------|
| `int`     | 64-bit signed integer                      | `42`, `-1`         |
| `float`   | 64-bit IEEE 754 double                     | `3.14`, `-0.5`     |
| `decimal` | Arbitrary-precision decimal (rust_decimal) | `0.1d`, `1.5d`     |
| `bigint`  | Arbitrary-precision integer                | `123n`, `0xFFn`    |
| `str`     | UTF-8 string                               | `"hello"`, `` `x${y}` `` |
| `char`    | Unicode scalar value (U+0000–U+10FFFF)     | `'A'`, `'🎉'`      |
| `bool`    | Boolean                                    | `true`, `false`    |
| `null`    | Null value                                 | `null`             |
| `void`    | No value (function returns nothing)        | —                  |
| `never`   | Bottom type (unreachable)                  | —                  |
| `unknown` | Top type (unchecked, requires narrowing)   | —                  |

**Note on `decimal` suffix**: `0.1d` uses `d` suffix. Java uses `d` for double; C# uses `m`. The `d` suffix is intentional in TSN (d for decimal), but developers coming from Java must be aware of the difference.

### 2.2 Composite Types

```tsn
T[]                          // Array of T
T | U                        // Union: T or U
T & U                        // Intersection: T and U
[T, U, V]                    // Tuple: fixed-length sequence
T?                           // Nullable: T | null (syntactic sugar only)
Generic<T>                   // Generic instantiation
(x: T, y: U) => V            // Function type
{ name: str, age: int }      // Object/record type
```

### 2.3 Nominal vs Structural Typing

TSN uses a hybrid model:

| Context        | Typing      | Rule                                                      |
|----------------|-------------|-----------------------------------------------------------|
| Object literals| Structural  | `{ name: str }` is compatible with any superset          |
| Interfaces     | Structural  | Compatibility checked by shape                           |
| Classes        | Nominal     | `Dog` is not `Cat` even if same fields                   |
| `newtype`      | Nominal     | Opaque wrapper, incompatible with underlying type        |
| `type` alias   | Structural  | Transparent alias, compatible with underlying type       |

### 2.4 Nullable Normalization (compiler invariant)

`T?` in source code is syntactic sugar. The binder resolves it immediately to `Union([T, Null])` via `Type::make_nullable()`. **The checker must never receive a `TypeKind::Nullable` node.**

This invariant is enforced by a `debug_assert` at the checker boundary:
```rust
debug_assert!(
    !matches!(ty.0, TypeKind::Nullable(_)),
    "Nullable must be normalized before reaching the checker"
);
```

Consequence: `is_nullable()` only needs to check `TypeKind::Null` and `TypeKind::Union` — never `TypeKind::Nullable`.

### 2.5 Intersection Types

`A & B` means "a value that satisfies both A and B simultaneously."

**Compatibility rules**:
- `T <: A & B` iff `T <: A` AND `T <: B`
- `A & B <: T` iff `A <: T` OR `B <: T`
- `str & int` → `never` (incompatible primitives)
- `{ name: str } & { age: int }` → `{ name: str, age: int }` (merged object)

**Display**: `A & B` for two types; complex unions are parenthesized: `(A | B) & C`.

### 2.6 Literal Types & Widening

Literal types (`1`, `"hello"`, `true`) are narrower than their base types. Assignment rules:

```tsn
const x = 1           // type: LiteralInt(1) — inferred narrow
let y: int = 1        // type: int — widens on annotation
const z: int = x      // ✓ — LiteralInt(1) <: int
```

---

## 3. Enums

### 3.1 Declaration

```tsn
enum Color {
    Red = 0,
    Green = 1,
    Blue = 2
}

enum Direction {
    North,   // auto-increments from 0
    South,
    East,
    West
}
```

### 3.2 Semantics

Enums are **nominal in the checker, integers in the VM**. The raw integer value is an implementation detail.

**Allowed operations**:
```tsn
const c: Color = Color.Red        // ✓ — assign enum variant
const b: bool = c === Color.Red   // ✓ — identity comparison
const raw: int = c.rawValue       // ✓ — explicit extraction
const c2: Color = Color(0)        // ✓ — construct from raw value (runtime validates)
for (const v of Color) { ... }   // ✓ — iterate variants
```

**Rejected operations**:
```tsn
const c: Color = 0                // ✗ — Error: "cannot assign int to Color; use Color(0)"
const n: int = Color.Red + 1      // ✗ — Error: "arithmetic not allowed on enum type Color"
const c2: Color = Status.Active   // ✗ — Error: "Color and Status are distinct enum types"
```

**Checker rule**: In `check_expr(Binary)`, if either operand's base type is a `Named` type found in `bind.enum_members`, and the operator is `Add|Sub|Mul|Div|Mod|Pow|BitAnd|BitOr|BitXor|Shl|Shr|UShr`, emit an error.

### 3.3 `.rawValue` and `Color(n)` constructor

Every enum type `E` with variants of type `int` automatically gets:
- `.rawValue: int` — returns the underlying integer
- `E(n: int): E` — static constructor; panics at runtime if `n` is not a valid variant

---

## 4. Type Aliases & Newtypes

### 4.1 Structural alias: `type`

```tsn
type ID = str                    // transparent: ID and str are interchangeable
type Fn<T> = (x: T) => T        // generic alias
type StringOrNumber = str | int  // union alias
```

`type` aliases are structural — a value of the aliased type is directly assignable to and from the alias without any conversion.

### 4.2 Nominal alias: `newtype`

```tsn
newtype UserId = str      // opaque: UserId is NOT assignable to/from str
newtype ProductId = str   // UserId and ProductId are distinct types
```

**Value construction**:
```tsn
const id: UserId = UserId("abc-123")   // constructor function — explicit wrap
const raw: str = id.value              // .value — explicit unwrap
const raw2: str = id as str            // cast — also valid
```

**Compatibility rules**:
```tsn
const s: str = "hello"
const id: UserId = s            // ✗ — Error: "cannot assign str to UserId; use UserId(s)"
const id2: ProductId = id       // ✗ — Error: "UserId and ProductId are distinct newtypes"
const raw: str = id             // ✗ — Error: "cannot assign UserId to str; use id.value or id as str"
```

**Checker implementation**:
- `newtype` creates a symbol with `SymbolKind::Newtype` and a special `Type::Newtype(name, inner_type)` variant
- `types_compatible` returns `false` for `(Newtype(n1, _), Newtype(n2, _))` where `n1 != n2`
- `types_compatible` returns `false` for `(Newtype(_, _), inner)` or `(inner, Newtype(_, _))` unless cast is explicit (`Expr::As`)

---

## 5. The `_` Placeholder

`_` is a **reserved keyword**, not an identifier. It has one unified semantic: **"this value is intentionally discarded."**

### 5.1 Contexts

**Destructuring** — skip a position:
```tsn
const [a, _, c] = arr      // skip index 1
const { name, _: _ } = obj // (using _ in object destructuring is unusual, use renaming)
```

**Match wildcard** — catch-all:
```tsn
match value {
    1 => "one",
    _ => "other"   // catches everything not previously matched
}
```

**Pipeline placeholder** — position of piped value:
```tsn
x |> f(_, y)   // equivalent to f(x, y) — _ is NOT a variable
```

### 5.2 Rules

- `_` as **l-value**: always valid — value is computed and discarded
- `_` as **r-value**: **compile error** — `"cannot use _ as a value"`
- `const _ = expr`: valid, but the checker emits a warning: `"value of expression is discarded; omit assignment or use void expr if intentional"`
- Pipeline `_` is **compile-time syntactic sugar** — the compiler inlines the piped value at the `_` position. No local variable is created.

### 5.3 Checker enforcement

In `check_expr(Identifier { name: "_", .. })` when NOT in assignment-target position:
```
Error: cannot use _ as a value
Hint: _ is the discard placeholder. Use a named binding if you need the value.
```

---

## 6. Named Arguments

### 6.1 Syntax

```tsn
function createUser(name: str, age: int, city: str): str { ... }

// All positional (unchanged)
createUser("Carol", 28, "LA")

// All named — any order
createUser(city: "LA", name: "Carol", age: 28)

// Mixed — positionals must come first
createUser("Carol", age: 28, city: "LA")   // ✓
createUser("Carol", city: "LA", age: 28)   // ✓ — named can be reordered after positional
createUser(name: "Carol", 28, "LA")        // ✗ — Error: "positional arg cannot follow named arg"
```

### 6.2 Rules

1. All positional arguments map to parameters by index (left to right)
2. Named arguments map to parameters by name
3. A parameter cannot receive both a positional value and a named value
4. After the last positional argument, all remaining arguments must be named
5. Named arguments after the positional prefix can appear in any order

### 6.3 Implementation

In `check_expr_no_record(Expr::Call)`, when `has_named`:
1. Build a mapping `param_index → arg` by scanning positional args left-to-right
2. For each named arg, find the param with matching name; error if not found or already bound
3. Verify no positional arg follows a named arg
4. Fill remaining positionals by index

In `compile_expr(Expr::Call)`, when named args are present:
- Determine the resolved order of arguments (matching to parameter positions)
- Emit arguments in parameter declaration order (not call-site order)
- Named args are purely a caller-side convenience — the callee receives them positionally

---

## 7. Async & Future\<T\>

### 7.1 Return type inference

Both forms are equivalent:

```tsn
async function f(): int { return 42 }         // ✓ — compiler treats as Future<int>
async function f(): Future<int> { return 42 } // ✓ — explicit
```

**Rule**: In an `async` function, the declared return type annotation `T` is automatically wrapped in `Future<T>` for type-checking purposes. If the annotation is already `Future<T>`, it is not double-wrapped.

**In the checker** (`check_decl(Function)`): if `f.modifiers.is_async` and `return_type` is `Some(T)` where `T` is not `Generic("Future", [_])`, wrap it: `expected_return_type = Future<T>`.

### 7.2 await type stripping

`await expr` where `expr: Future<T>` produces type `T`.
`await expr` where `expr: T` (not a Future) is a warning: `"await on non-Future value has no effect"`.

---

## 8. Match Expressions & Exhaustiveness

### 8.1 Syntax

```tsn
const result = match value {
    pattern1 if guard => expr,
    pattern2 => { stmt; stmt; expr },
    _ => default_expr
}
```

### 8.2 Exhaustiveness rules

**Over enums**: All variants must be covered, or a wildcard `_` must be present.
```tsn
enum Color { Red, Green, Blue }

match color {
    Color.Red   => "red",
    Color.Green => "green"
    // Error: match is not exhaustive — missing: Color.Blue
}
```

**Over bool**: Both `true` and `false` must be covered, or `_`.

**Over union types**: Every member type must be covered, or `_`.
```tsn
type Shape = Circle | Rectangle | Triangle

match shape {
    Circle    => ...,
    Rectangle => ...
    // Error: match is not exhaustive — missing: Triangle
}
```

**Over str/int/float**: Cannot be exhaustive. `_` is always required.
```tsn
match n {
    1 => "one",
    2 => "two"
    // Error: match over int requires a wildcard _ arm
}
```

### 8.3 Return type

The type of a `match` expression is the union of all arm body types. If all arms return the same type, the union collapses to that type.

```tsn
const s: str = match n {    // type: str
    1 => "one",
    _ => "other"
}
```

---

## 9. Type Predicates (`x is T`)

### 9.1 Syntax

```tsn
function isString(x: unknown): x is str {
    return typeof x === "str"
}

function isError(x: unknown): x is Error {
    return x instanceof Error
}
```

### 9.2 Semantics

A function with return type `param is T` is a **type predicate**. At call sites, the checker uses the boolean return value to narrow the type of the referenced argument.

```tsn
const val: unknown = getValue()

if (isString(val)) {
    val.toUpperCase()   // ✓ — val is narrowed to str in this branch
}
```

**In the false branch**: the type of `val` is narrowed to `original_type` minus `str` (i.e., if `val: str | int`, after `isString` returns false, `val: int`).

### 9.3 Implementation

1. Parser: in function return type position, parse `identifier is TypeNode` as `TypeNode::Predicate { param_name, ty }`
2. Binder: store `SymbolKind::Function` with `return_predicate: Option<(String, Type)>`
3. Checker: in `extract_narrowings`, handle `Expr::Call` where callee resolves to a predicate function — add narrowing for the referenced argument

---

## 10. Variance Annotations

### 10.1 Syntax

```tsn
class ImmutableList<out T> { ... }   // covariant
class Writer<in T> { ... }           // contravariant
class Ref<T> { ... }                 // invariant (default)
```

### 10.2 Semantics

**Covariant (`out T`)**: `C<Dog>` is assignable to `C<Animal>` if `Dog <: Animal`. `T` may only appear in output positions (return types). Attempting to use `T` as a method parameter produces an error.

**Contravariant (`in T`)**: `C<Animal>` is assignable to `C<Dog>` if `Dog <: Animal`. `T` may only appear in input positions (parameter types).

**Invariant (default)**: No subtyping relationship between `C<Dog>` and `C<Animal>`.

### 10.3 Array variance

`Array<T>` is **invariant** (mutable). Use `ReadonlyArray<out T>` for covariant collections:

```tsn
const dogs: Array<Dog> = [new Dog()]
const animals: ReadonlyArray<Animal> = dogs   // ✓ covariant
animals.push(new Cat())                        // ✗ 'push' does not exist on ReadonlyArray
```

---

## 11. `never` as Bottom Type

### 11.1 Rules

- `never <: T` for every type `T` — never is assignable to anything
- No type is assignable to `never` except `never` itself
- After `throw expr`, the continuation has type `never`
- `str & int` simplifies to `never` (checker warning: "intersection of incompatible types is never")
- `Array<never>` is the type of the empty array literal `[]` before context-driven widening

### 11.2 In match

A `match` over a `never`-typed subject requires zero arms and is trivially exhaustive.

### 11.3 Unreachable code detection

```tsn
function f(): int {
    throw new Error("nope")
    return 42   // Warning: unreachable code after throw
}
```

---

## 12. `this` Type in Inheritance

`this` as a return type resolves to the **concrete receiver type**, enabling fluent/builder patterns without losing type information.

```tsn
class Builder {
    setName(n: str): this {
        this.name = n
        return this
    }
}

class ExtendedBuilder extends Builder {
    setExtra(n: int): this { ... }
}

const b = new ExtendedBuilder()
b.setName("x").setExtra(42)   // ✓ — setName returns ExtendedBuilder, not Builder
```

**Checker rule**: When `infer_type(Member)` resolves a method whose return type is `TypeKind::This`, substitute `This` with the inferred type of the object expression.

---

## 13. Advanced Type System

### 13.1 `keyof T`

Produces the union of property name literal types:

```tsn
interface User { name: str, age: int }
type UserKey = keyof User   // → "name" | "age"
```

### 13.2 Mapped Types

```tsn
type Partial<T> = { [K in keyof T]?: T[K] }
type Readonly<T> = { readonly [K in keyof T]: T[K] }
type Record<K extends str, V> = { [key: K]: V }
type Pick<T, K extends keyof T> = { [P in K]: T[P] }
type Omit<T, K extends keyof T> = { [P in keyof T if P != K]: T[P] }
```

Standard library provides: `Partial<T>`, `Required<T>`, `Readonly<T>`, `Pick<T,K>`, `Omit<T,K>`, `Record<K,V>`, `NonNullable<T>`.

### 13.3 Conditional Types

```tsn
type IsArray<T> = T extends Array<unknown> ? true : false
type UnwrapArray<T> = T extends Array<infer U> ? U : T
type ReturnType<F> = F extends (...args: unknown[]) => infer R ? R : never
type Parameters<F> = F extends (...args: infer P) => unknown ? P : never
```

**Distribution**: Over naked type parameters, conditional types distribute over unions:
```tsn
type Wrap<T> = T extends unknown ? T[] : never
type R = Wrap<str | int>   // → str[] | int[]
```

### 13.4 Template Literal Types

```tsn
type EventName = `on${str}`
type Route<S extends str> = `/api/${S}`
type CSSMargin = `${"margin" | "padding"}-${"top" | "bottom" | "left" | "right"}`
// → "margin-top" | "margin-bottom" | "margin-left" | "margin-right" | "padding-..."
```

---

## 14. Stdlib: Json Type

`unknown[]` is not acceptable for JSON data. TSN provides a first-class `Json` type.

```tsn
// Defined in std:json
type JsonPrimitive = null | bool | int | float | str
type Json = JsonPrimitive | Json[] | { [key: str]: Json }
```

**API**:
```tsn
import { JSON } from 'std:json'

// Untyped parse — returns Json
const data: Json = JSON.parse(text)

// Typed parse — validates shape at runtime, returns T | null
const user = JSON.parseAs<User>(text)   // User | null

// Stringify
const s: str = JSON.stringify(value)    // any Json-serializable value
```

---

## 15. Resource Management: `using` & Disposable

```tsn
interface Disposable {
    dispose(): void
}

interface AsyncDisposable {
    dispose(): Future<void>
}

// Synchronous
using const conn = db.connect()
// conn.dispose() called automatically on scope exit (return, throw, or block end)

// Asynchronous
await using const conn = await db.connectAsync()
// conn.dispose() is awaited on scope exit
```

**Checker rule**: The right-hand side of `using` must implement `Disposable`. The right-hand side of `await using` must implement `AsyncDisposable`. If neither is implemented, error: `"type X does not implement Disposable; add a dispose(): void method"`.

---

## 16. `char`: Unicode Semantics

A `char` in TSN is a **Unicode scalar value** — any code point from U+0000 to U+10FFFF excluding surrogates (U+D800–U+DFFF). This matches Rust's `char` type.

```tsn
const c: char = 'A'                // ✓ ASCII
const emoji: char = '🎉'           // ✓ Emoji (U+1F389)
const invalid: char = '\uD800'     // ✗ — compile error: surrogate is not a scalar value
```

**Operations**:
```tsn
c.codepoint(): int          // → 65 for 'A'
char(65): char              // constructor from codepoint; panics if surrogate
c.toString(): str           // → "A"
c.isAlpha(): bool
c.isDigit(): bool
c.isWhitespace(): bool
c.toUppercase(): char
c.toLowercase(): char
```

**String indexing returns `char`**:
```tsn
const c: char = "hello"[0]   // → 'h'
const s: str = "hello"       // s.chars() → Array<char>
```

---

## 17. Implementation Roadmap

### Phase 0 — Critical Fixes (no new syntax)

| # | Fix | File(s) | Effort |
|---|-----|---------|--------|
| 0.1 | Add `debug_assert` for Nullable invariant | `checker/mod.rs` | 30min |
| 0.2 | Fix `Intersection` in `Type::Display` | `types.rs` | 1h |
| 0.3 | Fix `Intersection` in `types_compatible` | `checker/compat.rs` | 3h |
| 0.4 | Implement `Expr::Update` (++ / --) in compiler | `compiler/emit/exprs/mod.rs` | 2h |
| 0.5 | Enforce `_` cannot be used as r-value | `checker_expressions/check.rs` | 2h |
| 0.6 | Emit warning for `const _ = expr` | `checker/stmts.rs` | 1h |

**Exit criterion**: All existing examples compile and pass. No `???` in error messages.

### Phase 1 — Semantic Clarity (minimal syntax changes)

| # | Feature | File(s) | Effort |
|---|---------|---------|--------|
| 1.1 | Named args: order verification & out-of-order mapping | `checker_expressions/check.rs`, `compiler/emit/exprs/mod.rs` | 2 days |
| 1.2 | Async: implicit `Future<T>` wrapping | `checker/stmts.rs` | 1 day |
| 1.3 | Enum: reject arithmetic ops | `checker_expressions/check.rs` | 3h |
| 1.4 | Enum: `.rawValue` and `Color(n)` constructor | `stdlib_descriptors/`, vm | 1 day |
| 1.5 | Match: exhaustiveness checking | `checker/stmts.rs` | 2 days |
| 1.6 | `newtype` keyword (lexer → parser → binder → checker) | all layers | 3 days |

**Exit criterion**: Section 3 and Section 13 of `ultimate-test.tsn` don't have "Skipped" comments.

### Phase 2 — Type System Extensions

| # | Feature | File(s) | Effort |
|---|---------|---------|--------|
| 2.1 | Type predicates `x is T` | parser, binder, checker narrowing | 2 days |
| 2.2 | `never` propagation post-throw | `checker/stmts.rs` | 4h |
| 2.3 | Intersection collapse to `never` | `checker/compat.rs` | 4h |
| 2.4 | `this` covariant return type | `checker_expressions/infer.rs` | 1 day |
| 2.5 | Variance annotations (`out T`, `in T`) | all layers | 3 days |
| 2.6 | `ReadonlyArray<out T>` in stdlib | `stdlib_descriptors/`, vm | 4h |
| 2.7 | `Json` type in `std:json` | `tsn-modules/stdlib/json.rs` | 1 day |
| 2.8 | `Disposable` interface + `using` checker | `checker/stmts.rs` | 1 day |

**Exit criterion**: `api-showcase.tsn` works without `unknown[]`.

### Phase 3 — Advanced Type System

| # | Feature | Effort |
|---|---------|--------|
| 3.1 | `keyof T` | 3 days |
| 3.2 | Index types `T[K]` | 2 days |
| 3.3 | Mapped types | 1 week |
| 3.4 | Conditional types + `infer` | 2 weeks |
| 3.5 | Template literal types | 1 week |
| 3.6 | Stdlib utility types (`Partial<T>`, `Readonly<T>`, `ReturnType<F>`, etc.) | 1 week |

**Exit criterion**: Users can implement type-safe ORMs and generic utilities without `unknown`.

### Phase 4 — Developer Experience

| # | Feature | Notes |
|---|---------|-------|
| 4.1 | Extension methods (complete implementation) | Parser exists; checker+compiler missing |
| 4.2 | `await using` for AsyncDisposable | Requires Phase 2 |
| 4.3 | Method overloading | Add `overloads: Vec<OverloadSig>` to FunctionDecl |
| 4.4 | Discriminated union auto-narrowing | Based on `kind: "circle"` literal field |
| 4.5 | LSP: exhaustiveness warnings in match | Requires Phase 1 |

---

*End of TSN Language Specification v0.4*

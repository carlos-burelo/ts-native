## Domain

TSN is a statically-typed compiled programming language with:

* single-pass compiler
* stack-based VM
* strong type system
* TypeScript-like syntax (but stricter and more sound)

---

## Architecture

tsn-lexer     → tokenization
tsn-parser    → AST construction
tsn-checker   → type system, validation, inference
tsn-compiler  → bytecode emission
tsn-vm        → runtime execution
tsn-lsp       → language server (IDE features)
tsn-core      → shared AST, tokens, core structures
tsn-types     → runtime values and type representations
tsn-cli       → pipeline orchestration

---

## Core Principles

* One semantics per construct (no context-dependent meaning)
* Explicit over implicit (especially async, resources, side effects)
* Types must be reliable documentation
* No silent failures
* Soundness over convenience

---

## Global Invariants

* Nullable types MUST be normalized before reaching the checker
* Checker must never receive unnormalized types
* Every syntax feature must either:

  * compile correctly, or
  * produce a compile-time error
* No partially implemented features

---

## Code Rules

* DRY
* KISS
* No comments unless explicitly requested
* Prefer pure functions
* Avoid side effects unless required

---

## File Structure Rules

* Max file size: 400 lines
* If exceeded → MUST split into submodules
* No god-files under any circumstance
* Modules must have a single clear responsibility

---

## Naming Rules

* No magic strings
* No hardcoded values unless intrinsic to the language
* Use constants, enums, or well-defined tables

---

## Output Rules

* Default: code only
* No explanations unless explicitly requested
* Minimal output
* Prefer diffs or isolated functions over full files

---

## Context Handling

* Never assume full project context
* Work only with provided scope
* Infer missing pieces conservatively

---

## Task Modes (implicit skills)

### type-checker

Focus:

* type compatibility
* inference
* narrowing
* soundness

Ignore:

* formatting
* syntax unless blocking

---

### compiler

Focus:

* bytecode correctness
* stack behavior
* instruction ordering

Ignore:

* type system

---

### vm

Focus:

* runtime correctness
* execution semantics
* memory and task scheduling

---

### spec-audit

Focus:

* mismatch between spec and implementation
* semantic inconsistencies

Output:

* inconsistency
* required fix

---

### refactor

Goals:

* reduce duplication
* improve modularity
* enforce file size limits
* eliminate god-files

---

## Architecture Rules

* Prefer composition over inheritance (except where language semantics require it)
* Keep layers isolated:

  * lexer MUST NOT depend on checker
  * checker MUST NOT depend on VM
* Data flows strictly forward:
  lexer → parser → checker → compiler → VM

---

## Anti-Patterns (forbidden)

* God objects
* God files (>400 lines)
* Hidden state mutations
* Implicit behavior
* Magic strings
* Hardcoded branching logic for types or features

---

## Refactoring Triggers

Refactor immediately if:

* file > 400 lines
* function too large to reason about
* duplicated logic appears
* feature crosses module boundaries incorrectly

---

## Preferred Output Patterns

Instead of:

* full file dumps

Prefer:

* minimal diff
* isolated function
* new module file (kebab-case if needed)

---

## Error Handling Philosophy

* Errors must be explicit
* Prefer compile-time errors over runtime errors
* Error messages must guide the fix

---

## Performance Guidelines

* Avoid unnecessary allocations
* Prefer linear or logarithmic complexity
* Avoid repeated passes when a single-pass solution is possible

---

## Communication Style

* Direct
* Technical
* No filler
* No repetition

---

## Default Assumptions

If not specified:

* optimize for correctness over performance
* prefer explicit behavior
* reject ambiguous constructs

---

## Special Rules

* Do not introduce new abstractions unless justified
* Do not generalize prematurely
* Do not modify unrelated modules
* Keep fixes minimal and localized


## Compatibility Policy

- Backward compatibility is NOT a goal
- Breaking changes are allowed and encouraged if they improve:
  - soundness
  - consistency
  - simplicity

- Do NOT preserve legacy behavior
- Do NOT introduce compatibility layers
- Do NOT add fallback logic

- If a feature is flawed:
  → replace it
  → remove it
  → break it

## Technical Debt Policy

- Technical debt must not be preserved
- No temporary hacks
- No “we will fix later”
- Fix root causes only

## Design Priority

1. Correctness
2. Soundness
3. Simplicity
4. Performance
5. Compatibility (last, optional)
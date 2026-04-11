# TSN Module System — Plan Definitivo

Inspirado en Deno. Una sola fuente de la verdad. Sin deuda técnica.

---

## Problema actual

El sistema de módulos está fragmentado:

- Paths duplicados en `tsn-checker`, `tsn-vm`, `tsn-lsp`, `tsn-cli`
- Stdlib descubierta de forma independiente en cada crate
- Sin registro canónico — agregar un módulo requiere tocar N archivos
- Carga eager de toda la stdlib al arrancar el VM (allocaciones innecesarias)
- `stdlib_dir()` duplicada — función de deuda clásica

---

## Principios del diseño

1. **Un solo registro**: agregar un módulo = 1 entrada + 1 archivo TSN + 1 builder runtime
2. **Builtins**: inyectados en el scope global de cada archivo, cargados una sola vez
3. **Stdlib**: carga lazy — construida en el primer `OpImport`, nunca al arrancar el VM
4. **`tsn-modules`**: autoridad de resolución de paths para checker, VM y LSP
5. **Sin dependencias circulares**: `tsn-core → tsn-modules → tsn-checker → tsn-vm`
6. **Sin wrappers**: `stdlib_dir()` no existe en ninguna forma

---

## Nueva crate: `tsn-modules`

Ubicación: `crates/tsn-modules/`
Dependencias: **solo `tsn-core`**. Cero deps en checker, vm, parser o compiler.

### `src/spec.rs`

```rust
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ModuleKind {
    /// Inyectado en el scope global. No requiere import.
    Builtin,
    /// Cargado lazy en el primer `import { X } from "std:foo"`.
    Stdlib,
}

pub struct ModuleSpec {
    /// Identificador canónico: "builtin:global", "std:math"
    pub id: &'static str,
    pub kind: ModuleKind,
    /// Path relativo al stdlib root: "std/math/mod.tsn"
    pub tsn_source: &'static str,
}
```

### `src/registry.rs`

Única fuente de la verdad. Cada módulo conocido tiene exactamente una entrada.

```rust
pub static MODULE_REGISTRY: &[ModuleSpec] = &[
    // ── Builtins ─────────────────────────────────────────────────────────
    ModuleSpec { id: "builtin:global",     kind: Builtin, tsn_source: "builtins/global.tsn" },
    ModuleSpec { id: "builtin:primitives", kind: Builtin, tsn_source: "builtins/primitives.tsn" },
    ModuleSpec { id: "builtin:classes",    kind: Builtin, tsn_source: "builtins/classes.tsn" },

    // ── Stdlib ───────────────────────────────────────────────────────────
    ModuleSpec { id: "std:async",       kind: Stdlib, tsn_source: "std/async/mod.tsn" },
    ModuleSpec { id: "std:collections", kind: Stdlib, tsn_source: "std/collections/mod.tsn" },
    ModuleSpec { id: "std:console",     kind: Stdlib, tsn_source: "std/console/mod.tsn" },
    ModuleSpec { id: "std:crypto",      kind: Stdlib, tsn_source: "std/crypto/mod.tsn" },
    ModuleSpec { id: "std:dispose",     kind: Stdlib, tsn_source: "std/dispose/mod.tsn" },
    ModuleSpec { id: "std:fs",          kind: Stdlib, tsn_source: "std/fs/mod.tsn" },
    ModuleSpec { id: "std:http",        kind: Stdlib, tsn_source: "std/http/mod.tsn" },
    ModuleSpec { id: "std:io",          kind: Stdlib, tsn_source: "std/io/mod.tsn" },
    ModuleSpec { id: "std:json",        kind: Stdlib, tsn_source: "std/json/mod.tsn" },
    ModuleSpec { id: "std:math",        kind: Stdlib, tsn_source: "std/math/mod.tsn" },
    ModuleSpec { id: "std:net",         kind: Stdlib, tsn_source: "std/net/mod.tsn" },
    ModuleSpec { id: "std:path",        kind: Stdlib, tsn_source: "std/path/mod.tsn" },
    ModuleSpec { id: "std:reflect",     kind: Stdlib, tsn_source: "std/reflect/mod.tsn" },
    ModuleSpec { id: "std:result",      kind: Stdlib, tsn_source: "std/result/mod.tsn" },
    ModuleSpec { id: "std:sys",         kind: Stdlib, tsn_source: "std/sys/mod.tsn" },
    ModuleSpec { id: "std:temporal",    kind: Stdlib, tsn_source: "std/temporal/mod.tsn" },
    ModuleSpec { id: "std:test",        kind: Stdlib, tsn_source: "std/test/mod.tsn" },
    ModuleSpec { id: "std:time",        kind: Stdlib, tsn_source: "std/time/mod.tsn" },
    ModuleSpec { id: "std:types",       kind: Stdlib, tsn_source: "std/types/mod.tsn" },
];

pub fn spec_for(id: &str) -> Option<&'static ModuleSpec> {
    MODULE_REGISTRY.iter().find(|m| m.id == id)
}

pub fn is_known(specifier: &str) -> bool {
    MODULE_REGISTRY.iter().any(|m| m.id == specifier)
}
```

### `src/loader.rs`

Autoridad de resolución de paths. No maneja `ExportMap` ni `BindResult` — eso evita deps circulares.

```rust
pub struct ModuleLoader {
    stdlib_root: PathBuf,
}

impl ModuleLoader {
    pub fn new(stdlib_root: PathBuf) -> Self
    pub fn from_env() -> Self  // Ver nota sobre Windows abajo
    pub fn stdlib_root(&self) -> &Path
    pub fn is_known(&self, specifier: &str) -> bool
    pub fn spec_for(&self, specifier: &str) -> Option<&'static ModuleSpec>
    pub fn is_builtin(&self, specifier: &str) -> bool
    pub fn is_stdlib(&self, specifier: &str) -> bool
    pub fn builtins(&self) -> impl Iterator<Item = &'static ModuleSpec>
    pub fn stdlib_modules(&self) -> impl Iterator<Item = &'static ModuleSpec>
    pub fn tsn_source_path(&self, specifier: &str) -> Option<PathBuf>
    pub fn resolve_relative(&self, base_dir: &Path, specifier: &str) -> Option<String>
}
```

**`from_env()` — orden de resolución del stdlib root:**
1. `TSN_STDLIB` env var
2. `TSN_HOME/stdlib`
3. `{exe_parent}/../tsn-stdlib` (layout instalado)
4. `{cwd}/tsn-stdlib` (layout de desarrollo)

**CRÍTICO**: nunca llamar `canonicalize()` sobre el stdlib root.
`canonicalize()` en Windows retorna paths `\\?\C:\...` (extended-length).
`PathBuf::join("std/math/mod.tsn")` falla silenciosamente con esos paths.

### `src/lib.rs`

```rust
mod loader;
mod registry;
mod spec;

pub use loader::ModuleLoader;
pub use registry::{is_known, spec_for, MODULE_REGISTRY};
pub use spec::{ModuleKind, ModuleSpec};
```

---

## Ejemplo: registrar un módulo stdlib

**1. Entrada en el registry** (`tsn-modules/src/registry.rs`):
```rust
ModuleSpec { id: "std:math", kind: Stdlib, tsn_source: "std/math/mod.tsn" },
```

**2. Archivo TSN** (`tsn-stdlib/std/math/mod.tsn`):
```tsn
export declare namespace Math {
    const PI: float
    function floor(x: float): float
    // ...
}
```

**3. Builder runtime** (`tsn-vm/src/intrinsic/math.rs` o equivalente):
```rust
pub(crate) fn build() -> Value {
    // construye el Value::Object con las funciones nativas
}
```

**4. Entrada en el dispatch** (`tsn-vm` — `build_module_by_id`):
```rust
"std:math" => Some(math::build()),
```

Eso es todo. Sin tocar checker, lsp, cli.

---

## Ejemplo: registrar un módulo builtin

**1. Entrada en el registry**:
```rust
ModuleSpec { id: "builtin:global", kind: Builtin, tsn_source: "builtins/global.tsn" },
```

**2. Archivo TSN** (`tsn-stdlib/builtins/global.tsn`):
Declara los símbolos globales disponibles en todo archivo sin import.

**3. Carga**: el VM los inyecta en el scope global al arrancar, via `build_module_by_id("builtin:global")`.
No pasan por `OpImport`.

---

## Nueva crate: `tsn-runtime`

Ubicación: `crates/tsn-runtime/`
Propósito: **implementaciones nativas de todos los módulos stdlib**. Mantiene el VM limpio.

El VM (`tsn-vm`) solo contiene el ejecutor puro: stack, frames, opcodes, scheduler.
Todo lo que es lógica de módulo concreto vive aquí.

### Qué se mueve de `tsn-vm` a `tsn-runtime`

```
crates/tsn-vm/src/intrinsic/
    array.rs        → crates/tsn-runtime/src/modules/array.rs
    async_.rs       → crates/tsn-runtime/src/modules/async_.rs
    console.rs      → crates/tsn-runtime/src/modules/console.rs
    crypto.rs       → crates/tsn-runtime/src/modules/crypto.rs
    fs.rs           → crates/tsn-runtime/src/modules/fs.rs
    http.rs         → crates/tsn-runtime/src/modules/http.rs
    io.rs           → crates/tsn-runtime/src/modules/io.rs
    json.rs         → crates/tsn-runtime/src/modules/json.rs
    map.rs          → crates/tsn-runtime/src/modules/map.rs
    math.rs         → crates/tsn-runtime/src/modules/math.rs
    net.rs          → crates/tsn-runtime/src/modules/net.rs
    path.rs         → crates/tsn-runtime/src/modules/path.rs
    primitives.rs   → crates/tsn-runtime/src/modules/primitives.rs
    reflect.rs      → crates/tsn-runtime/src/modules/reflect.rs
    set.rs          → crates/tsn-runtime/src/modules/set.rs
    sys.rs          → crates/tsn-runtime/src/modules/sys.rs
    testing.rs      → crates/tsn-runtime/src/modules/testing.rs
    time.rs         → crates/tsn-runtime/src/modules/time.rs
    table.rs        → crates/tsn-runtime/src/dispatch.rs
```

### API pública de `tsn-runtime`

```rust
// dispatch.rs — tabla de intrinsics indexada por ID numérico
pub fn dispatch_intrinsic(id: u16, vm: &mut Vm, args: &[Value]) -> Result<Value, String>

// registry.rs — builders lazy por specifier
pub fn build_module_by_id(id: &str) -> Option<Value>
```

`build_module_by_id` es el punto de entrada para el VM cuando encuentra `OpImport "std:math"`:
```rust
pub fn build_module_by_id(id: &str) -> Option<Value> {
    match id {
        "std:math"    => Some(modules::math::build()),
        "std:console" => Some(modules::console::build()),
        "std:fs"      => Some(modules::fs::build()),
        "std:http"    => Some(modules::http::build()),
        // ...
        _ => None,
    }
}
```

### `crates/tsn-runtime/Cargo.toml`

```toml
[package]
name = "tsn-runtime"
version = "0.1.0"
edition = "2021"

[dependencies]
tsn-core     = { path = "../tsn-core" }
tsn-types    = { path = "../tsn-types" }
tsn-vm       = { path = "../tsn-vm" }
tsn-modules  = { path = "../tsn-modules" }
# deps de módulos nativos:
parking_lot  = "0.12"
rust_decimal = { version = "1", features = ["serde"] }
mio          = { version = "1", features = ["os-poll", "net"] }
serde_json   = "1.0"
rand         = "0.8"
sha2         = "0.10"
hmac         = "0.12"
uuid         = { version = "1", features = ["v4"] }
base64       = "0.21"
hex          = "0.4"
url          = "2"
urlencoding  = "2"
ureq         = "2"
tiny_http    = "0.12"
```

Todas las deps pesadas (ureq, tiny_http, sha2, etc.) salen de `tsn-vm` y quedan aquí.

### `tsn-vm` después del split

`tsn-vm/Cargo.toml` queda solo con:
```toml
[dependencies]
tsn-core     = { path = "../tsn-core" }
tsn-types    = { path = "../tsn-types" }
tsn-compiler = { path = "../tsn-compiler" }
tsn-modules  = { path = "../tsn-modules" }
parking_lot  = "0.12"
rust_decimal = { version = "1" }
mio          = { version = "1", features = ["os-poll", "net"] }
```

Sin `ureq`, `tiny_http`, `sha2`, `rand`, `uuid`, etc. El VM no sabe nada de HTTP, crypto o fs.

---

## Cambios por crate

### `Cargo.toml` (workspace root)
```toml
members = [
    # ... existentes ...
    "crates/tsn-modules",
]
```

### `crates/tsn-modules/Cargo.toml`
```toml
[package]
name = "tsn-modules"
version = "0.1.0"
edition = "2021"

[dependencies]
tsn-core = { path = "../tsn-core" }
```

### `tsn-checker`
- Add dep: `tsn-modules = { path = "../tsn-modules" }`
- `module_resolver.rs`:
  - **Eliminar** `stdlib_dir()` — no debe existir en ninguna forma
  - `stdlib_path_for(specifier)` → `tsn_modules::ModuleLoader::from_env().tsn_source_path(specifier)`
  - `is_known_stdlib(specifier)` → `tsn_modules::is_known(specifier)`

### `tsn-vm`
- Add dep: `tsn-modules = { path = "../tsn-modules" }`
- `vm/exec/modules.rs` — `resolve_import_path`:
  - Eliminar las ramas hardcodeadas `"std:"` y `"builtin:"`
  - Reemplazar con `tsn_modules::ModuleLoader::from_env().tsn_source_path(specifier)`
- `vm/mod.rs`:
  - `Vm::new()`: `modules` empieza como `HashMap::new()` — sin pre-carga de stdlib
  - Eliminar cualquier llamada a `register_native_std_modules` o equivalente
- `builtins/std_modules.rs`:
  - Reemplazar `register_std_modules(modules)` con:
    ```rust
    pub(crate) fn build_module_by_id(id: &str) -> Option<Value> {
        match id {
            "std:math"    => Some(math::build()),
            "std:console" => Some(console::build()),
            // ...
            _ => None,
        }
    }
    ```
- `vm/exec/modules.rs` — `OpImport`:
  - Antes de resolver el path, consultar `tsn_modules::is_known(specifier)` → si es conocido, llamar `build_module_by_id(specifier)` directamente (sin pasar por el filesystem para módulos nativos)

### `tsn-lsp`
- Add dep: `tsn-modules = { path = "../tsn-modules" }`
- `features/completion/imports.rs` — `stdlib_module_completions`:
  - Reemplazar el filesystem scan con iteración sobre `MODULE_REGISTRY`:
    ```rust
    tsn_modules::MODULE_REGISTRY
        .iter()
        .filter(|m| m.kind == ModuleKind::Stdlib && m.id.starts_with(prefix))
        .map(|m| CompletionItem { label: m.id.to_owned(), ... })
    ```
- `index/builder.rs` — `resolve_specifier_to_uri`:
  - Reemplazar `stdlib_dir()` con `tsn_modules::ModuleLoader::from_env().tsn_source_path(specifier)`

### `tsn-cli`
- Add dep: `tsn-modules = { path = "../tsn-modules" }`
- `doctor.rs`:
  - Reemplazar `tsn_checker::module_resolver::stdlib_dir()` con:
    ```rust
    let loader = tsn_modules::ModuleLoader::from_env();
    let stdlib = loader.stdlib_root();
    ```

---

## Flujo de carga por tipo

### Builtin (`builtin:global`, etc.)

```
VM::new()
  └─ para cada spec con kind == Builtin en MODULE_REGISTRY:
       build_module_by_id(id) → Value
       inyectar en globals del VM
```

### Stdlib (`std:math`, etc.)

```
OpImport "std:math"
  ├─ ¿ya en modules cache? → push cached
  └─ tsn_modules::is_known("std:math") == true
       └─ build_module_by_id("std:math") → Value   (lazy, primera vez)
            store en modules cache
            push
```

### Módulo relativo (`./foo`, `../bar`)

```
OpImport "./foo"
  └─ resolve_import_path → abs_path via ModuleLoader::resolve_relative
       └─ load_module_file(abs_path)   (parse + check + compile + run)
```

---

## Invariantes

- `tsn-modules` tiene cero deps en checker/vm/parser/compiler
- `stdlib_dir()` no existe en ningún crate — eliminada sin wrapper
- `MODULE_REGISTRY` es la única fuente de la verdad para especificadores conocidos
- Los módulos stdlib no se construyen al arrancar el VM — solo en el primer import
- Agregar un módulo nuevo requiere exactamente: 1 línea en registry + 1 archivo TSN + 1 arm en `build_module_by_id`
- `canonicalize()` no se llama sobre el stdlib root

---

## Dependencia entre crates (final)

```
tsn-core
  └─ tsn-types
  └─ tsn-modules
       ├─ tsn-checker
       │    └─ tsn-compiler
       │         └─ tsn-vm  (ejecutor puro, sin lógica de módulos)
       │              └─ tsn-runtime  (impls nativas + dispatch + build_module_by_id)
       │                   └─ tsn-cli
       └─ tsn-lsp
            └─ tsn-cli
```

`tsn-vm` no depende de `tsn-runtime`. `tsn-runtime` depende de `tsn-vm` para el tipo `Vm`
que usan los HostOps. `tsn-cli` ensambla todo.

# Auditoría General del Lenguaje TSN — Perspectiva Anders Hejlsberg

> **Fecha**: Abril 2026  
> **Versión evaluada**: v0.9 (Pre-release)  
> **Alcance**: Diseño del lenguaje, arquitectura del compilador, VM, sistema de tipos, biblioteca estándar, calidad del código, decisiones de ingeniería  
> **Criterio**: Rigor de un diseñador de lenguajes de producción (nivel C#/TypeScript/Delphi)

---

## Resumen Ejecutivo

TSN es un lenguaje compilado, estaticamente tipado, con sintaxis inspirada en TypeScript que compila a bytecode nativo sobre una VM de pila custom. El proyecto consta de **~42,500 líneas de Rust** distribuidas en **12 crates del workspace**, más la biblioteca estándar en TSN. La arquitectura general es **sólida y bien pensada**: pipeline de compilación con separación de fases correcta, VM con inline caching y vtable dispatch, scheduler cooperativo para async/await, y un sistema de tipos con generics multi-fase, narrowing sensitivo al flujo de control, y pattern matching con verificación de exhaustividad.

Sin embargo, **existen deficiidades críticas** que separan este proyecto de un lenguaje de producción: seguridad de memoria comprometida (`unsafe impl Send/Sync for Value` con raw pointers), ausencia casi total de tests unitarios (~8 tests en un proyecto de 42K líneas), mensajes de error sin códigos, límites artificiales de bytecode (`u16` = 64K), y un módulo `dynamic` en APIs centrales que socava el sistema de tipos desde el día uno.

**Veredicto general**: Fundamento arquitectónico de nivel profesional con problemas de ejecución y madurez propios de un proyecto de 2 meses. Las decisiones de diseño del lenguaje son en su mayoría acertadas; los problemas están en la ingeniería de producción.

---

## Tabla de Contenidos

1. [Diseño del Lenguaje](#1-diseño-del-lenguaje)
2. [Sistema de Tipos](#2-sistema-de-tipos)
3. [Modelo de Objetos](#3-modelo-de-objetos)
4. [Async/Await](#4-asyncawait)
5. [Pattern Matching](#5-pattern-matching)
6. [Sistema de Módulos](#6-sistema-de-módulos)
7. [Biblioteca Estándar](#7-biblioteca-estándar)
8. [Arquitectura del Compilador](#8-arquitectura-del-compilador)
9. [Arquitectura de la VM](#9-arquitectura-de-la-vm)
10. [Gestión de Memoria](#10-gestión-de-memoria)
11. [Manejo de Errores](#11-manejo-de-errores)
12. [Calidad del Código](#12-calidad-del-código)
13. [Tooling (LSP + CLI)](#13-tooling-lsp--cli)
14. [Documentación](#14-documentación)
15. [Decisiones de Diseño — Aciertos](#15-decisiones-de-diseño--aciertos)
16. [Decisiones de Diseño — Problemas](#16-decisiones-de-diseño--problemas)
17. [Legacy y Deuda Técnica](#17-legacy-y-deuda-técnica)
18. [Hoja de Ruta Crítica](#18-hoja-de-ruta-crítica)
19. [Calificación Final](#19-calificación-final)

---

## 1. Diseño del Lenguaje

### 1.1 Sintaxis TypeScript-like

**Calificación: 8/10**

La decisión de adoptar sintaxis TypeScript es pragmática: reduce la barrera de entrada para millones de desarrolladores que ya conocen `{}`, `const`, `let`, `function`, clases, etc. Sin embargo, esta decisión tiene un costo oculto: **la familiaridad engaña**. Un programador de TypeScript asumirá que `==` funciona como en JS, que `var` existe, que `delete` elimina propiedades. TSN elimina varios de estos (`==` planeado para eliminación, `var` deprecado), lo cual es correcto, pero necesita comunicación clara al usuario sobre qué se eliminó y por qué.

**Aciertos:**
- `const` (inmutable) y `let` (mutable) como únicos mecanismos de declaración — sin `var`
- `===` como único operador de igualdad (elimina la coerción implícita de JS)
- Pipeline operator `|>` con placeholder `_` (estilo Hack) — semántica unificada y predecible
- `using` para gestión de recursos — superior a `try/finally` manual
- Named arguments en sitio de llamada — feature que TypeScript no tiene y que TSN incorpora correctamente

**Problemas:**
- `match (n)` con paréntesis es innecesario — Rust y Kotlin no los usan. El paréntesis crea ruido visual para una expresión
- Dos formas de declarar tipos: `type` (alias estructural) y `newtype` (nominal). Correcto en teoría, pero la ergonomía de `newtype` es pesada: `UserId("abc")` para envolver, `.value` o `as str` para desenvolver. Sin derive macros (como `#[derive(Display, From)]` en Rust), la ceremonia se acumula rápido
- `record` aparece en ejemplos pero está "in progress" en el roadmap. Si `Record<K,V>` es un tipo mapa, choca conceptualmente con `record` como tipo valor inmutable. Necesitan nombres distintos
- Falta el operador `?` para propagación de `Result<T,E>` — crítico para ergonomía

### 1.2 Principios de Diseño Declarados

**Calificación: 7/10**

Los seis principios declarados son bien articulados pero no todos se cumplen consistentemente:

| Principio | Cumplimiento | Comentario |
|-----------|-------------|------------|
| "Una semántica, varios contextos" | ⚠️ Parcial | `dynamic` lo viola (significa "verificar en runtime" — semántica fundamentalmente distinta) |
| "Explicit over implicit" | ✅ Bien | `await` explícito, `using` explícito, `override` obligatorio |
| "Types are verifiable documentation" | ⚠️ Parcial | `Result<T,E>` usa `dynamic` internamente — violación directa |
| "The compiler tells you what to do" | ⚠️ Regular | Mensajes de error inconsistentes, sin códigos, sin sugerencias |
| "Soundness is a goal, not an excuse" | ✅ Pragmático | Mejor que la insoundez deliberada de TypeScript |
| "No silent failures" | ⚠️ Parcial | El bug de byte offset vs line number en extension methods fue exactamente un "silent failure" |

---

## 2. Sistema de Tipos

### 2.1 Diseño General

**Calificación: 8.5/10**

El sistema de tipos es **la mejor decisión arquitectónica de TSN**. La normalización de nullable (`T?` → `Union([T, Null])` en el binder) elimina una clase entera de bugs del checker. TypeScript carga `Nullable` y `Undefined` por todo su pipeline, creando casos especiales infinitos. TSN colapsa esto en una sola representación canónica — diseño superior.

**Aciertos:**
- Modelo híbrido nominal/estructural: objetos son estructurales, clases son nominales, `newtype` es opaco. Esto es correcto — las abstracciones nominales (clases) deben llevar identidad, las formas anónimas deben ser intercambiables
- `newtype` como constructo de primera clase elimina el problema "stringly-typed" (`UserId` vs `OrderId`). Rust usa este patrón; TSN lo hace ergonométrico
- `never` como bottom type con reglas de subtyping correctas (`never <: T` para todo T)
- Type predicates (`x is T`) con narrowing bidireccional (estrecha en true, resta en false)
- Narrowing literal con widening explícito: `let y: int = 1` amplia, `const x = 1` mantiene estrecho
- Subtyping de funciones: parámetros contravariantes, retorno covariante — correcto

**Problemas Críticos:**

**A. `dynamic` en APIs centrales (GRAVE — 2/10)**

El builtin `print` tiene firma `print(...args: dynamic[])`. Esto es el problema `any` de TypeScript all over again. Si TSN busca soundez, `dynamic` debe restringirse severamente — idealmente solo en fronteras FFI. El hecho de que la función más fundamental del lenguaje use tipos no verificados significa que **todo programa TSN encuentra `dynamic` inmediatamente**. Esto socava el sistema de tipos desde el día uno.

**Solución propuesta:** `print<T>(...args: T[])` con formatting basado en `Display` implícito, o usar `unknown[]` con narrowing automático vía `typeof`.

**B. `unknown` sin toolkit de narrowing (MODERADO — 5/10)**

`unknown` como top type es correcto en teoría, pero sin `is` type guards a nivel de expresión (solo existen como retorno de función), `unknown` se vuelve frustrante. Se necesita: type guards como expresión, conditional binding (`if let`), y narrowing automático en bloques.

**C. Tipos condicionales, mapeados, y template literals (BAJO — N/A)**

Listados como "in progress". Son las features más complejas de TypeScript y fuente de dolor inmenso de implementación. Los tipos condicionales con distribución sobre type parameters desnudos crean un sistema de meta-programación que hasta los contribuidores de TypeScript encuentran difícil. **Diferirlos es la decisión correcta** — pero el roadmap debería evaluar si valen el costo para un lenguaje de sistemas.

**D. Sin reglas de coerción numérica definidas (GRAVE — 3/10)**

El roadmap lista "numeric type hierarchy" como objetivo v1.0 pero "define clear coercion rules" está pendiente. ¿Puedes pasar `int` donde se espera `float`? ¿`float` a `decimal`? Las conversiones implícitas numéricas son fuente masiva de bugs (JavaScript `==`, C implicit promotions). La ausencia de reglas definidas en v0.9 es preocupante. Rust exige `as` explícito; C# tiene reglas estrictas. TSN necesita definir esto **antes de v1.0**.

**E. `T?` vs `T | null` — doble representación (MENOR — 6/10)**

Son equivalentes (azúcar sintáctico), pero cuando el compilador imprime `Union([int, Null])` y el usuario escribió `int?`, la conexión se pierde en mensajes de error. TypeScript prefiere `T | null` en diagnósticos. TSN debería imprimir `T?` cuando el usuario usó esa forma, o documentar claramente la equivalencia.

### 2.2 Generic Resolution

**Calificación: 8/10**

La resolución de generics en 5 fases (argumentos explícitos → inferencia → acceso a miembros → validación de constraints → herencia) es sólida. El sistema de variables de tipo `Named` con binding durante matching de argumentos es correcto.

**Problemas:**
- Sin Higher-Kinded Types (HKTs) — limitación aceptable para v0.9 pero relevante para `Monad`, `Functor` en stdlib
- Sin type-level computation (conditional types en progreso) — bien diferido

---

## 3. Modelo de Objetos

### 3.1 Clases, Interfaces, Herencia

**Calificación: 8/10**

**Aciertos:**
- Vtable dispatch es la elección correcta para un lenguaje compilado. El compiler bakea índices de vtable, la VM hace lookup O(1). Esto es territorio C++/Java — muy superior al hash-table property lookup de JS
- `override` obligatorio para métodos heredados — previene el bug clásico donde la firma cambia en la base y deja de sobrescribir en la subclase. C# lo hace bien; TSN sigue
- `this` return type para fluent builders — elegante. El tipo concreto del receptor se sustituye, así `ExtendedBuilder.setName()` devuelve `ExtendedBuilder`, no `Builder`. Como Ceylon y Kotlin
- Extension methods en cualquier tipo (incluyendo primitivos `str` e `int`) — implementación vía mangled name lookup en compile time = zero runtime overhead. Como C# y Rust
- Getter/setter como miembros de primera clase con sintaxis natural `get prop`, `set prop`

**Problemas:**

**A. Herencia simple sin traits/mixins (MODERADO — 5/10)**

El roadmap no menciona traits/mixins para v1.0. Esto es una limitación significativa de composición behavioral múltiple. El modelo de traits de Rust (sin herencia, con impl blocks) sería mejor que OOP tradicional. Sin traits, no hay mechanismo para "múltiples comportamientos sin múltiples herencias".

**B. `implements` puramente declarativo (MENOR — 6/10)**

Si `class Config implements Printable` es solo documentación (el checker atrapa el método faltante de todos modos), la keyword no agrega valor. C# lo enforcea en la declaración; TypeScript es una pista compile-time. TSN debe decidir cuál es — si es solo documentación, eliminar la keyword. Si es enforceamiento, verificar compatibilidad explícitamente.

**C. Sin delegación de constructores (MENOR — 6/10)**

Los ejemplos muestran `super(n)` pero no `this(...)` para constructor chaining. Esto lleva a lógica de inicialización duplicada.

---

## 4. Async/Await

### 4.1 Modelo de Ejecución

**Calificación: 7/10**

**Aciertos:**
- Wrapping implícito de `Future<T>` en tipo de retorno de async functions — elección ergonómica correcta. Kotlin hace lo mismo con `suspend`
- `await` con type stripping (`await Future<T>` produce `T`) con warning si se espera algo no-Future — buena ergonomía de error

**Problemas Críticos:**

**A. Child VM por tarea async es costoso (GRAVE — 4/10)**

Cada tarea async aloca su propia pila de bytecode, call frames, constant pool, y scope chain. En un sistema con miles de tareas concurrentes (servidor HTTP manejando conexiones), este overhead de memoria se acumula. Node.js usa un solo event loop con coroutine frames livianas; Tokio usa stackless futures. La arquitectura de TSN es más cercana a procesos Erlang que a async/await convencional — puede ser intencional, pero necesita ser consciente.

**Impacto estimado:** Cada VM child ≈ 2-4KB de stack pre-alocado + call frames + constantes. 10,000 conexiones HTTP concurrentes = 20-40MB solo en estructuras de VM, sin contar el heap.

**B. Reactor thread + mio poll (MODERADO — 6/10)**

El modelo async es fundamentalmente distinto para operaciones nativas (Rust FFI) vs código TSN. Operaciones nativas se registran con mio y wakean vía reactor. Operaciones TSN suspenden/resumen vía scheduler. La frontera entre estos dos mundos necesita diseño cuidadoso para evitar deadlocks y race conditions.

**C. Sin frontera `Send`/`Sync` (GRAVE — 3/10)**

En Rust, el type system enforce thread-safety. Las tareas async de TSN corren en un scheduler single-threaded (bien), pero el reactor thread está separado. Estado mutable compartido entre scheduler y reactor es una fuente potencial de data races. El `unsafe impl Send/Sync for Value` con raw pointers hace esto UB-adjacent (ver sección 10).

---

## 5. Pattern Matching

### 5.1 Implementación Actual

**Calificación: 7/10**

**Aciertos:**
- Exhaustiveness checking sobre enums, uniones, y bools — esencial para un lenguaje seguro
- Tipos primitivos (`str`, `int`, `float`) no pueden ser matched exhaustivamente (infinitos habitantes), requieren `_` — regla correcta
- OR-patterns (`2 | 3 => "two or three"`) reducen duplicación
- Match como expresión (tipo de retorno es unión de tipos de arms) — correcto, como Kotlin `when` y Rust `match`

**Problemas:**

**A. Sin patrones de destructuring (MODERADO — 4/10)**

Los ejemplos solo muestran matching de literales y variantes de enum. Un sistema completo necesita:
- Patrones constructor/destructor: `Point(x, y) => ...`
- Patrones anidados: `Some(Ok(value)) => ...`
- Guard clauses: `x if x > 0 => ...` (la spec menciona `if guard` pero sin ejemplos)
- As-patterns: `x @ Some(_) => ...` (bind del entero mientras se hace match)

**B. Sin discriminated union auto-narrowing (MODERADO — 4/10)**

Listado en "Phase 4". Si `type Shape = Circle | Rectangle` y `Circle` tiene `{ radius: float }` mientras `Rectangle` tiene `{ w: float, h: float }`, los arms del match deberían poder deconstruir: `Circle(r) => 3.14 * r * r`. Esto es una de las features más valiosas de pattern matching.

---

## 6. Sistema de Módulos

### 6.1 Architecture

**Calificación: 9/10**

El registro unificado de módulos (`MODULE_REGISTRY` como single source of truth) es arquitectura limpia. Tanto el checker como la VM resuelven a través del mismo mecanismo, previniendo divergencia checker/runtime.

**Aciertos:**
- Distinción Builtin vs stdlib bien diseñada. Builtins (`print`, `assert`) injectados en scope global — sin imports. Stdlib (`std:foo`) cargados lazy en primer import. Como Python builtins vs imports
- `ModuleSpec` con checker source (`.tsn`) y runtime source (TSN o Rust native) — descriptor limpio
- `ModuleLoader` con env-based stdlib root discovery, caching, lazy loading — correcto

**Problemas:**

**A. Sin package manager o versioning (MODERADO — 4/10)**

El roadmap lo lista para "ecosystem" pero sin dependency resolution, el sistema de módulos solo sirve para stdlib y código local. Todo lenguaje real eventualmente necesita un package registry.

**B. Prefijo `std:` es ad-hoc (MENOR — 6/10)**

Funciona, pero no es un mecanismo general de resolución de módulos. ¿Qué hay de paquetes third-party? `@npm:lodash`? `github:user/repo`? La spec no aborda esto.

**C. `std:types` registrado sin runtime builder (MODERADO — 5/10)**

`std:types` está en `MODULE_REGISTRY` pero NO aparece en `STD_MODULE_BUILDERS`. Tiene archivo de contrato TSN pero sin implementación runtime. Esto causará un error en runtime si alguien importa `std:types`.

**D. Módulos no registrados (MENOR — 6/10)**

- `std:temporal` — implementación completa (Instant, PlainDate, PlainTime, Duration, ZonedDateTime) pero no en registry. El roadmap lo marca "[ ]" (in progress) pero el código existe
- `std:dummy` — módulo de test, intencionalmente no registrado

---

## 7. Biblioteca Estándar

### 7.1 Organización

**Calificación: 7.5/10**

**Aciertos:**
- 16-17 módulos es el tamaño correcto — ni muy pocos (no kitchen sink), ni muchos (no fragmentación)
- `Result<T, E>` con `isOk`, `isErr`, `unwrap`, `map`, `andThen`, `orElse` — convención Rust/Swift
- Convención `mod.tsn` (un entry point por módulo) — limpio y predecible

**Problemas:**

**A. `dynamic` en firmas de builtins (GRAVE — 3/10)**

Ya discutido en sección 2.

**B. Sin tipo `Option<T>` (MODERADO — 5/10)**

El lenguaje tiene nullable types y `Result<T,E>` pero no `Option<T>` (aka `Maybe`). La sintaxis `T?` cubre nulabilidad, pero un tipo `Option` apropiado con `map`, `andThen`, etc. (como `Result`) sería más composable. Actualmente se tiene null-coalescing (`??`) y optional chaining (`?.`) pero no el toolkit de combinadores funcionales.

**C. `Result` usa `dynamic` internamente (GRAVE — 3/10)**

La clase `Result` usa `_value: dynamic` internamente. Esto es consecuencia de la falta de soporte proper para union/discriminated types — la clase tiene que erasar a `dynamic` y castear con `as`. Una vez que TSN tenga proper sum types, esto debería ser `enum Result<T, E> { Ok(T), Err(E) }`.

---

## 8. Arquitectura del Compilador

### 8.1 Pipeline General

**Calificación: 8.5/10**

La separación lex → parse → bind → check → emit es **correcta y profesional**. Cada fase tiene inputs y outputs bien definidos. Los tipos `Token` y `AST` son los contratos entre fases.

### 8.2 Lexer (`tsn-lexer`)

**Calificación: 8/10**

**Aciertos:**
- Hand-written single-pass byte scanner en raw `&[u8]` — correcto para performance
- Module split limpio (`core`, `literals`, `operators`, `templates`, `comments`)
- Dos outputs paralelos: `Vec<TokenRecord>` (struct-of-arrays, 9 u32 por token) + `Vec<u8>` lexeme pool — cache-friendly
- Regex disambiguation vía `can_start_regex()` / `last_kind` — maneja `/` operador vs regex literal correctamente
- Template literal tracking con stacks `template_depth` / `brace_depth`
- Maximal munch para operadores

**Problemas:**
- `TokenRecord` usa `const u32` en lugar de `#[repr(u32)] enum TokenKind` — sacrifica type safety y `match` exhaustiveness por ganancia de memoria negligible
- Sin error recovery: caracteres desconocidos se vuelven tokens `UNKNOWN` silenciosamente
- `scan_number` tiene duplicación significativa en ramas binary/octal/hex

### 8.3 Parser (`tsn-parser`)

**Calificación: 8.5/10**

**Aciertos:**
- Recursive descent con Pratt parsing — la elección textbook correcta para lenguajes tipo C
- 16 niveles de precedencia Pratt con right-associativity para `**` — correcto
- Speculative arrow parsing con save/restore — maneja ambigüedad `(x) => x` vs `(x)` limpiamente
- Generic call disambiguation con peek de 32 tokens tracking `<`/`>` nesting
- ASI guard para break/continue labels
- `ParseProfile` para timing instrumentation

**Problemas:**
- **Mensajes de error pobres**: `"Expected {:?}, got {:?} ({:?}) at {}:{}".` Expone nombres internos de enums (`TokenKind::FatArrow`) al usuario. TypeScript produce `"':' expected."`, no `"Expected FatArrow, got Identifier at 5:3"`
- **Sin error codes**: Cada error es un string sin tipo. El struct `Diagnostic` tiene `code: Option<u32>` pero el parser nunca lo usa
- **`types.rs` (593 líneas)** es el archivo más grande del parser — maneja toda la superficie de parsing de tipos. Debería modularizarse
- **`parse_stmt_or_decl_inner`** dispatcha ~20 variantes en un solo match

### 8.4 Type Checker (`tsn-checker`)

**Calificación: 9/10**

**Aciertos:**
- **Multi-fase** (Bind → Enrich → Full Check) — espeja correctamente el enfoque de TypeScript
- **Caching extensivo**: `infer_cache`, `compat_cache`, `type_node_cache`, `symbol_type_params_cache`, `member_exists_cache`. El `infer_env_rev` dirty counter es clever — invalida caches cuando nueva información de tipos es descubierta
- **`rustc_hash::FxHashMap`** — correcto para performance (sin riesgo de DoS por hash resistance)
- **Flow-sensitive type narrowing** con `narrowed_types` como stack y push/pop — maneja `if (x is T)` correctamente
- **`with_expected` contextual typing** para propagación de tipo contextual — como TypeScript
- **`in_progress` set** para manejar ciclos de tipos recursivos — previene stack overflow
- **Modularización excelente**: la crate mejor modularizada del proyecto

**Problemas:**
- **Pointer-as-hash-key anti-pattern (GRAVE — 3/10)**: `expr as *const Expr as usize` para cache keys. Esto es undefined behavior si el AST se mueve entre bind y check phases. Funciona en práctica porque `Program` vive en el stack del caller, pero no es safe
- **`TypeContext` trait duplicado** idénticamente en `Binder` y `BindResult` (~30 líneas cada uno, copia textual) — duplicación de código clara
- **`BindResult` tiene 17 campos** incluyendo `class_methods`, `class_members`, `interface_members`, etc. — fat result struct que debería splitarse en sub-objetos
- **Sin error recovery en checker**: Un error de tipo en un function body aborta el type checking de toda la función. TypeScript continúa checking después de errores
- **`Diagnostic::error` nunca setea `code`**: Cada error tiene `code: None`. Compiladores de producción usan error codes (TS2322, CS0103)

### 8.5 Compiler (`tsn-compiler`)

**Calificación: 7.5/10**

**Aciertos:**
- **Smart pop optimization**: transforma `SET_GLOBAL; POP` en `DEFINE_GLOBAL`, `PUSH_CONST; POP` en nada — peephole optimization efectivo
- **Finally block inlining**: correctamente inlines pending finally blocks antes de `OpReturn` con `PopTry` por cada handler activo
- **Vtable/field layout tracking**: compiler trackea field slots en compile time y emite `OpSetFixedField` en lugar de `OpSetProperty` para clases conocidas
- **`builtin_class_registry()`** pre-seedea field layouts para `Error`, `TypeError`, `RangeError` — clever y correcto
- **IR optimization passes**: unreachable block removal (BFS), trivial jump simplification (hasta 16 hops con cycle detection)

**Problemas:**
- **`unsafe` raw pointers innecesarios**: `type_annotations: Option<*const TypeAnnotations>`, `extension_calls: Option<*const FxHashMap<u32, String>>`. Raw pointers pasados como `&T as *const T`. Aunque nunca se dereferencian como mutable y la referencia outlive al compiler, esto es **unsafe code sin beneficio** — podría usar `Option<&'a TypeAnnotations>` con lifetime parameter
- **`Compiler` es un struct con 22 campos** (424 líneas en `emit/mod.rs`) — borderline god-file territory
- **Nombres de módulos opacos**: `structural_core.rs` y `structural_advanced.rs` — ¿qué los hace "structural"? Mirando el código, `structural_core` maneja literals, identifiers, arrays, objects; `structural_advanced` maneja match, class expressions, tagged templates. Nombres como `literals.rs`, `collections.rs`, `match_expr.rs` serían muy superiores
- **Dispatch cascading pobre**: `compile_expr_basic_ops` → `compile_expr_member_call` → `compile_expr_structural_core` → `compile_expr_structural_advanced`. Cada función retorna `Result<bool, String>` donde `Ok(true)` significa "handled". Esto debería ser un solo `match` expression
- **Mensajes de error mínimos**: `"unsupported expression variant"`, `"unsupported statement variant"` — deberían nombrar la variante específica e incluir source location
- **Sin IR-level optimization más allá de reachability y jump simplification**: No constant folding, no dead store elimination, no inlining

---

## 9. Arquitectura de la VM

### 9.1 Diseño General

**Calificación: 8.5/10**

La VM es un **stack-based bytecode interpreter** con inline caching, vtable dispatch, y cooperative async scheduler. Las decisiones de arquitectura son patrones bien entendidos de VMs de producción (V8 shape-based IC, JVM exception handling, Tokio async model, Lua stack frames).

**Aciertos:**
- Shape-based hidden classes para property access eficiente
- Vtable dispatch: O(1) method lookup, herencia preserva índices
- Monomorphic inline caching — bueno para el common case
- Cooperative async scheduler — modelo correcto para workloads I/O-bound
- Clean modularization: opcode execution split en 11 submodules focused
- Upvalue system: closure capture correcto con stack-to-heap migration
- Specialized arithmetic opcodes (`OpAddI32`, `OpAddF64`) — evitan runtime type checks cuando los tipos son conocidos

**Problemas:**

**A. Monomorphic-only IC (MODERADO — 5/10)**

Cualquier property access site que vea 2+ shapes will cache-miss en cada llamada. VMs de producción usan PICs de 2-4 vías. Esto es la causa #1 de performance impredecible en VMs de lenguajes dinámicos.

**B. Límites de bytecode `u16` (MODERADO — 5/10)**

- Máx. 65,535 instrucciones por función
- Máx. 65,535 constantes por función
- Máx. offset de jump de 65,535

Estos límites se alcanzan en código real (large switch statements, módulos con muchos imports). VMs de producción usan variable-length encoding.

**C. `Vec<Value>` stack sin small-value optimization (MENOR — 6/10)**

Cada clon de `Value` copia 24+ bytes. Una representación NaN-boxed (como LuaJIT) sería 8 bytes.

**D. `globals: Arc<RwLock<HashMap>>` (MODERADO — 5/10)**

Cada lectura/escritura de variable global adquiere un lock. Debería usar indexed global slots como V8's context.

**E. `Vec::insert` para method dispatch (MENOR — 6/10)**

`self.stack.insert(this_idx, method)` es O(n) stack shift. Debería usar un dedicated register para `this`.

**F. Sin JIT (MODERADO — 4/10)**

Interpretación pura. Para código compute-bound, será 10-100x más lento que código compilado.

### 9.2 Puntos de Falla Potenciales

- `Arc::get_mut(c).unwrap()` en operaciones de clase — panics si el `Arc` tiene múltiples referencias. Asume que los class objects solo se referencian una vez durante la definición de clase. Un check defensivo sería mejor
- `self.frames.last().expect("no active call frame")` — panic en frames vacíos
- `unreachable!()` macros — asumen corrección del compiler. Bytecode malformed podría panic en lugar de retornar error

---

## 10. Gestión de Memoria

### 10.1 Modelo Actual

**Calificación: 3/10 — PROBLEMA CRÍTICO**

El proyecto se describe como "reference-counting GC" pero **NO es reference counting**. Es un **arena allocator** thread-local que libera todo al revés cuando el VM hace drop.

```rust
thread_local! { static HEAP: RefCell<Heap> = ... }
struct Heap { objects: Vec<ObjRef>, arrays: Vec<ArrayRef>, ... }
```

`Value` contiene **raw pointers** (`ObjRef = *mut ObjData`, etc.) que **nunca son reference-counted a nivel de allocación**. El `Heap` owns el lifetime, pero nada previene que un `Value` outlive su `Heap`.

**PROBLEMA CRÍTICO: `unsafe impl Send for Value {}` y `unsafe impl Sync for Value {}`**

```rust
// tsn-types/src/value/mod.rs:90-91
unsafe impl Send for Value {}
unsafe impl Sync for Value {}
```

Esto declara que `Value` es seguro para cruzar thread boundaries. Pero `Value` contiene raw pointers a allocaciones de heap thread-local. Si un `Value::Object` se transfiere entre VMs (vía `AsyncFuture`), y el VM originante hace drop, **el pointer dangling es use-after-free**.

**Esto es undefined behavior potencial que causaría crashes intermitentes bajo carga en producción.**

**Recomendación de Hejlsberg:** C# usa tracing GC por exactamente estas razones. Manual memory management en un runtime de lenguaje es una liability de confiabilidad. Para un lenguaje targeting systems programming, esto debe eventualmente ser:
1. Un proper tracing GC (generational, concurrent), o
2. Un modelo de ownership borrow-checked (como Rust), o
3. Al menos `Arc`-based object handles con shared heap

---

## 11. Manejo de Errores

### 11.1 Diagnósticos del Compilador

**Calificación: 4/10**

**Problemas:**
- **Sin error codes**: Ningún diagnóstico tiene código asociado. Compiladores de producción usan códigos como `TS2322`, `CS0103` para que usuarios busquen documentación
- **Mensajes inconsistentes en tono y calidad**:
  ```
  "Expected FatArrow, got Identifier at 5:3"        — Parser (expone internals)
  "type mismatch: cannot assign 'int' to 'str'"      — Checker (bueno)
  "unsupported expression variant"                    — Compiler (pobre)
  "undefined variable: x"                             — Checker (bueno)
  ```
- **Sin sugerencias**: TypeScript sugiere `"Did you mean 'length'?"` para typos en nombres de propiedades
- **Sin contexto**: No se muestra la línea de código fuente con el error marcado
- **Error recovery inexistente en checker y compiler**: Un error aborta el procesamiento de la función/archivo

### 11.2 Errores en Runtime

**Calificación: 7/10**

- Stack underflow checks en cada `pop()` — bueno
- Frame boundary validation — bueno
- Exception handler unwinding con stack truncation — correcto
- Error object creation con captured stack traces — bueno
- Finalmente inlined correctamente en compiler

**Problemas:**
- Panic en `Arc::get_mut().unwrap()` en operaciones de clase
- `unreachable!()` en bytecode malformed

---

## 12. Calidad del Código

### 12.1 Modularización

**Calificación: 9/10**

La modularización de "god files" es excelente. **Ningún archivo supera las 600 líneas**. El más grande es `tsn-parser/src/types.rs` con 593 líneas.

| Crate | Líneas | Archivos |
|-------|--------|----------|
| tsn-checker | 10,802 | 60 |
| tsn-lsp | 5,442 | 51 |
| tsn-vm | 4,779 | 37 |
| tsn-runtime | 4,331 | 40 |
| tsn-parser | 3,892 | 21 |
| tsn-compiler | 3,983 | 26 |
| tsn-cli | 3,128 | 29 |
| tsn-core | 3,146 | 22 |
| tsn-types | 1,234 | 16 |
| tsn-lexer | 1,315 | 12 |
| tsn-modules | 225 | 4 |
| tsn-op-macros | 262 | 1 |

### 12.2 Testing

**Calificación: 1/10 — PROBLEMA CRÍTICO**

**Solo ~8 tests unitarios en todo el proyecto de 42,500 líneas.** Todos en `tsn-core` (time.rs y doc.rs). **Cero tests** en lexer, parser, checker, compiler, VM, runtime, modules, CLI, LSP.

La única validación es correr `examples/main.tsn` (33 smoke tests a nivel de programa TSN). Esto es una **brecha crítica** para producción. TypeScript tiene ~100,000 tests.

**Sin tests, cada refactoring es una apuesta.**

### 12.3 Convenciones de Nombrado

**Calificación: 7/10**

- `snake_case` para módulos/funciones, `PascalCase` para tipos — consistente
- Mezcla de convenciones: `check_expr` (verb-noun), `infer_type` (verb-noun), pero `member_exists_cached` (noun-verb-adjective)
- Nombres opacos: `structural_core`, `structural_advanced`, `basic_ops`
- Buenos nombres: `emit_smart_pop`, `parse_binary_expr`, `resolve_type_node`, `chase_jump_target`
- Prefijo `stmt_` redundante dentro del módulo `stmts`

### 12.4 Duplicación de Código

**Problemas identificados:**
1. **`TypeContext` trait** duplicado en `Binder` y `BindResult` (~30 líneas cada uno, copia idéntica)
2. **`push_token`** llamado ~15 veces con argumentos casi idénticos en el lexer
3. **Number literal parsing** en `scan_number` tiene 3 code paths casi idénticos para binary/octal/hex
4. **Métodos `expect_*`** en `TokenStream`: `expect`, `expect_lexeme`, `expect_token` son 90% idénticos
5. **`Chunk` definido en dos crates**: `tsn-compiler/src/chunk.rs` y `tsn-types/src/chunk.rs` (re-exportado)

### 12.5 Código Unsafe Innecesario

**Calificación: 4/10**

- Raw pointers en `Compiler` para `type_annotations`, `extension_calls`, etc. — deberían ser `&'a` references con lifetime
- Pointer-as-cache-key en el checker — UB-adjacent
- `unsafe impl Send/Sync for Value` — potencial use-after-free (ver sección 10)

---

## 13. Tooling (LSP + CLI)

### 13.1 Language Server (`tsn-lsp`)

**Calificación: 8/10**

5,442 líneas, 51 archivos. Servidor LSP stdio-based usando `tower-lsp`.

**Features:**
- Hover, go-to-definition, completions, references, rename
- Document symbols, workspace symbols, semantic tokens
- Document highlights, folding ranges, inlay hints, signature help

**Arquitectura:** `Workspace` (DashMap de `DocumentState`) + `ProjectIndex` (module exports, name index, reverse deps). Limpio.

**Problemas:**
- Sin incremental checking — re-checkea todo el programa en cada cambio
- Sin cross-file caching — cada archivo se re-checkea desde cero

### 13.2 CLI (`tsn-cli`)

**Calificación: 8.5/10**

Comandos bien diseñados:
- `tsn <file.tsn>` — run
- `tsn -e "source"` — eval
- `tsn bench <file.tsn> [--runs=N]` — benchmark
- `tsn disasm <file.tsn>` — disassemble
- `tsn doctor` — environment diagnostics
- `tsn version` — version

Pipeline con caching entre ejecuciones. Debug flags para lex, parse, types, disasm.

---

## 14. Documentación

### 14.1 Docs Existentes

**Calificación: 7/10**

| Documento | Calidad | Comentario |
|-----------|---------|------------|
| `README.md` | 8/10 | Buen overview, ejemplos claros |
| `TSN-SPEC.md` | 7/10 | Spec funcional pero incompleta |
| `STDLIB.md` | 7/10 | Referencia adecuada |
| `GETTING_STARTED.md` | 8/10 | Tutorial accesible |
| `INSTALL.md` | 7/10 | Instrucciones claras |
| `ROADMAP.md` | 4/10 | **Desactualizado**: header dice "Current Version: 0.4 (Alpha)" pero README dice v0.9 |
| `AUDIT.md` | N/A | Plan de modularización (ya ejecutado) |
| `CONTRIBUTING.md` | 6/10 | Básico |
| `SECURITY.md` | 6/10 | Básico |

**Directorio `docs/tasks/`**: Vacío (17 archivos git-ignored). Debería limpiarse o documentarse.

---

## 15. Decisiones de Diseño — Aciertos

Las siguientes decisiones merecen aplauso desde la perspectiva de un diseñador de lenguajes:

| # | Decisión | Razón |
|---|----------|-------|
| 1 | Nullable normalization (`T?` → `Union`) | Elimina casos especiales infinitos. Superior a TypeScript |
| 2 | Modelo híbrido nominal/estructural | Clases = identidad nominal, objetos = intercambiables estructurales |
| 3 | `newtype` como first-class | Elimina problema "stringly-typed". Como Rust pero más ergonómico |
| 4 | `never` como bottom type | Correcto, permite unreachable code detection |
| 5 | Type predicates bidireccionales | Narrow en true, resta en false — correcto |
| 6 | Vtable dispatch | O(1) method lookup. Como C++/Java |
| 7 | `override` obligatorio | Previene override silencioso roto |
| 8 | Extension methods con zero overhead | Mangled name en compile time — elegante |
| 9 | `this` type para fluent APIs | Concrete receiver substitution — como Kotlin |
| 10 | Pipeline `|>` con `_` placeholder | Semántica unificada Hack-style |
| 11 | `using` para recursos | Verifica `Disposable` en compile time |
| 12 | Eliminación de `==`, `var`, `delete`, `void` | Shed JS baggage — madurez |
| 13 | Multi-phase checker | Bind → Enrich → Check. Como TypeScript |
| 14 | Caching con dirty counter | `infer_env_rev` invalidación clever |
| 15 | Pratt parsing | Correcto, extensible |
| 16 | Smart pop optimization | Atención a bytecode quality |
| 17 | Module registry como single source of truth | Previene divergencia checker/runtime |
| 18 | Modularización de god files | Ningún archivo >600 líneas |
| 19 | `rustc_hash::FxHashMap` | Correcto para performance de compiler |
| 20 | Exhaustiveness checking en match | Essencial para lenguaje seguro |

---

## 16. Decisiones de Diseño — Problemas

Las siguientes decisiones son cuestionables o requieren corrección:

| # | Decisión | Gravedad | Alternativa |
|---|----------|----------|-------------|
| 1 | `dynamic` en `print(...args: dynamic[])` | **CRÍTICA** | `print<T>(...args: T[])` con Display implícito |
| 2 | `unsafe impl Send/Sync for Value` con raw pointers | **CRÍTICA** | Arc-based handles o shared heap |
| 3 | Arena allocator sin GC | **CRÍTICA** | Tracing GC o ownership model |
| 4 | Sin tests unitarios (~8 en 42K líneas) | **CRÍTICA** | Tests obligatorios por crate |
| 5 | Sin reglas de coerción numérica | **ALTA** | Definir antes de v1.0 (Rust: explícito, C#: reglas estrictas) |
| 6 | `Result<T,E>` usa `dynamic` internamente | **ALTA** | Proper sum types: `enum Result<T,E> { Ok(T), Err(E) }` |
| 7 | Child VM por tarea async | **ALTA** | Coroutine frames más livianas o work-stealing |
| 8 | Monomorphic-only inline caching | **ALTA** | PIC 2-4 vías como V8 |
| 9 | Bytecode `u16` (64K límite) | **ALTA** | Variable-length encoding |
| 10 | Pointer-as-hash-key en cache | **ALTA** | IDs estables o indices |
| 11 | Sin error codes en diagnósticos | **MEDIA** | Catálogo como TS2322, CS0103 |
| 12 | `globals: Arc<RwLock<HashMap>>` | **MEDIA** | Indexed global slots |
| 13 | `std:types` sin runtime builder | **MEDIA** | Agregar builder o quitar del registry |
| 14 | Sin `Option<T>` type | **MEDIA** | Agregar con combinadores funcionales |
| 15 | Sin traits/mixins | **MEDIA** | Modelo de traits de Rust |
| 16 | `unsafe` raw pointers en Compiler | **MEDIA** | Lifetime-parameterized references |
| 17 | `TypeContext` duplicado | **MENOR** | Extraer a módulo compartido |
| 18 | `BindResult` con 17 campos | **MENOR** | Split en sub-objetos |
| 19 | `match (n)` con paréntesis | **MENOR** | Eliminar paréntesis |
| 20 | ROADMAP desactualizado (v0.4 vs v0.9) | **MENOR** | Actualizar header |
| 21 | `std:temporal` implementado pero no registrado | **MENOR** | Registrar o marcar in-progress |
| 22 | Ejemplo 26 faltante | **MENOR** | Agregar o renumerar |
| 23 | Archivo `test_math_tmp.tsn` residual | **MENOR** | Eliminar |
| 24 | Directorio `intrinsic/` vacío en tsn-vm | **MENOR** | Eliminar directorio |

---

## 17. Legacy y Deuda Técnica

### 17.1 Deuda Técnica Acumulada

| Categoría | Descripción | Impacto | Esfuerzo Estimado |
|-----------|-------------|---------|-------------------|
| **Testing** | Ausencia de tests unitarios e integración | Bloquea producción | 3-6 meses |
| **Memory Safety** | `Send/Sync` unsafe para Value con raw pointers | Crashes potenciales | 2-4 semanas |
| **GC** | Arena allocator sin collection para procesos largos | Memory leaks en servidores | 2-3 meses |
| **Error Codes** | Diagnósticos sin códigos ni sugerencias | UX deficiente | 1-2 semanas |
| **Numéricos** | Sin reglas de coerción definidas | Bugs silenciosos | 1 semana |
| **Bytecode limits** | `u16` limita a 64K instrucciones/cnstes/jumps | No escala a programas grandes | 2-3 semanas |
| **Inline Caching** | Monomorphic-only, thrash en polimórfico | Performance impredecible | 1-2 meses |
| **Duplicación** | `TypeContext`, `push_token`, `expect_*`, `Chunk` | Mantenimiento | 1 semana |
| **Nombres opacos** | `structural_core`, `structural_advanced` | Onboarding difícil | 2 días |
| **ROADMAP** | Versión desactualizada (0.4 vs 0.9) | Confusión | 1 día |

### 17.2 Código Residual

- `examples/test_math_tmp.tsn` — archivo de test temporal
- `docs/tasks/` — directorio vacío con 17 archivos git-ignored
- `crates/tsn-vm/src/intrinsic/` — directorio vacío (intrinsics movidos a tsn-runtime)
- Ejemplo 26 faltante (salta de 25 a 27)
- `std:temporal` implementado pero no en registry

### 17.3 Inconsistencias entre Crates

| Inconsistencia | Crates | Descripción |
|----------------|--------|-------------|
| `Chunk` duplicado | tsn-compiler, tsn-types | Definido en compiler, re-exportado en types |
| `IntrinsicId` gaps | tsn-core | Valores no contiguos (gaps en 57-59, 68-69, 88-89, etc.) — frágil |
| Registro vs Builder | tsn-modules, tsn-runtime | `std:types` en registry pero sin builder |
| Versión README vs ROADMAP | docs | README dice v0.9, ROADMAP dice v0.4 |

---

## 18. Hoja de Ruta Crítica

Priorizada por impacto en producción. Las tareas están ordenadas por gravedad del problema que resuelven.

### Fase 0: Seguridad y Corrección (Bloqueantes de v1.0)

| # | Tarea | Razón | Criterio de Aceptación |
|---|-------|-------|----------------------|
| 1 | **Eliminar `unsafe impl Send/Sync for Value`** | UB potencial, crashes bajo carga | Value usa Arc-based handles o shared heap. `cargo clippy` sin warnings unsafe |
| 2 | **Definir reglas de coerción numérica** | Sin reglas = bugs silenciosos | Spec documentada + tests de coerción int→float, float→decimal, etc. |
| 3 | **Reemplazar `dynamic` en builtins** | Socava type system desde día 1 | `print` y otros builtins usan generics o uniones tipadas |
| 4 | **Reemplazar `dynamic` en `Result<T,E>`** | Erasure interna es violación de tipos | `enum Result<T,E> { Ok(T), Err(E) }` con sum types proper |
| 5 | **Agregar tests unitarios por crate** | Sin tests, cada cambio es una apuesta | Mínimo 50 tests por crate core (lexer, parser, checker, compiler, vm) |
| 6 | **Agregar tests de integración de pipeline** | Los 33 smoke tests no son suficientes | Tests con source input → expected output (tokens, AST, types, bytecode) |

### Fase 1: Calidad de Producción (Pre-requisitos para adopción)

| # | Tarea | Razón | Criterio de Aceptación |
|---|-------|-------|----------------------|
| 7 | **Error codes en diagnósticos** | UX profesional, documentación | Catálogo de 50+ códigos con documentación online |
| 8 | **Mensajes de error con sugerencias** | UX, reduce frustración de usuarios | `"Did you mean 'length'?"` para typos, snippets de código |
| 9 | **Polymorphic Inline Cache (2-4 vías)** | Performance predecible | Property access polimórfico no degrada a hash lookup siempre |
| 10 | **Variable-length bytecode encoding** | Escala a programas grandes | Sin límite de 64K instrucciones |
| 11 | **Reemplazar unsafe pointers en Compiler** | Safety audit limpio | Lifetimes en lugar de `*const T` |
| 12 | **Fix `std:types` sin builder** | Error en runtime si se importa | Agregar builder o quitar del registry |
| 13 | **Indexed global slots** | Performance de acceso a variables | O(1) indexado en lugar de RwLock<HashMap> |

### Fase 2: Madurez del Lenguaje (Para v1.0)

| # | Tarea | Razón | Criterio de Aceptación |
|---|-------|-------|----------------------|
| 14 | **Proper sum types / ADTs** | Reemplaza `dynamic`, habilita destructuring | `enum Result<T,E> { Ok(T), Err(E) }` con pattern matching destructuring |
| 15 | **Destructuring en match** | Feature esencial de pattern matching | `Circle(r) => 3.14 * r * r` |
| 16 | **Traits/mixins** | Composición behavioral múltiple | `impl Display for Point { ... }` |
| 17 | **`Option<T>` con combinadores** | Toolkit funcional para nulabilidad | `map`, `andThen`, `unwrap_or`, etc. |
| 18 | **Operador `?` para Result** | Ergonomía crítica | `let x = try_something()?` propaga error automáticamente |
| 19 | **Tracing GC o ownership model** | Procesos largos sin memory leaks | Servidor HTTP corre 24h sin growth de memoria |
| 20 | **Incremental checking** | LSP responsive en proyectos grandes | Re-check solo archivos afectados por cambio |

### Fase 3: Ecosistema (Post-v1.0)

| # | Tarea | Razón |
|---|-------|-------|
| 21 | Package manager con versioning | Dependency resolution para third-party |
| 22 | Module resolution general | Soporte para `@npm:`, `github:`, etc. |
| 23 | JIT o baseline compiler | Performance compute-bound |
| 24 | Work-stealing scheduler | Paralelismo multi-core |
| 25 | Higher-Kinded Types | Para `Monad`, `Functor` en stdlib |

---

## 19. Calificación Final

### Resumen por Categoría

| Categoría | Calificación | Peso | Ponderado |
|-----------|-------------|------|-----------|
| Diseño del Lenguaje | 8/10 | 15% | 1.20 |
| Sistema de Tipos | 8.5/10 | 20% | 1.70 |
| Modelo de Objetos | 8/10 | 10% | 0.80 |
| Async/Await | 7/10 | 10% | 0.70 |
| Pattern Matching | 7/10 | 5% | 0.35 |
| Sistema de Módulos | 9/10 | 5% | 0.45 |
| Biblioteca Estándar | 7.5/10 | 5% | 0.38 |
| Compilador | 8.5/10 | 10% | 0.85 |
| VM | 8.5/10 | 5% | 0.43 |
| Gestión de Memoria | 3/10 | 5% | 0.15 |
| Manejo de Errores | 4/10 | 5% | 0.20 |
| Calidad del Código | 7/10 | 5% | 0.35 |
| **TOTAL** | | **100%** | **7.56 / 10** |

### Veredicto Final: **7.6 / 10 — Buen Fundamento, Ejecución Incompleta**

TSN tiene **las decisiones arquitectónicas correctas en las áreas que más importan**: shape-based objects, vtable dispatch, cooperative async, multi-phase type checking, Pratt parsing, nullable normalization. La base es sólida y el camino hacia producción está claro: features incrementales, no rediseño.

**Lo que separa a TSN de producción** no es la arquitectura sino la **ingeniería de producción**: tests, seguridad de memoria, mensajes de error, y memory management. Estas son tareas de ejecución, no de diseño. Con 5-10 personas-año de ingeniería VM enfocada, TSN podría ser un lenguaje de producción competitivo.

### Perspectiva de Hejlsberg

**Lo que aprobaría:**
- Las correcciones desde JavaScript (eliminar `==`, `var`, `delete`, `void`) muestran madurez. TypeScript heredó baggage de JS; TSN está dispuesto a sheddingarlo
- Nullable normalization es una versión más limpia de lo que TypeScript hace
- El tipo `this` para fluent APIs es el tipo de feature pragmática de systema de tipos que él defiende
- Named arguments en sitio de llamada son una feature QoL que TypeScript no tiene
- El enfoque incremental, fase por fase, para generics y features de tipos es sensato

**Lo que cuestionaría:**
- **`dynamic` en core APIs**: Pushiría por overloading proper o union types en su lugar
- **La complejidad de tipos avanzados planeados** (condicionales, mapeados, template literal): Argumentaría que son acomodos específicos de TypeScript que no pertenecen en un lenguaje compilado donde el runtime puede hacer el trabajo de forma diferente
- **Herencia simple**: Diseñó C# con herencia simple pero luego agregó default interface methods. TSN enfrentará la misma presión
- **Arquitectura async**: Child VM por tarea es inusual. Favorecería un enfoque de coroutine más convencional
- **Ausencia de tests**: No negociable en revisión de código en Microsoft

**Los gaps críticos para v1.0:**
1. **Reglas de coerción numérica** — Deben definirse antes de que el lenguaje se considere completo
2. **Proper sum types / ADTs** — Necesarios para reemplazar `dynamic` en `Result`, habilitar pattern matching con destructuring, y hacer el sistema de tipos genuinamente útil
3. **Package management** — Sin él, el sistema de módulos solo sirve al stdlib
4. **Eliminar `dynamic` de builtin APIs** — O al mínimo, restringirlo a fronteras FFI y proveer `print<T>(...args: T[])` con formatting basado en type-class
5. **Tests** — La infraestructura de tests es la red de seguridad sin la cual ningún refactoring es seguro

---

## Apéndice A: Issues de Seguridad Identificados

| ID | Severidad | Descripción | Archivo | Línea |
|----|-----------|-------------|---------|-------|
| SEC-001 | **CRÍTICA** | `unsafe impl Send/Sync for Value` con raw pointers a heap thread-local | `tsn-types/src/value/mod.rs` | 90-91 |
| SEC-002 | **CRÍTICA** | Pointer-as-hash-key para cache (UB si AST se mueve) | `tsn-checker/src/checker/mod.rs` | Múltiples |
| SEC-003 | **ALTA** | `Arc::get_mut().unwrap()` panic en class objects compartidos | `tsn-vm/src/vm/exec/class.rs` | 19, 38, 48, 58 |
| SEC-004 | **ALTA** | Cross-VM Value transfer con arena allocator (dangling pointer potencial) | `tsn-vm/src/runtime/heap.rs` | Todo el módulo |
| SEC-005 | **MEDIA** | `unreachable!()` asume corrección del compiler | `tsn-vm/src/vm/exec/*.rs` | Múltiples |

## Apéndice B: Métricas del Proyecto

| Métrica | Valor | Benchmark |
|---------|-------|-----------|
| Líneas totales (Rust) | ~42,500 | TypeScript: ~1.2M |
| Tests unitarios | ~8 | TypeScript: ~100K |
| Archivos más grande | 593 líneas (`parser/types.rs`) | God file threshold: 400 |
| Crates | 12 | — |
| OpCodes | 102 | JVM: ~200, Lua: ~40 |
| Módulos stdlib | 17 + 3 builtins = 20 | Python stdlib: ~200 |
| Ejemplos | 28 (+ 1 tmp) | — |
| Documentación | 6 docs principales | — |

## Apéndice C: Comparación con Lenguajes de Referencia

| Feature | TSN | TypeScript | C# | Rust |
|---------|-----|------------|----|-----|
| Tipado | Estático, gradual | Estático, gradual | Estático | Estático, ownership |
| Memoria | Arena (sin GC) | V8 GC | Tracing GC | Ownership + RC |
| Null safety | `T?` → `Union` | `T \| null` | `T?` (nullable ref types) | `Option<T>` |
| Async | Child VM + scheduler | Promises + event loop | async/await + Task | async/await + Future |
| Pattern matching | `match` básico | No nativo | Pattern matching | `match` completo |
| Traits | ❌ | ❌ | Interfaces + default impls | ✅ (core feature) |
| Sum types | ❌ (planificado) | ❌ | ✅ (records discriminated) | ✅ (`enum`) |
| Error codes | ❌ | ✅ (TS####) | ✅ (CS####) | ✅ (E####) |
| Package manager | ❌ | npm | NuGet | crates.io |
| Tests | ~8 | ~100K | ~1M | ~80K |
| GC | Arena | Generational (V8) | Generational (concurrent) | Ownership |

---

*Esta auditoría fue realizada evaluando cada decisión del lenguaje TSN bajo los criterios de diseño que Anders Hejlsberg aplicaría: seguridad, ergonomía, performance, mantenibilidad, y viabilidad de producción. El proyecto demuestra un talento excepcional en arquitectura de lenguajes; las brechas identificadas son de ingeniería de producción, no de diseño fundamental.*

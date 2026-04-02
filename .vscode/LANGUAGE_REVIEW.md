# TSN — Revisión crítica de diseño
### Perspectiva: Anders Hejlsberg, mentor

> *"El mejor lenguaje no es el que tiene más características. Es el que tiene las características correctas, diseñadas con coherencia interna. Cada decisión se paga durante décadas."*

---

## Prefacio

Este documento asume que la meta es exactamente lo que se declaró: el TypeScript que Anders habría querido construir desde cero, sin deuda de compatibilidad con JavaScript ni V8. Eso cambia todo. Cuando no hay legado que proteger, las malas decisiones son imperdonables.

Lo que sigue no es una lista de bugs. Es una revisión de las decisiones de diseño del lenguaje como sistema, juzgadas con el estándar que un lenguaje de producción requiere.

---

## I. Lo que está bien hecho

Antes de la crítica, lo que merece crédito genuino:

### ✅ Correcto por diseño

| Decisión | Por qué es buena |
|----------|-----------------|
| `decimal` con sufijo literal `1.5d` | TypeScript nunca lo resolvió. Necesario para finanzas. |
| `char` como tipo separado | Correcto. `str[0]` que devuelve `str` es un error semántico. |
| Match con exhaustividad obligatoria | TypeScript requiere hacks (`never`) para esto. Aquí es nativo. |
| `newtype` como tipo nominal | Branded types de TS son un workaround. Esto es semántica real. |
| Argumentos nombrados | Elimina la ambigüedad de `createServer(8080, true, false, null)`. |
| `using` / `await using` | La gestión de recursos como ciudadano de primera clase. Correcto. |
| `never` como bottom type | Semánticamente sólido. |
| `unknown` como top type (no `any`) | La decisión correcta desde el día uno. |
| No hay `null` implícito | `T` y `T | null` son distintos. Correcto. |
| `_` como descarte unificado | Una semántica, múltiples contextos. Elegante. |
| Enums como tipos opacos sin aritmética | `Color.Red + 1` debe ser error. Lo es. Correcto. |
| Generics con 5 fases | El enfoque más completo que he visto en un lenguaje artesanal. |
| `override` obligatorio | Lo mismo que pusimos en C# 9. Debería ser siempre así. |

---

## II. Problemas fundamentales

Estos no son preferencias. Son defectos de diseño que escalan mal.

---

### 🔴 Problema 1: Cuatro tipos numéricos sin jerarquía clara

El lenguaje tiene `int`, `float`, `decimal`, y `bigint`.

```tsn
const a: int     = 42
const b: float   = 3.14
const c: decimal = 1.5d
const d: bigint  = 999999999999n
```

El problema no es tenerlos. El problema es que **no hay reglas de coerción** entre ellos. ¿`int + float` produce qué? ¿`decimal + int` es válido? ¿`bigint` y `int` son intercambiables?

En la práctica, el usuario va a escribir:
```tsn
function precio(cantidad: int, valor: decimal): decimal {
    return cantidad * valor  // ← error? coerción? comportamiento definido?
}
```

**El diagnóstico real**: `bigint` es redundante si `int` puede ser de precisión arbitraria (como en Haskell/Python). Si `int` es 64-bit (como en Rust), `bigint` tiene sentido pero necesita coerción explícita con sintaxis limpia.

**Propuesta**:
- `int` → 64-bit signed (rendimiento, el caso más común)
- `float` → IEEE 754 double
- `decimal` → precisión arbitraria para dinero
- Eliminar `bigint` o renombrarlo `int128` / `largeint` con semántica clara
- Definir una tabla de coerción numérica explícita: `int op float → float`, `int op decimal → decimal`

---

### 🔴 Problema 2: `==` y `===` coexisten — herencia envenenada de JavaScript

Este es el error más grave que TSN hereda innecesariamente.

```tsn
if (a == b)   // ¿Coerción de tipos? ¿Valor? ¿Referencia?
if (a === b)  // ¿Identidad? ¿Valor estricto?
```

En JavaScript, `==` hace coerción implícita (`"1" == 1` es `true`). En TSN con sistema de tipos estricto, `"1" == 1` debería ser error de compilación porque los tipos son incomparables. Entonces `==` y `===` producen el mismo resultado en un lenguaje con tipos sólidos — **son semánticamente idénticos**, lo que los hace redundantes.

**Propuesta**:
```tsn
==   // Igualdad por valor (el único operador de igualdad)
is   // Identidad de referencia cuando sea necesario
!=   // No-igualdad por valor
```

Dos operadores de igualdad que significan lo mismo son ruido cognitivo. Si el sistema de tipos garantiza que solo se comparan valores compatibles, uno es suficiente.

---

### 🔴 Problema 3: `var` sigue existiendo

Si construyes un lenguaje desde cero sin compatibilidad con JavaScript, `var` no debe existir. Punto.

`var` es function-scoped, hoisted, y reentrante. `let`/`const` resuelven todos esos problemas. Tener `var` en TSN solo confunde a usuarios que vienen de JS pensando que funciona igual.

**Decisión**: Eliminar `var`. Solo `let` (mutable) y `const` (inmutable).

---

### 🔴 Problema 4: `with` statement

```tsn
with (obj) {
    name = "Alice"  // ¿Qué `name`? ¿El de obj? ¿El del scope exterior?
}
```

`with` hace el scope irresoluble estáticamente. Esto rompe toda posibilidad de análisis de flujo, renombrado de símbolos, optimización del compilador, y análisis LSP. JavaScript lo removió en strict mode por exactamente esta razón.

En un lenguaje con sistema de tipos sólido, `with` es incompatible con los invariantes del compilador.

**Decisión**: Eliminar `with`. Sin excepción.

---

### 🔴 Problema 5: Sin ADTs (Algebraic Data Types) — el hueco más costoso

Este es el problema de diseño más serio del sistema de tipos.

TSN tiene `match` con exhaustividad. Tiene `enum`. Tiene clases. Pero no tiene **sum types de datos**:

```tsn
// Lo que TSN TIENE (con clases):
class Circle { radius: float }
class Rectangle { width: float, height: float }
type Shape = Circle | Rectangle  // Union de clases nominales

// Lo que TSN NECESITA (ADTs):
type Shape =
    | Circle(radius: float)
    | Rectangle(width: float, height: float)
    | Triangle(base: float, height: float)
```

La diferencia es fundamental:
1. Con clases necesitas `new Circle(5.0)` — ceremonia de construcción
2. Con ADTs tienes `Circle(5.0)` — construcción directa
3. Con clases, `Circle` es una entidad global del scope; con ADTs es un constructor local al tipo
4. Los ADTs se desestructuran directamente en match; las clases requieren asignación manual

El resultado actual es que el `match` expression existe pero es subóptimo porque no hay forma limpia de modelar datos con variantes. Los usuarios acaban usando jerarquías de clases innecesariamente complejas para algo que debería ser trivial.

**Propuesta**: Sum types como ciudadanos de primera clase:
```tsn
type Result<T, E> =
    | Ok(value: T)
    | Err(error: E)

type Option<T> =
    | Some(value: T)
    | None

// Uso:
const r: Result<int, str> = Ok(42)
match r {
    Ok(v) => print(v),
    Err(e) => print("Error: " + e)
}
```

Sin ADTs, el match expression es un feature a medias.

---

### 🔴 Problema 6: Manejo de errores sin estrategia

TSN tiene `throw/catch`. Eso es todo.

Las excepciones son poor man's control flow. En un lenguaje diseñado desde cero, la pregunta correcta es: **¿qué quieres que le sea difícil al usuario hacer mal?**

Con excepciones, es fácil ignorar errores (`try {} catch {}`). Es fácil olvidar que una función puede fallar. Los tipos de error no aparecen en la firma de la función.

Con `Result<T, E>` (que TSN tiene como stdlib pero no como idioma), el error es parte del tipo:

```tsn
// Con excepciones (actual):
function divide(a: int, b: int): int {
    if (b == 0) throw new DivisionError()
    return a / b
}
// El caller no sabe que puede fallar a menos que lea la implementación.

// Con Result (propuesta):
function divide(a: int, b: int): Result<int, DivisionError> {
    if (b == 0) return Err(DivisionError("division by zero"))
    return Ok(a / b)
}
// El caller DEBE manejar el caso de error. El tipo lo garantiza.
```

**Propuesta**:
1. Hacer `Result<T, E>` un tipo builtin (no solo stdlib)
2. Añadir operador `?` para propagación: `const v = divide(a, b)?` (retorna el `Err` si hay error)
3. Mantener `throw/catch` solo para errores realmente excepcionales (panics de sistema)

---

### 🔴 Problema 7: `delete` y `void` como operadores unarios

```tsn
delete obj.prop  // Rompe el tipo de obj en runtime
void expr        // Evalúa y descarta — para eso está `_`
```

`delete` es incompatible con un sistema de tipos estático. Si `obj: { name: str, age: int }`, después de `delete obj.name`, ¿cuál es el tipo de `obj`? O `name` se vuelve `str | undefined` (rompe el tipo original), o el compilador tiene que rastrear mutaciones de forma (como análisis de flujo de tipos, lo cual es muy caro).

`void` como expresión unaria es completamente reemplazable por el descarte con `_`:
```tsn
_ = sideEffect()  // En lugar de void sideEffect()
```

**Decisión**: Eliminar `delete`. Eliminar `void` como operador unario.

---

## III. Problemas de diseño importantes

Significativos pero no bloqueantes. Deben corregirse antes de v1.0.

---

### 🟡 Problema 8: `str` vs `string` — denominación inconsistente

Los primitivos son: `int`, `float`, `decimal`, `bigint`, `str`, `char`, `bool`.

`str` es la convención de Rust y Python. En un lenguaje diseñado para ser legible, `string` comunica más claramente a alguien que viene de cualquier otro lenguaje. C#, Java, Kotlin, Swift, Go: todos usan `string` o `String`.

Esto es menor pero la consistencia importa. Si el objetivo es legibilidad como documentación, los nombres de tipos deben ser autoexplicativos.

**Propuesta**: Renombrar `str` a `string`. (O mantener ambos como alias con `str` siendo la forma corta para quienes la prefieran.)

---

### 🟡 Problema 9: Regla de argumentos nombrados — cognitivamente costosa

La regla actual: *todos los args posicionales antes de los nombrados*.

```tsn
createServer(8080, host: "localhost", tls: true)  // OK
createServer(port: 8080, "localhost", tls: true)  // ERROR: posicional después de nombrado
```

El problema: el usuario tiene que saber qué parámetros son "posicionales" antes de llamar. Esto crea una categorización invisible en la mente del programador.

La alternativa de Swift es más limpia: **todos los argumentos tienen etiqueta de llamada por defecto**. El autor de la función decide si la etiqueta es pública (externa) o suprimida (`_`):

```tsn
// Swift-style: la etiqueta es parte de la firma pública
function move(from source: Point, to destination: Point): void { ... }
move(from: a, to: b)  // El sitio de llamada es auto-documentado

// Para casos donde no quieres etiqueta:
function sqrt(_ value: float): float { ... }
sqrt(2.0)  // Sin etiqueta requerida
```

Esto es más verbose en la declaración pero elimina la ambigüedad en el sitio de llamada.

---

### 🟡 Problema 10: Pipeline `|>` — dos semánticas conflictivas

```tsn
x |> f()           // ¿Qué pasa? ¿x es el primer arg? ¿Error?
x |> f(_, config)  // x se pasa en la posición de _
```

El pipeline operator tiene dos modos: con `_` explícito (Hack-style) y sin `_` (Elixir-style). Mezclar ambos crea ambigüedad mental.

Si usas Hack-style (recomendado para un lenguaje con tipos), `_` debería ser SIEMPRE obligatorio en el RHS. Esto elimina la pregunta "¿dónde va el valor?" y hace el código más explícito:

```tsn
// Siempre explícito:
result = input
    |> validate(_, rules)
    |> transform(_, config)
    |> serialize(_)
```

---

### 🟡 Problema 11: Structs sin semántica definida

`struct` existe en el parser y el checker pero sin semántica clara respecto a `class`.

En C#, la distinción struct/class tiene significado preciso: structs son value types (en stack), clases son reference types (en heap). En Rust, lo mismo. Sin esa distinción semántica, `struct` es solo ruido sintáctico que confunde a usuarios que vienen de otros lenguajes.

**Decisión**: O `struct` es un value type con semántica de copia en asignación, o debe eliminarse. No puede existir como keyword sin comportamiento diferenciado.

---

### 🟡 Problema 12: `char` y Unicode — la promesa que no se puede cumplir

`str[0]` devuelve `char`. `char` es un Unicode scalar value (U+0000–U+10FFFF).

El problema: en Unicode moderno, la unidad de percepción del usuario es el **grapheme cluster**, no el scalar value. `"👨‍👩‍👧"` es un grapheme cluster compuesto por 5 scalars (U+1F468 + ZWJ + U+1F469 + ZWJ + U+1F467). Si `str[0]` devuelve `char`, devuelve `👨` (solo el hombre), no la familia.

Esto no tiene solución limpia si se expone `char` directamente. Las opciones son:
1. `str[0]` devuelve `int` (code point) — preciso pero poco ergonómico
2. `str[0]` devuelve `string` (grapheme cluster de length 1) — mejor para display, peor para manipulación
3. Tener iteradores separados: `.chars()` (scalars), `.graphemes()` (clusters) — lo correcto pero más complejo

**Propuesta**: El tipo `char` debe llevar un comentario de advertencia explícito en la documentación y ser renombrado `codepoint` o `rune` para comunicar exactamente qué es. `str[0]: rune` es más honesto que `str[0]: char`.

---

### ✅ Resuelto: Extension methods

Extension methods ya existen de punta a punta con sintaxis estilo Dart adaptada a ECMAScript:

```tsn
extension StringTools on str {
    shout(): str {
        return this + "!"
    }

    get sizeHint(): int {
        return this.length
    }
}
```

La implementación actual cubre parser, binder, checker, compiler, VM y LSP, incluyendo:
- method calls
- optional chaining
- method values ligados al receiver
- getters y setters
- completado y hover

Este ya no es un hueco de diseño. La deuda restante aquí es evolutiva: seguir alineando la ergonomía con el resto del lenguaje.

---

### 🟡 Problema 14: 200+ intrinsics es demasiado

El diseño de `IntrinsicId` tiene 200+ entradas. Esto viola el principio más importante del diseño de intrinsics:

> *Un intrinsic debe existir solo cuando es imposible implementar la operación en el propio lenguaje con rendimiento aceptable.*

Actualmente `__str_trim`, `__str_split`, `__str_replace`, y docenas de métodos de string son intrinsics. Estos deberían ser métodos normales del tipo `string` implementados en TSN (posiblemente llamando a un pequeño conjunto de primitivas de bajo nivel).

La tabla de intrinsics debería tener ~20-30 entradas: I/O real del sistema, criptografía, red, tiempo, aleatoridad. Todo lo demás puede y debe ser TSN.

**Beneficio**: Reduce el acoplamiento compiler↔VM, hace el stdlib auditarle en TSN, y permite que usuarios avanzados extiendan el lenguaje sin tocar Rust.

---

## IV. Oportunidades perdidas — lo que debería existir

---

### 💡 Oportunidad 1: Records como tipo de datos inmutable

```tsn
// Propuesta: record para datos inmutables por valor
record Person {
    name: string
    age: int
}

const p = Person { name: "Alice", age: 30 }
const p2 = p with { age: 31 }  // Copy con modificación (no muta p)
```

`record` en C# 9 es la característica más solicitada de la última década. Modelar datos puros sin mutabilidad es el patrón más común en código moderno. Clases son demasiado para esto.

---

### 💡 Oportunidad 2: Protocolo Iterable tipado

```tsn
// Actualmente: for-of funciona "por magia" en el runtime
for (const x of myCollection) { ... }

// Debería ser tipado:
interface Iterable<T> {
    [Symbol.iterator](): Iterator<T>
}

interface Iterator<T> {
    next(): { value: T, done: bool }
}
```

Sin `Iterable<T>` en el sistema de tipos, no se puede verificar que un objeto es iterable en tiempo de compilación. El compilador acepta `for (const x of 42)` y falla en runtime.

---

### 💡 Oportunidad 3: Operador `?` para propagación de errores

Con ADTs y `Result<T, E>`:

```tsn
function readConfig(path: string): Result<Config, IoError> {
    const text = readFile(path)?       // Propaga IoError si falla
    const json = JSON.parse(text)?     // Propaga ParseError si falla
    return Ok(parseConfig(json))
}
```

El `?` elimina el boilerplate de `match result { Ok(v) => v, Err(e) => return Err(e) }`. Es la diferencia entre manejo de errores como ciudadano de primera clase vs como ruido sintáctico.

---

### 💡 Oportunidad 4: Traits o Default Interface Methods

Sin herencia múltiple de implementación, ¿cómo se reutiliza comportamiento?

```tsn
// Opción A: Default interface methods (C# 8, Java 8)
interface Printable {
    toString(): string

    // Implementación por defecto
    print(): void {
        console.log(this.toString())
    }
}

// Cualquier clase que implemente Printable obtiene print() gratis
```

Sin esto, el patrón es: copiar código, crear clase base artificial, o usar funciones libres. Ninguna es ideal.

---

### 💡 Oportunidad 5: `const` en parámetros de función

```tsn
// Actualmente: se puede mutar cualquier parámetro
function process(items: string[]): void {
    items.push("extra")  // ¿Intencional o bug?
}

// Propuesta: parámetros inmutables por defecto, mutable explícito
function process(const items: string[]): void {
    items.push("extra")  // Error de compilación
}
```

Los parámetros mutables son una fuente constante de bugs sutiles. En un lenguaje diseñado desde cero, los parámetros deberían ser inmutables por defecto.

---

### 💡 Oportunidad 6: Typed string literals (Template Literal Types)

Están en el spec pero sin implementar. Son más poderosos de lo que parecen:

```tsn
type EventName = `on${string}`      // "onClick", "onSubmit", etc.
type CSSProperty = `--${string}`    // Variables CSS
type ApiEndpoint = `/api/${string}` // Rutas tipadas

function on<T extends EventName>(event: T, handler: () => void): void
```

Esto permite type-safe string APIs sin codegen. En TypeScript esto fue revolucionario para el ecosistema.

---

## V. Evaluación de la arquitectura de implementación

Para separar el diseño del lenguaje de la implementación:

### ✅ Decisiones arquitectónicas sólidas

- **Compilación a bytecode**: Correcto para esta fase. Extensible.
- **VM de stack vs registros**: Stack es más simple. La penalización de rendimiento es aceptable hasta optimizar.
- **Intrinsics con tabla de despacho O(1)**: La única forma correcta de hacerlo.
- **Checker separado del compiler**: Separa preocupaciones. Extensible para LSP.
- **Binder antes del checker**: Dos pasadas es la arquitectura correcta.
- **Shape optimization para objetos**: Esencial para rendimiento. Bien ejecutado.

### ⚠️ Deuda técnica a pagar

| Deuda | Impacto |
|-------|---------|
| Spread en llamadas a funciones ahora implementado en compiler/VM | Bajo — falta ampliar la precisión estática para casos más ricos |
| Extension methods resueltos | Bajo — deuda principal es ergonomía futura, no soporte base |
| 200+ intrinsics en tabla plana | Medio — mantenimiento costoso |
| `using` sin soporte en el compiler para early-exit con excepciones | Alto — el recurso no se libera si hay throw |
| Varianza genérica declarada pero no enforced | Alto — da falsa seguridad |

---

## VI. Resumen ejecutivo — las 10 decisiones

Si tuviera que priorizar, estas son las 10 decisiones a tomar antes de declarar el lenguaje "diseñado correctamente":

| # | Decisión | Urgencia |
|---|----------|----------|
| 1 | Eliminar `var`, `==`, `===`, `delete`, `void` (operador), `with` — la herencia de JS | Crítica |
| 2 | Definir ADTs como sum types de primera clase | Crítica |
| 3 | Estrategia de manejo de errores: `Result<T,E>` + `?` como ciudadano nativo | Crítica |
| 4 | Tabla de coerción numérica explícita y decidir si `bigint` vs `int` | Alta |
| 5 | Mantener extension methods coherentes con la sintaxis de clases (`get`/`set`, docs, ergonomía) | Media |
| 6 | Extender el análisis estático de spread en llamadas más allá de arrays simples | Media |
| 7 | `struct` con semántica de value type o eliminarlo | Alta |
| 8 | Implementar `Iterable<T>` en el sistema de tipos | Media |
| 9 | Añadir `record` como tipo inmutable por valor | Media |
| 10 | Reducir intrinsics a ~30, mover el resto a TSN puro | Media |

---

## Nota final

Lo que tienes es un lenguaje con las decisiones correctas en las partes difíciles (generics, match exhaustivo, newtype, decimal, char, using) y las decisiones incorrectas en las partes que parecían fáciles (operadores de JS, var, error handling, ADTs).

El primer grupo demuestra que el diseñador piensa como un diseñador de lenguajes.
El segundo grupo demuestra que el copiar de JavaScript fue la decisión de menor resistencia.

La buena noticia: las partes incorrectas son eliminaciones y adiciones, no refactores profundos del compilador. El sistema de tipos tiene la arquitectura correcta para soportar ADTs. El match existe. El compilador es extensible.

La pregunta no es si el lenguaje puede ser bueno. La pregunta es si tienes la disciplina de remover lo que no debería estar ahí.

> *"Perfection is achieved not when there is nothing more to add, but when there is nothing left to take away."*
> — Antoine de Saint-Exupéry

---

*Este documento debe revisarse después de implementar los cambios. Las secciones marcadas con 🔴 deben resolverse antes de cualquier release público.*

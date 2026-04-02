---
description: Deben estar presentes siempre que se modifique o extienda el proyecto, para garantizar consistencia, calidad y ausencia de deuda técnica.
applyTo: '**'
---

## Entorno

El proyecto es un compilador e intérprete escrito completamente en **Rust** bajo un workspace Cargo con los crates `tsn-core`, `tsn-lexer`, `tsn-parser`, `tsn-compiler`, `tsn-vm`, `tsn-cli`. No existe base TypeScript activa; los paquetes bajo `packages/` son legacy y no deben modificarse.

Se usa `cargo` para compilar y ejecutar. El binario principal es `tsn-cli`. Todos los builds deben terminar con **0 warnings y 0 errors**.

## Reglas de código Rust

- Prohibido `unwrap()` en código de producción; usar `?`, `expect` con mensaje descriptivo, o manejo explícito de errores.
- Prohibido el uso de `unsafe` salvo que sea estrictamente necesario y esté documentado con justificación.
- Prohibido `clone()` innecesario; preferir referencias y lifetimes adecuados.
- El tipo `_` en patrones debe usarse sólo cuando el valor realmente no importa, nunca para silenciar errores de compilación.
- Toda función pública debe tener documentación `///`.
- Los `match` deben ser exhaustivos y explícitos; prohibido el uso de `_ =>` como comodín cuando los casos son enumerables.

## Arquitectura y modularidad

- Cada crate tiene una única responsabilidad; la lógica no debe cruzar fronteras de crate si no pertenece a esa capa.
- La lógica pura (sin dependencias de runtime ni de Value) va en `tsn-core`.
- Las utilidades compartidas dentro de un crate van en un módulo `helpers` dedicado; nunca se duplica código entre módulos.
- Pensar siempre en extensibilidad: si una abstracción va a necesitar variantes en el futuro, modelarla con un trait o un enum desde el principio.
- Los módulos grandes (>400 líneas) deben dividirse en submódulos con responsabilidades claras.

## Calidad y completitud

- Nunca implementaciones parciales: si una feature se empieza, se termina completa en ese mismo cambio. Está prohibido dejar TODOs o stubs funcionales con la intención de completarlos después.
- Prohibida la deuda técnica deliberada: no se añade código legacy, no se mantienen rutas de compatibilidad innecesarias, no se conservan abstracciones obsoletas.
- El mejor código posible aunque sea más costoso de implementar: se prefiere la solución correcta y eficiente sobre la solución rápida.
- Nunca dejar comentarios `// TODO`, `// FIXME`, `// HACK`, `// STUB` en código que se entrega.
- Si una refactorización es necesaria para implementar algo correctamente, se hace; no se parchea sobre código incorrecto.

## Rendimiento

- Preferir estructuras de datos con complejidad O(1) o O(log n) para operaciones frecuentes.
- Evitar allocations innecesarias en rutas calientes (loops de interpretación, dispatch de opcodes).
- Usar `Arc` sólo cuando el ownership compartido es genuinamente necesario; preferir ownership directo o referencias.
- Los `HashMap` de tamaño fijo conocido deben inicializarse con `HashMap::with_capacity`.

## Migración desde TypeScript

- Antes de implementar cualquier feature en Rust, buscar su implementación previa en `packages/` (la base TypeScript legacy).
- Leer el código TypeScript correspondiente para entender la lógica, los casos borde y las decisiones de diseño originales.
- La migración debe ser 1:1 en semántica: mismos casos cubiertos, mismos errores propagados, mismo comportamiento observable. No se simplifica la lógica por comodidad.
- Si la implementación TypeScript tiene deuda técnica o workarounds conocidos, la versión Rust los corrige; no los replica.
- Los archivos bajo `packages/` son de sólo lectura y referencia; nunca se modifican.

## Visión del lenguaje TSN

TSN es TypeScript rehecho desde cero, sin las ataduras de JavaScript. Cada decisión de diseño debe partir de esta premisa: si JavaScript no existiera, ¿cómo diseñaríamos este lenguaje correctamente?

### Núcleo del sistema de valores

- **No existe `undefined`**. Hay exactamente un concepto de ausencia: `null`. La distinción `null`/`undefined` es deuda histórica de JS; TSN no la replica.
- **`null` no es un objeto**. `typeof null === "null"`. Punto.
- **No hay coerción implícita de tipos**. `"1" + 1` es un error en tiempo de compilación, no `"11"`. La coerción existe sólo cuando es explícita y el programador la solicita.
- **Un solo operador de igualdad**: `==` compara por valor con semántica estricta. No existe `===`. No existe `==` con coerción silenciosa.

### Sistema de tipos primitivos

Los tipos primitivos actuales del lenguaje son:

| Tipo | Semántica |
|---|---|
| `str` | Cadena de texto UTF-8 inmutable |
| `char` | Un único codepoint Unicode |
| `int` | Entero con signo de precisión nativa (64 bits en la implementación actual) |
| `float` | Punto flotante de doble precisión (64 bits) |
| `decimal` | Decimal de precisión exacta, sin errores de representación binaria |
| `bigint` | Entero de precisión arbitraria |
| `bool` | Booleano: exactamente `true` o `false` |
| `symbol` | Valor único e irrepetible, no coercible a ningún otro tipo |

**No existe `number`**. La fusión de enteros y flotantes en un solo tipo es deuda histórica de JS y la fuente de incontables bugs silenciosos. `int` y `float` son tipos distintos. La conversión entre ellos es explícita salvo cuando el compilador puede garantizar que es segura (widening de `int` a `float` en contextos numéricos).

**Hoja de ruta para tipos numéricos de precisión**: cuando el lenguaje alcance fase de optimización de bajo nivel, `int` se subdividirá en `i32`, `i64`, `u32`, `u64` y `float` en `f32`, `f64`. En ese momento `int` y `float` actuarán como aliases del tamaño nativo por defecto. El código existente no cambia de semántica.

### Sistema de tipos compuesto y especial

- **El tipo `any` no existe como escapatoria**. Si el tipo es desconocido en tiempo de compilación se usa `unknown` y debe ser discriminado antes de usarse.
- **Los tipos nullables son explícitos**: `str | null`, nunca implícito. Una variable de tipo `str` jamás puede contener `null` sin que el tipo lo declare.
- **`void` es el tipo de retorno de funciones que no producen valor**, no un pseudo-valor. Una expresión de tipo `void` no puede asignarse.
- **`never` es el tipo vacío real**: una función marcada `never` que retorna produce un error de compilación, no un warning. Es el tipo de ramas imposibles.
- **`this` como tipo es válido en métodos** para expresar polimorfismo de retorno ligado al receptor concreto.
- **Tipos literales son de primera clase**: `42`, `"ok"`, `true` son tipos, no sólo valores. Permiten discriminación sin boxing adicional.
- **Tipos suma (union) y producto (tuple) son nativos**, no emulados. `str | int | null` es un tipo real que el compilador analiza exhaustivamente, no un `any` con comentario.
- **Intersección de tipos** (`A & B`) expresa la satisfacción simultánea de dos contratos, no la mezcla dinámica de propiedades.

### Enums

El estado actual del lenguaje define enums como conjuntos de variantes nombradas con valor asociado opcional. La dirección de evolución es hacia **enums algebraicos** con payload tipado por variante:

```tsn
// Estado actual: variantes con valor escalar
enum Status { Active = 1, Inactive = 2 }

// Objetivo: enums algebraicos con datos por variante
enum Shape {
  Circle(radius: float),
  Rect(width: float, height: float),
  Point,
}
```

Cualquier nueva feature sobre enums debe diseñarse con los algebraicos como destino. No se añaden workarounds que asuman que los enums son sólo números.

### Orientación a objetos

- **Las clases tienen semántica de referencia; los `struct`s tienen semántica de valor**. Un `struct` copiado no comparte estado con el original.
- **No existe cadena de prototipos expuesta**. La herencia es explícita mediante `extends` y el compilador la modela como vtable, no como búsqueda dinámica en `__proto__`.
- **`this` siempre es léxico dentro de métodos de clase**. No existe el problema del `this` perdido en callbacks; una función flecha dentro de un método captura el receptor de la clase.
- **Los campos de clase son privados por defecto**. El modificador `public` debe ser explícito. No existe `#campo` como segundo mecanismo de privacidad; hay uno solo.
- **No existe `arguments`**. Los parámetros rest (`...args: T[]`) son la única forma de capturar argumentos variádicos.

### Control de flujo y expresiones

- **`match` es una expresión**, no una sentencia. Retorna un valor. Los `if/else` también son expresiones.
- **No existe `switch` con fall-through implícito**. `match` requiere arms explícitos y sin fall-through.
- **Las declaraciones de variables son `const` (inmutable), `let` (inmutable ligada), y `var` (mutable)**. La distinción es semántica y verificada por el compilador. El hoisting no existe.
- **Las variables no pueden usarse antes de ser declaradas**. La zona temporal muerta es el comportamiento correcto, no la excepción.
- **El narrowing es exhaustivo**: sobre un tipo suma, el compilador exige que todos los casos estén cubiertos. El comodín `_` existe pero genera un warning si hay variantes enumerables sin cubrir.

### Manejo de errores

- **Las excepciones existen para errores irrecuperables** (panics del runtime). Para errores esperados del dominio, el tipo de retorno es `Result<T, E>`.
- **`try/catch` está reservado para errores de runtime genuinos**, no para control de flujo. El compilador emite un warning si se captura un tipo que podría modelarse como `Result`.
- **El operador `?` propaga `Result` y `null` de forma explícita**, sin magia oculta.

### Módulos y organización

- **Un solo sistema de módulos**. No existe CommonJS ni AMD. `import`/`export` ES-module es la única forma.
- **Las importaciones circulares son un error de compilación**, no un comportamiento indefinido resuelto en runtime.
- **Los módulos de la stdlib tienen el prefijo `std:`** y están tipados de forma nativa; no son polyfills sobre APIs de host.

### Asincronía

- **`async`/`await` son nativos del lenguaje**, no azúcar sobre Promises de JS. El runtime gestiona la cola de tareas sin depender de un event loop de V8.
- **Las funciones `async` retornan `Future<T>`**, no `Promise<T>`. El nombre refleja semántica, no legado.
- **No existe callback hell como patrón idiomático**. Si una API del stdlib requiere callbacks, tiene también su variante `async` de primera clase.

### Lo que TSN no debe replicar nunca de JavaScript

- `typeof null === "object"`
- Variables declaradas con `var` que se elevan al scope de función
- Coerción en comparaciones con `==`
- La dualidad `null`/`undefined`
- El tipo `number` que mezcla enteros y flotantes sin distinción
- Prototype chain accesible en runtime como mecanismo principal de herencia
- `arguments` como objeto implícito en funciones
- `NaN === NaN` siendo `false`
- El objeto global `window`/`global` como namespace implícito
- `with` statement
- Labeled breaks y continues como mecanismo de control de flujo estructurado

Cada vez que una feature nueva se diseñe o se migre desde `packages/`, esta lista es el filtro: si la feature existe sólo por compatibilidad con JS y no aporta expresividad real al lenguaje, no se implementa o se implementa con una semántica corregida.

## Comunicación

- Nunca resumir trabajo realizado con listas largas al final de cada respuesta; confirmar con una frase concisa.
- Las respuestas deben ser directas y orientadas a la acción; omitir preámbulos, conclusiones y repetición de contexto ya conocido.


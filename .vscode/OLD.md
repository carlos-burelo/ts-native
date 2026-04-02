# DECISIÓN 1 — ELIMINAR `Arc` Y `RwLock` DEL MODELO DE OBJETOS

Esto es **la corrección más importante**.

Ahora mismo cada objeto es:

```
Arc<RwLock<ObjData>>
```

Eso introduce:

* atomics
* fences
* contención
* penalización en cada acceso

Las VMs rápidas **no usan locks en objetos**.

### Nueva regla

```
Value
   └── puntero a objeto del heap
```

sin `Arc`
sin `RwLock`.

---

# IMPLEMENTACIÓN

Crear un heap central.

Nuevo archivo:

```
tsn-vm/runtime/heap.rs
```

Estructura:

```rust
pub struct Heap {
    objects: Vec<Box<ObjData>>,
    arrays: Vec<Box<ArrayData>>,
}
```

Cada asignación devuelve un puntero estable:

```rust
pub type ObjRef = *mut ObjData;
pub type ArrayRef = *mut ArrayData;
```

---

### Value cambia a

```
Object(ObjRef)
Array(ArrayRef)
Class(ClassRef)
```

no `Arc`.

---

### Ejemplo

ANTES

```rust
Value::Object(Arc<RwLock<ObjData>>)
```

DESPUÉS

```rust
Value::Object(ObjRef)
```

y el VM accede:

```rust
unsafe { &mut *obj_ptr }
```

Esto elimina **dos capas de sincronización por acceso**.

---

# DECISIÓN 2 — HEAP CONTROLADO POR LA VM

Todos los objetos deben ser creados solo por el VM.

Crear funciones:

```
vm.alloc_object()
vm.alloc_array()
vm.alloc_map()
```

ejemplo:

```rust
pub fn alloc_object(&mut self) -> ObjRef {
    let obj = Box::new(ObjData::new());
    let ptr = Box::into_raw(obj);
    self.heap.objects.push(ptr);
    ptr
}
```

Esto garantiza:

* punteros estables
* control del lifetime

---

# DECISIÓN 3 — GC FUTURO (PREPARAR DESDE AHORA)

No necesitas GC ahora.

Pero debes diseñar el heap para permitirlo.

El heap debe tener:

```
Heap
   objects
   arrays
   maps
   sets
```

y más adelante puedes agregar:

```
mark()
sweep()
```

Si mantienes `Arc`, GC será imposible.

---

# DECISIÓN 4 — OBJECT MODEL SIN LOCKS

`ObjData` debe ser:

```
pub struct ObjData {
    class: Option<ClassRef>,
    slots: Vec<Value>,
    fields: RuntimeObject
}
```

sin `RwLock`.

El VM ya controla la ejecución.

---

# DECISIÓN 5 — SHAPES SIN CLONAR HASHMAP

Tu transición actual:

```
clone property_names
```

Esto escala mal.

Nueva estructura:

```
Shape
   parent
   added_property
   slot
```

Ejemplo:

```rust
pub struct Shape {
    pub id: u32,
    pub parent: Option<Arc<Shape>>,
    pub key: Option<RuntimeString>,
    pub slot: usize,
}
```

El lookup se hace recorriendo la cadena.

Pero el **slot ya está calculado**, así que el acceso sigue siendo O(1).

Esto evita copiar mapas.

---

# DECISIÓN 6 — INLINE CACHE EN OPCODES

Agregar estructura:

```
struct InlineCache {
    shape: u32
    slot: u16
}
```

En bytecode:

```
GetProp
```

se convierte en:

```
GetProp {
   name
   cache
}
```

Ejecución:

```
if obj.shape.id == cache.shape
    fast slot read
else
    slow lookup
```

Esto es lo que hace V8.

---

# DECISIÓN 7 — TAMAÑO DE `Value`

Debes medir:

```
size_of::<Value>()
```

Objetivo ideal:

```
16 bytes
```

Si es mayor de 24 bytes, conviene reorganizar.

Estrategia:

* números inline
* objetos por puntero
* strings por puntero

---

# DECISIÓN 8 — `Vec<Value>` CON CAPACIDAD INICIAL

Cuando creas un objeto:

```
Vec::with_capacity(4)
```

La mayoría de objetos tienen ≤4 propiedades.

Evita reallocaciones.

---

# DECISIÓN 9 — STRING INTERNING

Ahora usas:

```
Arc<str>
```

Esto funciona, pero para propiedades conviene:

```
SymbolId(u32)
```

y un interner global.

Entonces:

```
Shape.property_names
```

usa enteros, no strings.

Eso acelera comparaciones.

---

# DECISIÓN 10 — MAP / SET

No usar `std::HashMap`.

Usar:

```
hashbrown::HashMap
```

porque es la implementación usada por Rust internamente y es más rápida.

---

# DECISIÓN 11 — CLASES SIN `RwLock`

Ahora tienes:

```
Arc<RwLock<ClassObj>>
```

Esto debe cambiar a:

```
ClassRef
```

puntero simple.

Las clases son **inmutables después de definirse**.

Por lo tanto no necesitan locks.

---

# DECISIÓN 12 — BOUND METHOD

Esto:

```
receiver: Box<Value>
```

no es ideal.

Debe ser:

```
receiver: Value
```

Value ya es pequeño.

---

# DECISIÓN 13 — FUTURES SIN ARC

Si `AsyncFuture` usa `Arc`, revisarlo.

Lo ideal es:

```
FutureRef
```

puntero a heap.

---

# ARQUITECTURA FINAL DEL VALUE

El Value debería quedar conceptualmente así:

```
Value
 ├─ Null
 ├─ Bool
 ├─ Int
 ├─ Float
 ├─ Str(StrRef)
 ├─ Object(ObjRef)
 ├─ Array(ArrayRef)
 ├─ Map(MapRef)
 ├─ Set(SetRef)
 ├─ Closure(ClosureRef)
 ├─ Class(ClassRef)
 ├─ Future(FutureRef)
 ├─ Generator(GeneratorRef)
```

Todo lo complejo vive en el heap.

---

# REFACTOR QUE DEBES HACER

Orden recomendado.

### Paso 1

Eliminar `RwLock`.

### Paso 2

Eliminar `Arc`.

### Paso 3

Crear `Heap`.

### Paso 4

Cambiar `Value` a punteros.

### Paso 5

Actualizar todas las operaciones de objeto.

### Paso 6

Implementar `alloc_*`.

### Paso 7

Actualizar arrays / maps / sets.

---

# RESULTADO

El runtime pasa de:

```
obj.prop
= Arc + lock + lookup
```

a

```
obj.prop
= pointer + slot
```

Eso puede ser **10-50x más rápido**.

---

# La decisión filosófica final

Debes elegir uno de estos dos modelos.

### MODELO A (JS)

```
single-thread VM
GC heap
no locks
```


Los runtimes rápidos usan **A**.

Incluso Go, Erlang, Node y Python usan VM single-thread por instancia.

Puedes tener **muchos VMs**, pero cada uno sin locks internos.

---

Mi recomendación fuerte:

**elige el modelo A.**

VM single-thread
heap propio
sin locks.

Luego puedes ejecutar **muchos VMs en paralelo**.

---
Claude’s Plan
Plan: Runtime Cleanup — Thread-contained single-thread model
Context
El runtime actual está contaminado con primitivas de multithreading (Arc, Mutex, Waker, AtomicUsize, unsafe impl Send) en componentes que NUNCA se ejecutan en múltiples hilos simultáneamente. El resultado:

TaskRegistry con Mutex donde sólo un hilo escribe
Waker/WakerVtable abstracting algo que el reactor puede hacer directamente
block_until_ready que DEADLOCK en single-thread event loop
unsafe impl Send for ReadyTask — lie compilatoria
GeneratorDriver: Send + Sync cuando los drivers contienen Box<Vm> que es !Send
El objetivo es separar claramente: núcleo determinista (VM, Scheduler, TaskRegistry, Futures) vs sistema (Reactor, IO, timers). El reactor corre en su propio hilo pero sólo produce eventos — nunca toca el TaskRegistry directamente.

Arquitectura resultante

Thread principal:
  Scheduler
  ├── TaskRegistry (owned, plain struct, no Arc/Mutex)
  ├── event_queue: Arc<Mutex<VecDeque<ExternalEvent>>> ←─ lee
  ├── WAKE_QUEUE (thread-local, para generator wakes)
  └── VM executions

Thread reactor (separado):
  └── event_queue ──────────────────────────────────────→ push(WakeTask)
Cambios por archivo
1. crates/tsn-types/src/future.rs
Eliminar Waker, WakerVtable (struct + trait)
Eliminar block_until_ready (deadlock en single-thread)
Eliminar register_waker
Mantener AsyncFuture como Arc<Mutex<Inner>> (necesario para cross-thread settlement en future_delayed y AsyncQueue)
SettleCallback mantiene Send + 'static (AsyncQueue puede ser settled desde otro hilo)
2. crates/tsn-types/src/lib.rs
Eliminar Waker, WakerVtable de los re-exports
3. crates/tsn-types/src/generator.rs
GeneratorDriver trait: eliminar supertraits Send + Sync
GeneratorObj: cambiar Arc<dyn GeneratorDriver> → Rc<dyn GeneratorDriver>
GenChannel: Mutex<Option<AsyncFuture>> → RefCell<Option<AsyncFuture>>, AtomicBool → Cell<bool> (para done y started)
GenChannel::new(): retorna Rc<Self> en vez de Arc<Self>
4. crates/tsn-vm/src/runtime/waker.rs
Vaciar completamente (dejar archivo vacío para no romper git), o eliminar referencia en mod.rs
5. crates/tsn-vm/src/runtime/task.rs
Eliminar Arc, parking_lot::Mutex, AtomicUsize de TaskRegistry
TaskRegistry: struct plain con suspended: HashMap<TaskId, SuspendedTask>, queue: VecDeque<ReadyTask>, active: usize
TaskRegistry::new() → retorna Self (ya no Arc<Self>)
Todos los métodos toman &mut self
Eliminar unsafe impl Send for ReadyTask
Añadir thread-local:

thread_local! {
    static WAKE_QUEUE: RefCell<VecDeque<(TaskId, Option<Result<Value,Value>>)>> = ...
}
pub fn schedule_wake_sync(task_id: TaskId, resume: Option<Result<Value,Value>>)
pub(super) fn drain_sync_wake_queue() -> Vec<(TaskId, Option<Result<Value,Value>>)>
Usado por AsyncGenDriver::next() (siempre desde el scheduler thread)
6. crates/tsn-vm/src/runtime/reactor.rs
Definir:

pub struct WakeResult(pub Option<Result<Value, Value>>);
unsafe impl Send for WakeResult {} // Value se origina y termina en scheduler thread
pub enum ExternalEvent {
    WakeTask(TaskId),                        // timer: resume con Value::Null
    WakeTaskWithResult(TaskId, WakeResult),  // future await: resume con result
}
TimerEntry: reemplazar waker: Waker con task_id: TaskId, event_queue: Arc<Mutex<VecDeque<ExternalEvent>>>
IoEntry: igual
TimerWheel::schedule(duration, task_id, event_queue): sin Waker
Reactor::sleep(duration, task_id, event_queue): sin Waker
Reactor::register_io(source, interest, task_id, event_queue): sin Waker
reactor_loop: event_queue.lock().push_back(ExternalEvent::WakeTask(task_id)) en vez de waker.wake()
7. crates/tsn-vm/src/runtime/scheduler.rs
Scheduler:

pub struct Scheduler {
    registry: TaskRegistry,                           // owned, no Arc
    reactor: Reactor,
    event_queue: Arc<Mutex<VecDeque<ExternalEvent>>>, // shared with reactor
}
Eliminar Arc<TaskRegistry>, TaskWaker, imports de waker
Añadir TaskStrategy stub:

pub enum TaskStrategy { Local, Parallel }
Scheduler::new(): crea event_queue, TaskRegistry::new(), Reactor::spawn(Arc::clone(&event_queue))
spawn_root(): ya NO setea vm.task_registry
run() loop:
Drain event_queue → registry.wake(id, resume) para cada evento
Drain task::drain_sync_wake_queue() → registry.wake(id, resume) para cada
Si hay task: run_task(task, &mut registry, &reactor, &event_queue)
Si no hay task + all_done(): break
Idle: thread::sleep(Duration::from_micros(100))
run_task() (free fn): parámetros (ReadyTask, &mut TaskRegistry, &Reactor, &Arc<...>)
VmSuspend::Future(fut): registry.suspend(...), luego:

let eq = Arc::clone(event_queue);
fut.on_settle(move |result| {
    eq.lock().unwrap().push_back(ExternalEvent::WakeTaskWithResult(task_id, WakeResult(Some(result))));
});
VmSuspend::Timer(duration): registry.suspend(...), reactor.sleep(duration, task_id, Arc::clone(event_queue))
VmSuspend::Yield(val): sin cambio
enqueue_spawns(): toma &mut TaskRegistry (ya no Arc)
Añadir enqueue_async_gen_spawns(vm, &mut registry): drena pending_async_gen_spawns → inserta en registry.suspended, incrementa registry.active
Reactor::spawn(): ahora recibe Arc<Mutex<VecDeque<ExternalEvent>>> como parámetro
8. crates/tsn-vm/src/runtime/mod.rs
Eliminar pub(super) mod waker
Mantener pub use task::{TaskId, TaskRegistry}
9. crates/tsn-vm/src/vm/mod.rs
Eliminar campo task_registry: Option<Arc<TaskRegistry>>
Añadir campo pending_async_gen_spawns: Vec<(TaskId, Box<Vm>)>
Importar use crate::runtime::task::TaskId
Actualizar Vm::new() y new_with_globals()
Añadir pub fn take_pending_async_gen_spawns(&mut self) -> Vec<(TaskId, Box<Vm>)>
10. crates/tsn-vm/src/vm/call.rs
Eliminar uso de self.task_registry
Async generator branch:

let task_id = crate::runtime::task::alloc_task_id();
// crear gen_channel, task_vm
let driver = super::generator::AsyncGenDriver::new(gen_channel, task_id);
self.pending_async_gen_spawns.push((task_id, task_vm));
self.push(Value::Generator(...));
11. crates/tsn-vm/src/vm/generator.rs
SyncGenDriver: Mutex<SyncGenInner> → RefCell<SyncGenInner>, next() usa borrow_mut()
SyncGenDriver::new() → retorna Rc<Self>
AsyncGenDriver: eliminar campo registry: Arc<TaskRegistry>, mantener gen_channel: Rc<GenChannel> y task_id
AsyncGenDriver::new(): ya no toma registry, retorna Rc<Self>
AsyncGenDriver::next(): crate::runtime::task::schedule_wake_sync(self.task_id, resume) en vez de self.registry.wake(...)
AsyncGenDriver::drop(): schedule_wake_sync(self.task_id, Some(Err(...)))
ArrayGenDriver en stream.rs: Arc<RwLock<Vec<Value>>> → RefCell<Vec<Value>>, AtomicUsize → Cell<usize>
12. crates/tsn-modules/src/builtins/complex/future.rs
future_all, future_race, future_all_settled, future_any: reescribir como combinadores async usando on_settle callbacks en cadena (ya no block_until_ready)
Ejemplo future_all: crea output future, para cada sub-future registra on_settle que decrementa contador compartido; cuando llega a 0 resuelve el output.
Usar Arc<Mutex<Vec<Value>>> + Arc<AtomicUsize> para estado compartido entre callbacks Send + 'static
13. crates/tsn-vm/src/vm/props/future.rs
future_unwrap: reemplazar block_until_ready → chequear peek_state(), error si Pending
future_expect: igual
14. crates/tsn-modules/src/stdlib/stream.rs
stream_collect: eliminar block_until_ready, sólo soportar sync generators, error si async
ArrayGenDriver: Arc<RwLock<...>> → RefCell<Vec<Value>>, AtomicUsize → Cell<usize>
Orden de implementación
tsn-types/src/future.rs — eliminar Waker/block_until_ready
tsn-types/src/lib.rs — actualizar re-exports
tsn-types/src/generator.rs — Rc + RefCell + Cell
tsn-vm/src/runtime/task.rs — TaskRegistry plain + wake queue TLS
tsn-vm/src/runtime/reactor.rs — ExternalEvent + sin Waker
tsn-vm/src/runtime/waker.rs — vaciar
tsn-vm/src/runtime/mod.rs — eliminar waker module
tsn-vm/src/vm/mod.rs — eliminar task_registry, añadir pending_async_gen_spawns
tsn-vm/src/vm/generator.rs — Rc/RefCell, schedule_wake_sync
tsn-vm/src/vm/call.rs — async gen sin registry
tsn-vm/src/runtime/scheduler.rs — loop nuevo con event_queue
tsn-modules/src/builtins/complex/future.rs — combinadores async
tsn-vm/src/vm/props/future.rs — unwrap/expect sin block
tsn-modules/src/stdlib/stream.rs — collect sin block
Verificación

cargo build --workspace
cargo run --bin tsn -- examples/for-await-test.tsn
cargo run --bin tsn -- examples/ultimate-test.tsn
cargo run --bin tsn -- examples/generic-test.tsn
Verificar que:

async/await sigue funcionando
for await ... of con async generators funciona
timers (await sleep(n)) funcionan
Future.all([...]) resuelve correctamente (ahora async, no blocking)
No hay Arc/Mutex en TaskRegistry
No hay unsafe impl Send en ningún lugar del scheduler/task
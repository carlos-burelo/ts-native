use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicUsize, Ordering};

use tsn_types::future::AsyncFuture;
use tsn_types::Value;

use crate::vm::Vm;

pub type TaskId = usize;

static NEXT_TASK_ID: AtomicUsize = AtomicUsize::new(1);

pub fn alloc_task_id() -> TaskId {
    NEXT_TASK_ID.fetch_add(1, Ordering::Relaxed)
}

thread_local! {
    static WAKE_QUEUE: RefCell<VecDeque<(TaskId, Option<Result<Value, Value>>)>> =
        RefCell::new(VecDeque::new());
}

pub fn schedule_wake_sync(task_id: TaskId, resume: Option<Result<Value, Value>>) {
    WAKE_QUEUE.with(|q| q.borrow_mut().push_back((task_id, resume)));
}

pub(super) fn drain_sync_wake_queue() -> Vec<(TaskId, Option<Result<Value, Value>>)> {
    WAKE_QUEUE.with(|q| q.borrow_mut().drain(..).collect())
}

pub struct ReadyTask {
    pub id: TaskId,
    pub vm: Box<Vm>,
    pub output: AsyncFuture,
    pub resume: Option<Result<Value, Value>>,
}

pub(super) struct SuspendedTask {
    pub vm: Box<Vm>,
    pub output: AsyncFuture,
}

pub struct TaskRegistry {
    pub(super) suspended: HashMap<TaskId, SuspendedTask>,
    pub(super) queue: VecDeque<ReadyTask>,
    pub(super) active: usize,
}

impl TaskRegistry {
    pub fn new() -> Self {
        TaskRegistry {
            suspended: HashMap::new(),
            queue: VecDeque::new(),
            active: 0,
        }
    }

    pub fn spawn(&mut self, vm: Box<Vm>, output: AsyncFuture) -> TaskId {
        let id = alloc_task_id();
        self.active += 1;
        self.queue.push_back(ReadyTask {
            id,
            vm,
            output,
            resume: None,
        });
        id
    }

    pub fn suspend(&mut self, id: TaskId, vm: Box<Vm>, output: AsyncFuture) {
        self.suspended.insert(id, SuspendedTask { vm, output });
    }

    pub fn wake(&mut self, id: TaskId, resume: Option<Result<Value, Value>>) {
        if let Some(t) = self.suspended.remove(&id) {
            self.queue.push_back(ReadyTask {
                id,
                vm: t.vm,
                output: t.output,
                resume,
            });
        }
    }

    pub fn finish(&mut self) {
        self.active -= 1;
    }

    pub fn all_done(&self) -> bool {
        self.active == 0
    }
}

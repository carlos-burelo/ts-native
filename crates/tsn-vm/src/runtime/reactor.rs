use std::cmp::Reverse;
use std::collections::{BinaryHeap, VecDeque};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use mio::event::Source;
use mio::{Events, Interest, Poll, Token};

use super::task::TaskId;

pub struct WakeResult(pub Option<Result<tsn_types::Value, tsn_types::Value>>);
unsafe impl Send for WakeResult {}

pub struct CallbackTask {
    pub cb: tsn_types::Value,
    pub arg: Result<tsn_types::Value, tsn_types::Value>,
    pub output: tsn_types::future::AsyncFuture,
    pub kind: CallbackKind,
}
unsafe impl Send for CallbackTask {}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CallbackKind {
    Then,

    Catch,

    Finally,
}

pub enum ExternalEvent {
    WakeTask(TaskId),

    WakeTaskWithResult(TaskId, WakeResult),

    RunCallback(CallbackTask),
}

pub type EventQueue = Arc<Mutex<VecDeque<ExternalEvent>>>;

struct TimerEntry {
    deadline: Instant,
    task_id: TaskId,
    event_queue: EventQueue,
}

impl PartialEq for TimerEntry {
    fn eq(&self, other: &Self) -> bool {
        self.deadline == other.deadline
    }
}
impl Eq for TimerEntry {}
impl Ord for TimerEntry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other.deadline.cmp(&self.deadline)
    }
}
impl PartialOrd for TimerEntry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Clone)]
pub struct TimerWheel(Arc<Mutex<BinaryHeap<Reverse<TimerEntry>>>>);

impl TimerWheel {
    fn new() -> Self {
        TimerWheel(Arc::new(Mutex::new(BinaryHeap::new())))
    }

    pub fn schedule(&self, duration: Duration, task_id: TaskId, event_queue: EventQueue) {
        let deadline = Instant::now() + duration;
        self.0.lock().unwrap().push(Reverse(TimerEntry {
            deadline,
            task_id,
            event_queue,
        }));
    }

    fn drain_expired(&self, now: Instant) -> Vec<TimerEntry> {
        let mut heap = self.0.lock().unwrap();
        let mut expired = Vec::new();
        while let Some(top) = heap.peek() {
            if top.0.deadline <= now {
                expired.push(heap.pop().unwrap().0);
            } else {
                break;
            }
        }
        expired
    }

    fn time_to_next(&self, now: Instant) -> Option<Duration> {
        self.0
            .lock()
            .unwrap()
            .peek()
            .map(|top| top.0.deadline.saturating_duration_since(now))
    }
}

struct IoEntry {
    task_id: TaskId,
    event_queue: EventQueue,
}

struct IoRegistry {
    entries: std::collections::HashMap<Token, IoEntry>,
    next_token: usize,
}

impl IoRegistry {
    fn new() -> Self {
        IoRegistry {
            entries: std::collections::HashMap::new(),
            next_token: 1,
        }
    }

    fn alloc_token(&mut self) -> Token {
        let t = Token(self.next_token);
        self.next_token += 1;
        t
    }
}

const MAX_POLL_TIMEOUT: Duration = Duration::from_millis(10);

#[derive(Clone)]
pub struct Reactor {
    pub timers: TimerWheel,
    nudge: Arc<mio::Waker>,
    io_registry: Arc<Mutex<IoRegistry>>,
    poll: Arc<Mutex<Poll>>,
}

impl Reactor {
    pub fn spawn(event_queue: EventQueue) -> Self {
        let poll = Poll::new().expect("mio::Poll::new failed");
        let nudge = Arc::new(
            mio::Waker::new(poll.registry(), Token(0)).expect("mio::Waker creation failed"),
        );
        let timers = TimerWheel::new();
        let io_registry = Arc::new(Mutex::new(IoRegistry::new()));
        let poll_arc = Arc::new(Mutex::new(poll));

        let reactor = Reactor {
            timers: timers.clone(),
            nudge,
            io_registry: io_registry.clone(),
            poll: poll_arc.clone(),
        };

        {
            let timers = timers.clone();
            let io_reg = io_registry.clone();
            let poll_arc2 = poll_arc.clone();
            let _eq = event_queue;

            thread::Builder::new()
                .name("tsn-reactor".into())
                .spawn(move || reactor_loop(timers, io_reg, poll_arc2))
                .expect("failed to spawn reactor thread");
        }

        reactor
    }

    pub fn sleep(&self, duration: Duration, task_id: TaskId, event_queue: EventQueue) {
        self.timers.schedule(duration, task_id, event_queue);
        let _ = self.nudge.wake();
    }

    pub fn register_io<S: Source>(
        &self,
        source: &mut S,
        interest: Interest,
        task_id: TaskId,
        event_queue: EventQueue,
    ) -> Token {
        let mut reg = self.io_registry.lock().unwrap();
        let token = reg.alloc_token();
        reg.entries.insert(
            token,
            IoEntry {
                task_id,
                event_queue,
            },
        );
        self.poll
            .lock()
            .unwrap()
            .registry()
            .register(source, token, interest)
            .expect("mio register failed");
        let _ = self.nudge.wake();
        token
    }

    pub fn deregister_io<S: Source>(&self, source: &mut S, token: Token) {
        self.io_registry.lock().unwrap().entries.remove(&token);
        let _ = self.poll.lock().unwrap().registry().deregister(source);
    }
}

fn reactor_loop(timers: TimerWheel, io_registry: Arc<Mutex<IoRegistry>>, poll: Arc<Mutex<Poll>>) {
    let mut events = Events::with_capacity(256);
    loop {
        let now = Instant::now();

        let timeout = timers
            .time_to_next(now)
            .map(|d| d.min(MAX_POLL_TIMEOUT))
            .unwrap_or(MAX_POLL_TIMEOUT);

        {
            let mut p = poll.lock().unwrap();
            let _ = p.poll(&mut events, Some(timeout));
        }

        let now = Instant::now();

        for event in events.iter() {
            let token = event.token();
            if token == Token(0) {
                continue;
            }
            if let Some(entry) = io_registry.lock().unwrap().entries.remove(&token) {
                entry
                    .event_queue
                    .lock()
                    .unwrap()
                    .push_back(ExternalEvent::WakeTask(entry.task_id));
            }
        }
        events.clear();

        for entry in timers.drain_expired(now) {
            entry
                .event_queue
                .lock()
                .unwrap()
                .push_back(ExternalEvent::WakeTask(entry.task_id));
        }
    }
}

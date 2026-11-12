//! Native multicore workers (multicore.md D1/D8/D12, native.md N5/N7/N8/N10/N12).
//!
//! One module, N instances. The instance of the entry is worker 0; its i
//! scheduler is the one authority of the run's shared state (N8's one lock:
//! scopes, joins, channels, shares, cancellation). Workers 1..N-1 each have
//! their own `Store` on their own OS thread, with the same host functions.
//!
//! At `go` the scheduler encodes a portable task record into one shared
//! queue (`mc_publish`, N7). A record is claimed once: by the maker
//! (`mc_claim`), which runs its retained original, or by a worker, which
//! decodes it and runs the task there to its next step (`ilang_mc_begin`).
//! Each step goes to worker 0 as a message (`mc_inbox`, `mc_message`); the
//! answer goes back to the worker that holds the task's stack (`mc_send`,
//! `ilang_mc_resume`). A started task never moves (N6). Values that cannot
//! move travel as tokens of their worker; a record that holds tokens of a
//! worker is published for that worker only.
//!
//! The worker count comes from `I_WORKERS`, else from the available cores.
//! One worker keeps the FIFO fast path: no queue, no encoding (N10).

use std::{
    collections::{HashMap, VecDeque},
    sync::{
        Arc, Condvar, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread::JoinHandle,
    time::Duration,
};

use anyhow::{Context, Result, anyhow, ensure};
use wasmtime::{Caller, Engine, Linker, Module, Store, UpdateDeadline};

use crate::{Abi, Bridge, HOST_MODULE, HostOptions, ResourceContext, State, THREAD_STACK};

/// The number of workers of an entry run: `I_WORKERS=<n>` asks for n;
/// unset or `auto` gives the available cores, at most one per GiB of
/// available memory.
pub fn worker_count() -> usize {
    match std::env::var("I_WORKERS") {
        Err(_) => auto_workers(),
        Ok(text) if text.trim() == "auto" => auto_workers(),
        Ok(text) => match text.trim().parse::<usize>() {
            Ok(n) if n >= 1 => n.min(255),
            _ => panic!("I_WORKERS must be `auto` or a positive number of workers, not {text:?}"),
        },
    }
}

/// The available cores, at most one worker per WORKER_MEMORY of available
/// memory (each worker is an instance with its own GC heap), at least one.
fn auto_workers() -> usize {
    let cores = std::thread::available_parallelism().map_or(1, |n| n.get().min(255));
    match available_memory() {
        Some(bytes) => cores.min((bytes / WORKER_MEMORY).max(1) as usize),
        None => cores,
    }
}

/// The memory a default worker count reserves for each worker.
const WORKER_MEMORY: u64 = 1 << 30;

/// `MemAvailable` of Linux, in bytes.
fn available_memory() -> Option<u64> {
    let text = std::fs::read_to_string("/proc/meminfo").ok()?;
    let line = text
        .lines()
        .find(|line| line.starts_with("MemAvailable:"))?;
    let kib: u64 = line.split_whitespace().nth(1)?.parse().ok()?;
    Some(kib * 1024)
}

/// The key id prefix of a worker (D6): worker w takes ids above
/// `(w + 1) << 56`, and a new instance of the same worker a new generation.
pub(crate) fn id_prefix(worker: usize, generation: u64) -> i64 {
    (((worker as u64 + 1) << 56) | ((generation & 0xffff) << 40)) as i64
}

type ShareKey = (i64, i64, i64);

/// A published record: its bytes, and the worker its tokens bind it to.
struct Record {
    bytes: Vec<u8>,
    affinity: Option<usize>,
}

/// A task that a worker other than 0 claimed.
struct Away {
    worker: usize,
    slot: Option<i32>,
    serial: Option<i64>,
    /// Its last message ends it.
    ended: bool,
    /// Messages to worker 0 not yet taken (at most one outstanding step).
    messages: VecDeque<Vec<u8>>,
}

#[derive(Default)]
struct PoolState {
    queue: VecDeque<u64>,
    ready: HashMap<u64, Record>,
    away: HashMap<u64, Away>,
    by_serial: HashMap<i64, u64>,
    /// Records worker 0 started as tasks away from it: record → task serial.
    adopted: HashMap<u64, i64>,
    /// Workers that have their instance.
    running: usize,
    /// Records adopted by worker 0 with a message waiting.
    inbox: VecDeque<u64>,
    /// The message worker 0 took last (`mc_message`).
    current: Option<Vec<u8>>,
    /// Answers for each worker: (record, bytes).
    answers: Vec<VecDeque<(u64, Vec<u8>)>>,
    next: u64,
    threads: Vec<JoinHandle<()>>,
    spawned: usize,
    shutdown: bool,
    broken: bool,
    /// The first stop of a task on a worker (N12).
    stop: Option<String>,
    /// Steps run per worker: index 0 is worker 0.
    ran: Vec<u64>,
    /// Tasks started per worker.
    started: Vec<u64>,
    /// Tasks a worker holds now.
    live: Vec<usize>,
    published: u64,
    shares: HashMap<ShareKey, Arc<Vec<u8>>>,
}

impl PoolState {
    /// Workers that hold no task now (also those still starting).
    fn empty(&self) -> usize {
        (1..=self.spawned).filter(|w| self.live[*w] == 0).count()
    }
}

pub(crate) struct Pool {
    state: Mutex<PoolState>,
    results: Condvar,
    engine: Engine,
    module: Module,
    abi: Abi,
    options: HostOptions,
    /// The resource context of the live invocation, shared by all workers (N9.1).
    resources: Mutex<Option<ResourceContext>>,
    remote: usize,
    abandon: Arc<AtomicBool>,
    ticker: Mutex<Option<JoinHandle<()>>>,
    /// Wakes worker w (index w - 1) for a job, an answer or the end.
    wake: Vec<Arc<tokio::sync::Notify>>,
}

impl Pool {
    pub(crate) fn new(
        engine: Engine,
        module: Module,
        workers: usize,
        abi: Abi,
        options: HostOptions,
    ) -> Arc<Self> {
        Arc::new(Self {
            state: Mutex::new(PoolState {
                next: 1,
                ran: vec![0; workers],
                started: vec![0; workers],
                live: vec![0; workers],
                answers: (0..workers).map(|_| VecDeque::new()).collect(),
                ..PoolState::default()
            }),
            results: Condvar::new(),
            engine,
            module,
            abi,
            options,
            resources: Mutex::new(None),
            remote: workers - 1,
            abandon: Arc::new(AtomicBool::new(false)),
            ticker: Mutex::new(None),
            wake: (1..workers.max(2))
                .map(|_| Arc::new(tokio::sync::Notify::new()))
                .collect(),
        })
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, PoolState> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// The epoch ticker lets shutdown stop a worker that computes (N12).
    fn ensure_ticker(self: &Arc<Self>) {
        let mut ticker = self.ticker.lock().unwrap_or_else(|e| e.into_inner());
        if ticker.is_some() {
            return;
        }
        let pool = Arc::downgrade(self);
        let engine = self.engine.clone();
        *ticker = Some(
            std::thread::Builder::new()
                .name("ilang-epoch".into())
                .spawn(move || {
                    loop {
                        std::thread::sleep(Duration::from_millis(1));
                        let Some(pool) = pool.upgrade() else { return };
                        if pool.lock().shutdown {
                            return;
                        }
                        engine.increment_epoch();
                    }
                })
                .expect("a thread for the epoch ticker"),
        );
    }

    fn spawn_worker(self: &Arc<Self>, state: &mut PoolState) {
        if state.spawned >= self.remote {
            return;
        }
        state.spawned += 1;
        let index = state.spawned;
        let pool = Arc::clone(self);
        if let Ok(thread) = std::thread::Builder::new()
            .name(format!("ilang-worker-{index}"))
            .stack_size(THREAD_STACK)
            .spawn(move || pool.worker(index))
        {
            state.threads.push(thread);
        }
    }

    pub(crate) fn publish(
        self: &Arc<Self>,
        bytes: Vec<u8>,
        affinity: i32,
        resources: &ResourceContext,
    ) -> i64 {
        *self.resources.lock().unwrap_or_else(|e| e.into_inner()) = Some(resources.clone());
        let mut state = self.lock();
        if state.broken || state.shutdown || state.stop.is_some() {
            return 0;
        }
        let affinity = usize::try_from(affinity).ok();
        // Tokens of worker 0 itself never reach a published record.
        if affinity == Some(0) {
            return 0;
        }
        let id = state.next;
        state.next += 1;
        state.published += 1;
        state.ready.insert(id, Record { bytes, affinity });
        state.queue.push_back(id);
        // A new worker when every worker holds tasks (Go starts a spinning M
        // when no P is idle).
        if state.empty() < state.queue.len() {
            self.spawn_worker(&mut state);
        }
        drop(state);
        self.ensure_ticker();
        self.wake_all();
        id as i64
    }

    fn wake_all(&self) {
        for wake in &self.wake {
            wake.notify_one();
        }
    }

    /// The maker takes its own record when no worker has claimed it, no
    /// worker's token binds it elsewhere, and no other worker is free for it
    /// (Go's runnext stays with its P only when no P spins).
    pub(crate) fn claim(&self, id: u64) -> bool {
        let mut state = self.lock();
        let free = state.empty();
        match state.ready.get(&id) {
            Some(record) if record.affinity.is_none() && free == 0 => {
                state.ready.remove(&id);
                if let Some(at) = state.queue.iter().position(|x| *x == id) {
                    state.queue.remove(at);
                }
                state.started[0] += 1;
                true
            }
            Some(_) => false,
            None => !state.away.contains_key(&id),
        }
    }

    /// Worker 0 starts the task of a record a worker claimed: from now on
    /// its messages are delivered.
    pub(crate) fn adopt(&self, id: u64, serial: i64) {
        let mut state = self.lock();
        state.by_serial.insert(serial, id);
        state.adopted.insert(id, serial);
        if let Some(away) = state.away.get_mut(&id) {
            away.serial = Some(serial);
            if !away.messages.is_empty() {
                state.inbox.push_back(id);
            }
        }
    }

    /// The task serial of the next message, or 0.
    pub(crate) fn inbox(&self) -> Result<i64, String> {
        let mut state = self.lock();
        if let Some(stop) = &state.stop {
            return Err(stop.clone());
        }
        let Some(id) = state.inbox.pop_front() else {
            return Ok(0);
        };
        let away = state.away.get_mut(&id).expect("an inbox record is away");
        let message = away
            .messages
            .pop_front()
            .expect("an inbox record has a message");
        let serial = away.serial.expect("an inbox record is adopted");
        if !away.messages.is_empty() {
            state.inbox.push_back(id);
        } else if away.ended {
            state.away.remove(&id);
            state.by_serial.remove(&serial);
            state.adopted.remove(&id);
        }
        state.current = Some(message);
        Ok(serial)
    }

    pub(crate) fn message(&self) -> Vec<u8> {
        self.lock()
            .current
            .take()
            .expect("mc_message after mc_inbox")
    }

    /// Whether a task runs on another worker: worker 0 then waits for its
    /// message instead of a deadlock.
    pub(crate) fn pending(&self) -> Result<bool, String> {
        let state = self.lock();
        if let Some(stop) = &state.stop {
            return Err(stop.clone());
        }
        Ok(!state.inbox.is_empty() || !state.adopted.is_empty())
    }

    pub(crate) fn wait(&self) {
        let state = self.lock();
        if state.inbox.is_empty() && state.stop.is_none() {
            let _ = self
                .results
                .wait_timeout(state, Duration::from_millis(2))
                .unwrap_or_else(|e| e.into_inner());
        }
    }

    /// The answer to the step of a task on another worker.
    pub(crate) fn send(&self, serial: i64, bytes: Vec<u8>) {
        let mut state = self.lock();
        let id = *state
            .by_serial
            .get(&serial)
            .unwrap_or_else(|| panic!("mc_send to task {serial}, which is not away"));
        let worker = state.away[&id].worker;
        state.answers[worker].push_back((id, bytes));
        drop(state);
        self.wake[worker - 1].notify_one();
    }

    pub(crate) fn share_put(&self, key: ShareKey, bytes: Vec<u8>) {
        self.lock().shares.insert(key, Arc::new(bytes));
    }

    pub(crate) fn share_drop(&self, key: ShareKey) {
        self.lock().shares.remove(&key);
    }

    fn share_get(&self, key: ShareKey) -> Option<Arc<Vec<u8>>> {
        self.lock().shares.get(&key).cloned()
    }

    fn stopped(&self, message: String) {
        let mut state = self.lock();
        state.stop.get_or_insert(message);
        drop(state);
        self.results.notify_all();
    }

    /// Stop every worker and join the threads.
    pub(crate) fn shutdown(&self) {
        let threads = {
            let mut state = self.lock();
            if state.shutdown {
                return;
            }
            state.shutdown = true;
            self.abandon.store(true, Ordering::SeqCst);
            if std::env::var_os("I_WORKERS_TRACE").is_some() && state.published > 0 {
                eprintln!(
                    "i: workers {} published {} started {:?} steps {:?}",
                    self.remote + 1,
                    state.published,
                    state.started,
                    state.ran
                );
            }
            std::mem::take(&mut state.threads)
        };
        self.wake_all();
        self.results.notify_all();
        self.engine.increment_epoch();
        for thread in threads {
            let _ = thread.join();
        }
        if let Some(ticker) = self.ticker.lock().unwrap_or_else(|e| e.into_inner()).take() {
            let _ = ticker.join();
        }
    }

    fn worker(self: Arc<Self>, index: usize) {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("a tokio runtime for a worker");
        let mut worker = match Worker::new(&self, index) {
            Ok(worker) => worker,
            Err(_) => {
                // A worker that cannot start (its start-up calls the host,
                // D7): the maker keeps every record it can claim.
                self.lock().broken = true;
                return;
            }
        };
        self.lock().running += 1;
        let wake = Arc::clone(&self.wake[index - 1]);
        let mut slots: HashMap<i32, u64> = HashMap::new();
        runtime.block_on(async {
            loop {
                enum Job {
                    Begin(u64, Vec<u8>),
                    Resume(u64, Vec<u8>),
                }
                let job = {
                    let mut state = self.lock();
                    if state.shutdown || state.stop.is_some() {
                        return;
                    }
                    if let Some((id, bytes)) = state.answers[index].pop_front() {
                        Some(Job::Resume(id, bytes))
                    } else if let Some(at) = state.queue.iter().position(|id| {
                        let affinity = state.ready[id].affinity;
                        // A worker that holds tasks leaves new ones to idle
                        // workers (tokio and Go hand work to idle Ps first).
                        let free = state.empty();
                        affinity == Some(index)
                            || affinity.is_none() && (state.live[index] == 0 || free == 0)
                    }) {
                        let id = state.queue.remove(at).expect("a queued record");
                        let record = state.ready.remove(&id).expect("a ready record");
                        let serial = state.adopted.get(&id).copied();
                        state.away.insert(
                            id,
                            Away {
                                worker: index,
                                slot: None,
                                serial,
                                ended: false,
                                messages: VecDeque::new(),
                            },
                        );
                        state.started[index] += 1;
                        state.live[index] += 1;
                        if std::env::var_os("I_WORKERS_TRACE").is_some() {
                            eprintln!("i: worker {index} started a task");
                        }
                        Some(Job::Begin(id, record.bytes))
                    } else {
                        None
                    }
                };
                let Some(job) = job else {
                    // Wait for a job, an answer, or a request of a task here.
                    let notified = wake.notified();
                    if worker.store.data().requests.is_empty() {
                        notified.await;
                        continue;
                    }
                    let picked = {
                        let next = Box::pin(worker.store.data_mut().requests.next());
                        match futures_util::future::select(Box::pin(notified), next).await {
                            futures_util::future::Either::Left(_) => None,
                            futures_util::future::Either::Right((done, _)) => Some(done),
                        }
                    };
                    match picked {
                        None => continue,
                        Some(done) => {
                            let Some((slot_id, request, reply)) = done else {
                                continue;
                            };
                            let woken = worker.settle(slot_id, &request, reply);
                            match woken {
                                Ok(woken) => {
                                    let mut state = self.lock();
                                    for (slot, message) in woken {
                                        let id = slots[&slot];
                                        let away = state
                                            .away
                                            .get_mut(&id)
                                            .expect("a waiting task is away");
                                        away.messages.push_back(message);
                                        if away.serial.is_some() && away.messages.len() == 1 {
                                            state.inbox.push_back(id);
                                        }
                                    }
                                    drop(state);
                                    self.results.notify_all();
                                }
                                Err(error) => {
                                    self.fail(&mut worker, error);
                                    return;
                                }
                            }
                            continue;
                        }
                    }
                };
                let (id, outcome) = match job {
                    Job::Begin(id, bytes) => (id, worker.begin(&bytes)),
                    Job::Resume(id, bytes) => {
                        let slot = self.lock().away[&id]
                            .slot
                            .expect("a started task has a slot");
                        (
                            id,
                            worker
                                .resume(slot, &bytes)
                                .map(|(ended, message)| (slot, ended, message)),
                        )
                    }
                };
                match outcome {
                    Ok((slot, ended, message)) => {
                        slots.insert(slot, id);
                        let mut state = self.lock();
                        state.ran[index] += 1;
                        let away = state.away.get_mut(&id).expect("a running task is away");
                        away.slot = Some(slot);
                        away.ended = ended;
                        away.messages.push_back(message);
                        let deliver = away.serial.is_some() && away.messages.len() == 1;
                        if deliver {
                            state.inbox.push_back(id);
                        }
                        if ended {
                            state.live[index] -= 1;
                        }
                        drop(state);
                        self.results.notify_all();
                    }
                    Err(error) => {
                        self.fail(&mut worker, error);
                        return;
                    }
                }
            }
        });
    }

    fn fail(&self, worker: &mut Worker, error: anyhow::Error) {
        if std::env::var_os("I_WORKERS_TRACE").is_some() {
            eprintln!("i: worker failed: {error:?}");
        }
        let message = worker
            .store
            .data_mut()
            .stop_message
            .take()
            .unwrap_or_else(|| format!("{error:#}"));
        if !self.lock().shutdown {
            self.stopped(message);
        }
    }
}

struct Worker {
    store: Store<State>,
    instance: wasmtime::Instance,
    checked: bool,
}

impl Worker {
    fn new(pool: &Arc<Pool>, index: usize) -> Result<Self> {
        let mut linker: Linker<State> = Linker::new(&pool.engine);
        crate::define_builtins(&mut linker)?;
        linker.allow_shadowing(true);
        define_remote(&mut linker, Arc::downgrade(pool))?;
        for (kind, modules) in [(true, &pool.abi.services), (false, &pool.abi.externals)] {
            let _ = kind;
            for (module, functions) in modules {
                for (name, signature) in functions {
                    let Some(function) = pool
                        .options
                        .imports
                        .functions
                        .get(&(module.clone(), name.clone()))
                    else {
                        continue;
                    };
                    crate::define_import(
                        &mut linker,
                        &pool.engine,
                        module,
                        name,
                        signature,
                        function.clone(),
                        pool.options.check_replies,
                    )?;
                }
            }
        }
        linker.define_unknown_imports_as_traps(&pool.module)?;
        let mut state = State::worker();
        if let Some(resources) = pool
            .resources
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
        {
            state.resources = resources;
        }
        let mut store = Store::new(&pool.engine, state);
        store.set_epoch_deadline(1);
        let abandon = Arc::clone(&pool.abandon);
        store.epoch_deadline_callback(move |_| {
            if abandon.load(Ordering::SeqCst) {
                Err(wasmtime::Error::from_anyhow(anyhow!("the run ended")))
            } else {
                Ok(UpdateDeadline::Continue(1))
            }
        });
        let instance = linker.instantiate(&mut store, &pool.module)?;
        let start = instance.get_typed_func::<(), ()>(&mut store, "ilang_start")?;
        start.call(&mut store, ())?;
        let setup = instance.get_typed_func::<i64, ()>(&mut store, "ilang_mc_setup")?;
        setup.call(&mut store, id_prefix(index, 1))?;
        Ok(Self {
            store,
            instance,
            checked: pool.options.check_replies,
        })
    }

    fn input(&mut self, bytes: &[u8]) -> Result<(i32, i32)> {
        let memory = self
            .instance
            .get_memory(&mut self.store, "memory")
            .context("memory")?;
        let alloc = self
            .instance
            .get_typed_func::<i32, i32>(&mut self.store, "ilang_alloc")?;
        let len = bytes.len() as i32;
        let ptr = crate::run::in_wasm(|| alloc.call(&mut self.store, len))?;
        memory.write(&mut self.store, ptr as usize, bytes)?;
        Ok((ptr, len))
    }

    fn output(&mut self, input: (i32, i32)) -> Result<Vec<u8>> {
        let store = &mut self.store;
        let free = self
            .instance
            .get_typed_func::<(i32, i32), ()>(&mut *store, "ilang_free")?;
        if input.0 != 0 {
            free.call(&mut *store, input)?;
        }
        let get = |store: &mut Store<State>, name: &str| -> Result<i32> {
            Ok(self
                .instance
                .get_typed_func::<(), i32>(&mut *store, name)?
                .call(&mut *store, ())?)
        };
        let ptr = get(store, "ilang_mc_out_ptr")?;
        let len = get(store, "ilang_mc_out_len")?;
        let cap = get(store, "ilang_mc_out_cap")?;
        let memory = self
            .instance
            .get_memory(&mut *store, "memory")
            .context("memory")?;
        let message = memory
            .data(&*store)
            .get(ptr as usize..ptr as usize + len as usize)
            .context("worker message outside memory")?
            .to_vec();
        free.call(&mut *store, (ptr, cap))?;
        Ok(message)
    }

    /// Settle a request cell of a task here; gives the `Woken` messages of
    /// the tasks whose awaited requests settled.
    fn settle(
        &mut self,
        id: u32,
        request: &crate::requests::Request,
        reply: crate::requests::Reply,
    ) -> Result<Vec<(i32, Vec<u8>)>> {
        let answer = reply?;
        let name = format!("ilang_settle:{}:{}", request.namespace, request.label);
        let settle = self
            .instance
            .get_func(&mut self.store, &name)
            .with_context(|| format!("the module does not export {name}"))?;
        let bridge = Bridge::from_instance(&self.instance, &mut self.store);
        let mut values = vec![wasmtime::Val::I32(id as i32)];
        let checked = self.checked;
        ensure!(
            crate::lower_reply(
                &mut self.store,
                bridge,
                &request.signature,
                answer,
                &mut values,
                checked
            )?,
            "the host gave a value that is not correct"
        );
        crate::run::in_wasm(|| settle.call(&mut self.store, &values, &mut []))?;
        let ready = self
            .instance
            .get_typed_func::<(), i32>(&mut self.store, "ilang_mc_ready")?;
        let mut woken = Vec::new();
        loop {
            let slot = ready.call(&mut self.store, ())?;
            if slot < 0 {
                return Ok(woken);
            }
            woken.push((slot, self.output((0, 0))?));
        }
    }

    /// Run a claimed record to its first step: (slot, ended, message).
    fn begin(&mut self, bytes: &[u8]) -> Result<(i32, bool, Vec<u8>)> {
        let input = self.input(bytes)?;
        let begin = self
            .instance
            .get_typed_func::<(i32, i32), i64>(&mut self.store, "ilang_mc_begin")?;
        self.store.data_mut().requests.set_accepting(true);
        let answer = crate::run::in_wasm(|| begin.call(&mut self.store, input));
        self.store.data_mut().requests.set_accepting(false);
        let answer = answer?;
        Ok(((answer >> 1) as i32, answer & 1 == 1, self.output(input)?))
    }

    /// Answer a task's step; it runs to its next step: its message.
    fn resume(&mut self, slot: i32, bytes: &[u8]) -> Result<(bool, Vec<u8>)> {
        let input = self.input(bytes)?;
        let resume = self
            .instance
            .get_typed_func::<(i32, i32, i32), i32>(&mut self.store, "ilang_mc_resume")?;
        self.store.data_mut().requests.set_accepting(true);
        let ended = crate::run::in_wasm(|| resume.call(&mut self.store, (slot, input.0, input.1)));
        self.store.data_mut().requests.set_accepting(false);
        let ended = ended?;
        Ok((ended == 1, self.output(input)?))
    }
}

/// The runtime imports of a worker other than 0.
fn define_remote(linker: &mut Linker<State>, pool: std::sync::Weak<Pool>) -> Result<()> {
    fn refuse() -> wasmtime::Error {
        wasmtime::Error::from_anyhow(anyhow!(
            "a scheduler operation of worker 0 ran on another worker"
        ))
    }
    linker.func_wrap(
        HOST_MODULE,
        "mc_publish",
        |_: Caller<'_, State>, _: i32, _: i32, _: i32| -> wasmtime::Result<i64> { Ok(0) },
    )?;
    linker.func_wrap(
        HOST_MODULE,
        "mc_claim",
        |_: Caller<'_, State>, _: i64| -> wasmtime::Result<i32> { Err(refuse()) },
    )?;
    linker.func_wrap(
        HOST_MODULE,
        "mc_adopt",
        |_: Caller<'_, State>, _: i64, _: i64| -> wasmtime::Result<()> { Err(refuse()) },
    )?;
    linker.func_wrap(
        HOST_MODULE,
        "mc_send",
        |_: Caller<'_, State>, _: i64, _: i32, _: i32| -> wasmtime::Result<()> { Err(refuse()) },
    )?;
    linker.func_wrap(
        HOST_MODULE,
        "mc_inbox",
        |_: Caller<'_, State>| -> wasmtime::Result<i64> { Ok(0) },
    )?;
    linker.func_wrap(
        HOST_MODULE,
        "mc_message",
        |_: Caller<'_, State>| -> wasmtime::Result<i64> { Err(refuse()) },
    )?;
    linker.func_wrap(
        HOST_MODULE,
        "mc_pending",
        |_: Caller<'_, State>| -> wasmtime::Result<i32> { Ok(0) },
    )?;
    linker.func_wrap(
        HOST_MODULE,
        "mc_wait",
        |_: Caller<'_, State>| -> wasmtime::Result<()> { Ok(()) },
    )?;
    linker.func_wrap(
        HOST_MODULE,
        "mc_share_put",
        |_: Caller<'_, State>, _: i64, _: i64, _: i64, _: i32, _: i32| -> wasmtime::Result<()> {
            Ok(())
        },
    )?;
    linker.func_wrap(
        HOST_MODULE,
        "mc_share_drop",
        |_: Caller<'_, State>, _: i64, _: i64, _: i64| -> wasmtime::Result<()> { Ok(()) },
    )?;
    linker.func_wrap(
        HOST_MODULE,
        "mc_share_get",
        move |mut caller: Caller<'_, State>,
              run: i64,
              task: i64,
              at: i64|
              -> wasmtime::Result<i64> {
            let Some(bytes) = pool
                .upgrade()
                .and_then(|pool| pool.share_get((run, task, at)))
            else {
                return Ok(0);
            };
            let bridge = Bridge::from_caller(&mut caller);
            let ptr = bridge
                .write(&mut caller, &bytes, &mut Vec::new())
                .map_err(wasmtime::Error::from_anyhow)?;
            Ok(((ptr as i64) << 32) | bytes.len() as i64)
        },
    )?;
    Ok(())
}

/// The runtime imports of worker 0.
pub(crate) fn define_home(linker: &mut Linker<State>) -> Result<()> {
    fn pool(caller: &Caller<'_, State>) -> Option<Arc<Pool>> {
        caller.data().workers.clone()
    }
    /// A stop on another worker stops the run with its message (N12).
    fn stop(caller: &mut Caller<'_, State>, message: String) -> wasmtime::Error {
        caller.data_mut().stop_message.get_or_insert(message);
        wasmtime::Error::from_anyhow(anyhow!("a task on another worker stopped"))
    }
    linker.func_wrap(
        HOST_MODULE,
        "mc_publish",
        |mut caller: Caller<'_, State>,
         ptr: i32,
         len: i32,
         affinity: i32|
         -> wasmtime::Result<i64> {
            let Some(pool) = pool(&caller) else {
                return Ok(0);
            };
            let bridge = Bridge::from_caller(&mut caller);
            let bytes = bridge
                .read(&caller, ptr as u32, len as u32)
                .map_err(wasmtime::Error::from_anyhow)?;
            let resources = caller.data().resources.clone();
            Ok(pool.publish(bytes, affinity, &resources))
        },
    )?;
    linker.func_wrap(
        HOST_MODULE,
        "mc_claim",
        |caller: Caller<'_, State>, id: i64| -> wasmtime::Result<i32> {
            Ok(pool(&caller).is_none_or(|pool| pool.claim(id as u64)) as i32)
        },
    )?;
    linker.func_wrap(
        HOST_MODULE,
        "mc_adopt",
        |caller: Caller<'_, State>, id: i64, serial: i64| -> wasmtime::Result<()> {
            pool(&caller)
                .expect("mc_adopt with workers")
                .adopt(id as u64, serial);
            Ok(())
        },
    )?;
    linker.func_wrap(
        HOST_MODULE,
        "mc_send",
        |mut caller: Caller<'_, State>, serial: i64, ptr: i32, len: i32| -> wasmtime::Result<()> {
            let pool = pool(&caller).expect("mc_send with workers");
            let bridge = Bridge::from_caller(&mut caller);
            let bytes = bridge
                .read(&caller, ptr as u32, len as u32)
                .map_err(wasmtime::Error::from_anyhow)?;
            pool.send(serial, bytes);
            Ok(())
        },
    )?;
    linker.func_wrap(
        HOST_MODULE,
        "mc_inbox",
        |mut caller: Caller<'_, State>| -> wasmtime::Result<i64> {
            let Some(pool) = pool(&caller) else {
                return Ok(0);
            };
            pool.inbox().map_err(|message| stop(&mut caller, message))
        },
    )?;
    linker.func_wrap(
        HOST_MODULE,
        "mc_message",
        |mut caller: Caller<'_, State>| -> wasmtime::Result<i64> {
            let pool = pool(&caller).expect("mc_message with workers");
            let bytes = pool.message();
            let bridge = Bridge::from_caller(&mut caller);
            let ptr = bridge
                .write(&mut caller, &bytes, &mut Vec::new())
                .map_err(wasmtime::Error::from_anyhow)?;
            Ok(((ptr as i64) << 32) | bytes.len() as i64)
        },
    )?;
    linker.func_wrap(
        HOST_MODULE,
        "mc_pending",
        |mut caller: Caller<'_, State>| -> wasmtime::Result<i32> {
            let Some(pool) = pool(&caller) else {
                return Ok(0);
            };
            pool.pending()
                .map(i32::from)
                .map_err(|message| stop(&mut caller, message))
        },
    )?;
    linker.func_wrap(
        HOST_MODULE,
        "mc_wait",
        |caller: Caller<'_, State>| -> wasmtime::Result<()> {
            if let Some(pool) = pool(&caller) {
                pool.wait();
            }
            Ok(())
        },
    )?;
    linker.func_wrap(
        HOST_MODULE,
        "mc_share_put",
        |mut caller: Caller<'_, State>,
         run: i64,
         task: i64,
         at: i64,
         ptr: i32,
         len: i32|
         -> wasmtime::Result<()> {
            let Some(pool) = pool(&caller) else {
                return Ok(());
            };
            let bridge = Bridge::from_caller(&mut caller);
            let bytes = bridge
                .read(&caller, ptr as u32, len as u32)
                .map_err(wasmtime::Error::from_anyhow)?;
            pool.share_put((run, task, at), bytes);
            Ok(())
        },
    )?;
    linker.func_wrap(
        HOST_MODULE,
        "mc_share_drop",
        |caller: Caller<'_, State>, run: i64, task: i64, at: i64| -> wasmtime::Result<()> {
            if let Some(pool) = pool(&caller) {
                pool.share_drop((run, task, at));
            }
            Ok(())
        },
    )?;
    linker.func_wrap(
        HOST_MODULE,
        "mc_share_get",
        |_: Caller<'_, State>, _: i64, _: i64, _: i64| -> wasmtime::Result<i64> { Ok(0) },
    )?;
    Ok(())
}

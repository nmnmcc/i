//! Native multicore workers (multicore.md D8/D12, native.md N5/N7/N10/N11).
//!
//! One module, N instances: the instance of the entry is worker 0 (the home
//! of every started task); workers 1..N each have their own `Store`, on their
//! own OS thread. At `go` the scheduler of `i/task` encodes a portable task
//! record into one shared queue (`mc_publish`). A worker claims an unstarted
//! record once: either the maker (`mc_claim`, which runs its retained
//! original) or a remote worker, which decodes the bytes and runs the body
//! as a speculation. A speculation has no scheduler step, host call or stop
//! in it: every import of a remote instance traps, so the worker gives the
//! record back and the maker runs its original. A finished speculation gives
//! its encoded completion to the maker (`mc_poll`, `mc_take`). The maker can
//! forget a record (`mc_forget`, after a cancel); a remote worker that runs it
//! is interrupted through its epoch deadline (N12) and makes a new instance.
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

use anyhow::{Context, Result, anyhow};
use wasmtime::{Caller, Engine, Linker, Module, Store, UpdateDeadline};

use crate::{Bridge, HOST_MODULE, State, THREAD_STACK};

/// The number of workers of an entry run: `I_WORKERS` when it is set to a
/// positive number, else the available cores.
pub fn worker_count() -> usize {
    match std::env::var("I_WORKERS") {
        Ok(text) => match text.trim().parse::<usize>() {
            Ok(n) if n >= 1 => n.min(255),
            _ => panic!("I_WORKERS must be a positive number of workers, not {text:?}"),
        },
        Err(_) => std::thread::available_parallelism().map_or(1, |n| n.get().min(255)),
    }
}

/// The key id prefix of a worker (D6): worker w takes ids above
/// `(w + 1) << 56`, and a new instance of the same worker a new generation.
pub(crate) fn id_prefix(worker: usize, generation: u64) -> i64 {
    (((worker as u64 + 1) << 56) | ((generation & 0xffff) << 40)) as i64
}

type ShareKey = (i64, i64, i64);

enum Entry {
    Ready(Vec<u8>),
    Taken(usize),
    /// The completion and the shared values the speculation read.
    Done(Vec<u8>, Vec<ShareKey>),
    Returned,
}

#[derive(Clone, Copy)]
enum Kind {
    Ready,
    Taken,
    Done,
    Returned,
    Gone,
}

fn kind(entry: Option<&Entry>) -> Kind {
    match entry {
        Some(Entry::Ready(_)) => Kind::Ready,
        Some(Entry::Taken(_)) => Kind::Taken,
        Some(Entry::Done(..)) => Kind::Done,
        Some(Entry::Returned) => Kind::Returned,
        None => Kind::Gone,
    }
}

struct Slot {
    abandon: Arc<AtomicBool>,
    running: Option<u64>,
    reads: Vec<ShareKey>,
}

#[derive(Default)]
struct PoolState {
    queue: VecDeque<u64>,
    entries: HashMap<u64, Entry>,
    next: u64,
    slots: Vec<Slot>,
    threads: Vec<JoinHandle<()>>,
    idle: usize,
    finished: usize,
    shutdown: bool,
    broken: bool,
    /// Tasks run per worker: index 0 is the home worker.
    ran: Vec<u64>,
    published: u64,
    /// `scope.share` values, encoded once, until their scope ends.
    shares: HashMap<ShareKey, Arc<Vec<u8>>>,
    /// Speculations interrupted because their maker forgot them (a cancel).
    stopped: u64,
}

pub(crate) struct Pool {
    state: Mutex<PoolState>,
    work: Condvar,
    results: Condvar,
    engine: Engine,
    module: Module,
    remote: usize,
    ticker: Mutex<Option<JoinHandle<()>>>,
}

impl Pool {
    pub(crate) fn new(engine: Engine, module: Module, workers: usize) -> Arc<Self> {
        Arc::new(Self {
            state: Mutex::new(PoolState {
                next: 1,
                ran: vec![0; workers],
                ..PoolState::default()
            }),
            work: Condvar::new(),
            results: Condvar::new(),
            engine,
            module,
            remote: workers - 1,
            ticker: Mutex::new(None),
        })
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, PoolState> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Start the epoch ticker that lets a forgotten speculation be stopped.
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

    pub(crate) fn publish(self: &Arc<Self>, bytes: Vec<u8>) -> i64 {
        let mut state = self.lock();
        if state.broken || state.shutdown {
            return 0;
        }
        let id = state.next;
        state.next += 1;
        state.published += 1;
        state.entries.insert(id, Entry::Ready(bytes));
        state.queue.push_back(id);
        if state.idle == 0 && state.slots.len() < self.remote {
            let index = state.slots.len() + 1;
            let abandon = Arc::new(AtomicBool::new(false));
            state.slots.push(Slot {
                abandon: Arc::clone(&abandon),
                running: None,
                reads: Vec::new(),
            });
            let pool = Arc::clone(self);
            let thread = std::thread::Builder::new()
                .name(format!("ilang-worker-{index}"))
                .stack_size(THREAD_STACK)
                .spawn(move || pool.worker(index, abandon));
            match thread {
                Ok(thread) => state.threads.push(thread),
                Err(_) => {
                    state.slots.pop();
                }
            }
            drop(state);
            self.ensure_ticker();
        } else {
            drop(state);
        }
        self.work.notify_one();
        id as i64
    }

    /// The maker takes its own record when no worker has claimed it.
    pub(crate) fn claim(&self, id: u64) -> bool {
        let mut state = self.lock();
        match kind(state.entries.get(&id)) {
            Kind::Ready => {
                state.entries.remove(&id);
                if let Some(at) = state.queue.iter().position(|x| *x == id) {
                    state.queue.remove(at);
                }
                state.ran[0] += 1;
                true
            }
            Kind::Returned => {
                state.entries.remove(&id);
                state.finished -= 1;
                state.ran[0] += 1;
                true
            }
            Kind::Taken | Kind::Done => false,
            Kind::Gone => true,
        }
    }

    /// 0 while a worker runs the record, 1 when its completion is there, 2
    /// when the worker gave the record back. Waits a little when no result of
    /// any record is there, so a home worker with nothing else to do does
    /// not spin.
    pub(crate) fn poll(&self, id: u64) -> i32 {
        let mut state = self.lock();
        for round in 0..2 {
            match kind(state.entries.get(&id)) {
                Kind::Done => {
                    // A value read by the speculation may have ended with
                    // its scope since: then the maker runs its original.
                    let Some(Entry::Done(_, reads)) = state.entries.get(&id) else {
                        unreachable!()
                    };
                    if reads.iter().all(|key| state.shares.contains_key(key)) {
                        return 1;
                    }
                    state.entries.insert(id, Entry::Returned);
                    continue;
                }
                Kind::Returned | Kind::Gone => {
                    if state.entries.remove(&id).is_some() {
                        state.finished -= 1;
                        state.ran[0] += 1;
                    }
                    return 2;
                }
                Kind::Ready => {
                    // Not claimed by anyone yet: the maker takes it now.
                    state.entries.remove(&id);
                    if let Some(at) = state.queue.iter().position(|x| *x == id) {
                        state.queue.remove(at);
                    }
                    state.ran[0] += 1;
                    return 2;
                }
                Kind::Taken => {}
            }
            if round == 0 && state.finished == 0 {
                state = self
                    .results
                    .wait_timeout(state, Duration::from_micros(500))
                    .unwrap_or_else(|e| e.into_inner())
                    .0;
            } else {
                break;
            }
        }
        0
    }

    pub(crate) fn take(&self, id: u64) -> Option<Vec<u8>> {
        let mut state = self.lock();
        match state.entries.remove(&id) {
            Some(Entry::Done(bytes, _)) => {
                state.finished -= 1;
                Some(bytes)
            }
            Some(other) => {
                state.entries.insert(id, other);
                None
            }
            None => None,
        }
    }

    pub(crate) fn share_put(&self, key: ShareKey, bytes: Vec<u8>) {
        self.lock().shares.insert(key, Arc::new(bytes));
    }

    pub(crate) fn share_drop(&self, key: ShareKey) {
        self.lock().shares.remove(&key);
    }

    /// A remote worker reads a shared value; its maker checks the read when
    /// it takes the completion.
    fn share_get(&self, worker: usize, key: ShareKey) -> Option<Arc<Vec<u8>>> {
        let mut state = self.lock();
        let bytes = state.shares.get(&key).cloned()?;
        state.slots[worker - 1].reads.push(key);
        Some(bytes)
    }

    pub(crate) fn forget(&self, id: u64) {
        let mut state = self.lock();
        match state.entries.remove(&id) {
            Some(Entry::Ready(_)) => {
                if let Some(at) = state.queue.iter().position(|x| *x == id) {
                    state.queue.remove(at);
                }
            }
            Some(Entry::Taken(worker)) => {
                let slot = &state.slots[worker - 1];
                if slot.running == Some(id) {
                    slot.abandon.store(true, Ordering::SeqCst);
                }
            }
            Some(Entry::Done(..) | Entry::Returned) => state.finished -= 1,
            None => {}
        }
    }

    /// Stop every worker, interrupting speculations, and join the threads.
    pub(crate) fn shutdown(&self) {
        let threads = {
            let mut state = self.lock();
            if state.shutdown {
                return;
            }
            state.shutdown = true;
            for slot in &state.slots {
                slot.abandon.store(true, Ordering::SeqCst);
            }
            state.queue.clear();
            state.entries.clear();
            if std::env::var_os("I_WORKERS_TRACE").is_some() && state.published > 0 {
                eprintln!(
                    "i: workers {} published {} ran {:?} stopped {}",
                    self.remote + 1,
                    state.published,
                    state.ran,
                    state.stopped
                );
            }
            std::mem::take(&mut state.threads)
        };
        self.work.notify_all();
        self.results.notify_all();
        self.engine.increment_epoch();
        for thread in threads {
            let _ = thread.join();
        }
        if let Some(ticker) = self
            .ticker
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take()
        {
            let _ = ticker.join();
        }
    }

    fn worker(self: Arc<Self>, index: usize, abandon: Arc<AtomicBool>) {
        let mut generation = 0u64;
        'instances: loop {
            generation += 1;
            let mut worker = match Worker::new(&self, index, generation, Arc::clone(&abandon)) {
                Ok(worker) => worker,
                Err(_) => {
                    // A start-up that calls the host, or fails, keeps this
                    // run on the home worker (D7, D12); the maker claims
                    // every record that is still queued.
                    self.lock().broken = true;
                    return;
                }
            };
            loop {
                let (id, bytes) = {
                    let mut state = self.lock();
                    loop {
                        if state.shutdown {
                            return;
                        }
                        if let Some(id) = state.queue.pop_front() {
                            let bytes = match state.entries.insert(id, Entry::Taken(index)) {
                                Some(Entry::Ready(bytes)) => bytes,
                                _ => panic!("worker queue entry {id} was not ready"),
                            };
                            state.slots[index - 1].running = Some(id);
                            abandon.store(false, Ordering::SeqCst);
                            break (id, bytes);
                        }
                        state.idle += 1;
                        state = self.work.wait(state).unwrap_or_else(|e| e.into_inner());
                        state.idle -= 1;
                    }
                };
                let outcome = worker.run(&bytes);
                drop(bytes);
                let mut state = self.lock();
                state.slots[index - 1].running = None;
                let reads = std::mem::take(&mut state.slots[index - 1].reads);
                let current = matches!(state.entries.get(&id), Some(Entry::Taken(w)) if *w == index);
                let trapped = outcome.is_err();
                if current {
                    match outcome {
                        Ok(Some(result)) => {
                            state.entries.insert(id, Entry::Done(result, reads));
                            state.ran[index] += 1;
                        }
                        Ok(None) | Err(_) => {
                            state.entries.insert(id, Entry::Returned);
                        }
                    }
                    state.finished += 1;
                    drop(state);
                    self.results.notify_all();
                } else {
                    if trapped && !state.shutdown {
                        state.stopped += 1;
                    }
                    drop(state);
                }
                if trapped {
                    continue 'instances;
                }
            }
        }
    }
}

struct Worker {
    store: Store<State>,
    instance: wasmtime::Instance,
}

impl Worker {
    fn new(pool: &Arc<Pool>, index: usize, generation: u64, abandon: Arc<AtomicBool>) -> Result<Self> {
        let mut linker: Linker<State> = Linker::new(&pool.engine);
        define_remote(&mut linker)?;
        let weak = Arc::downgrade(pool);
        linker.func_wrap(
            HOST_MODULE,
            "mc_share_get",
            move |mut caller: Caller<'_, State>, run: i64, task: i64, at: i64| -> wasmtime::Result<i64> {
                let Some(bytes) = weak.upgrade().and_then(|pool| pool.share_get(index, (run, task, at)))
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
        linker.define_unknown_imports_as_traps(&pool.module)?;
        let mut store = Store::new(&pool.engine, State::worker());
        store.set_epoch_deadline(1);
        store.epoch_deadline_callback(move |_| {
            if abandon.load(Ordering::SeqCst) {
                Err(wasmtime::Error::from_anyhow(anyhow!("the speculation was abandoned")))
            } else {
                Ok(UpdateDeadline::Continue(1))
            }
        });
        let instance = linker.instantiate(&mut store, &pool.module)?;
        let start = instance.get_typed_func::<(), ()>(&mut store, "ilang_start")?;
        start.call(&mut store, ())?;
        let setup = instance.get_typed_func::<i64, ()>(&mut store, "ilang_mc_setup")?;
        setup.call(&mut store, id_prefix(index, generation))?;
        Ok(Self { store, instance })
    }

    /// Ok(Some(bytes)): the encoded completion; Ok(None): the record goes
    /// back to its maker; Err: the instance trapped and is not used again.
    fn run(&mut self, bytes: &[u8]) -> Result<Option<Vec<u8>>> {
        let store = &mut self.store;
        let memory = self
            .instance
            .get_memory(&mut *store, "memory")
            .context("memory")?;
        let alloc = self
            .instance
            .get_typed_func::<i32, i32>(&mut *store, "ilang_alloc")?;
        let free = self
            .instance
            .get_typed_func::<(i32, i32), ()>(&mut *store, "ilang_free")?;
        let run = self
            .instance
            .get_typed_func::<(i32, i32), i32>(&mut *store, "ilang_mc_run")?;
        let len = bytes.len() as i32;
        let ptr = alloc.call(&mut *store, len)?;
        memory.write(&mut *store, ptr as usize, bytes)?;
        let status = run.call(&mut *store, (ptr, len))?;
        free.call(&mut *store, (ptr, len))?;
        if status != 1 {
            // The instance may hold a partly unwound shadow stack; start a
            // new one rather than reason about it.
            return Err(anyhow!("the speculation stopped"));
        }
        let out_ptr = self
            .instance
            .get_typed_func::<(), i32>(&mut *store, "ilang_mc_out_ptr")?
            .call(&mut *store, ())?;
        let out_len = self
            .instance
            .get_typed_func::<(), i32>(&mut *store, "ilang_mc_out_len")?
            .call(&mut *store, ())?;
        let out_cap = self
            .instance
            .get_typed_func::<(), i32>(&mut *store, "ilang_mc_out_cap")?
            .call(&mut *store, ())?;
        let data = memory.data(&*store);
        let result = data
            .get(out_ptr as usize..(out_ptr as usize + out_len as usize))
            .context("speculation result outside memory")?
            .to_vec();
        free.call(&mut *store, (out_ptr, out_cap))?;
        Ok(Some(result))
    }
}

/// The runtime imports of a remote instance: every one ends the speculation.
fn define_remote(linker: &mut Linker<State>) -> Result<()> {
    fn stop() -> wasmtime::Error {
        wasmtime::Error::from_anyhow(anyhow!("a speculation reached the host"))
    }
    linker.func_wrap(HOST_MODULE, "panic", |_: Caller<'_, State>, _: i32, _: i32| -> wasmtime::Result<()> {
        Err(stop())
    })?;
    linker.func_wrap(HOST_MODULE, "mc_publish", |_: Caller<'_, State>, _: i32, _: i32| -> wasmtime::Result<i64> {
        Err(stop())
    })?;
    linker.func_wrap(HOST_MODULE, "mc_claim", |_: Caller<'_, State>, _: i64| -> wasmtime::Result<i32> {
        Err(stop())
    })?;
    linker.func_wrap(HOST_MODULE, "mc_poll", |_: Caller<'_, State>, _: i64| -> wasmtime::Result<i32> {
        Err(stop())
    })?;
    linker.func_wrap(HOST_MODULE, "mc_take", |_: Caller<'_, State>, _: i64| -> wasmtime::Result<i64> {
        Err(stop())
    })?;
    linker.func_wrap(HOST_MODULE, "mc_forget", |_: Caller<'_, State>, _: i64| -> wasmtime::Result<()> {
        Err(stop())
    })?;
    linker.func_wrap(
        HOST_MODULE,
        "mc_share_put",
        |_: Caller<'_, State>, _: i64, _: i64, _: i64, _: i32, _: i32| -> wasmtime::Result<()> { Err(stop()) },
    )?;
    linker.func_wrap(
        HOST_MODULE,
        "mc_share_drop",
        |_: Caller<'_, State>, _: i64, _: i64, _: i64| -> wasmtime::Result<()> { Err(stop()) },
    )?;
    Ok(())
}

/// The runtime imports of the home instance.
pub(crate) fn define_home(linker: &mut Linker<State>) -> Result<()> {
    fn pool(caller: &Caller<'_, State>) -> Option<Arc<Pool>> {
        caller.data().workers.clone()
    }
    linker.func_wrap(
        HOST_MODULE,
        "mc_publish",
        |mut caller: Caller<'_, State>, ptr: i32, len: i32| -> wasmtime::Result<i64> {
            let Some(pool) = pool(&caller) else { return Ok(0) };
            let bridge = Bridge::from_caller(&mut caller);
            let bytes = bridge
                .read(&caller, ptr as u32, len as u32)
                .map_err(wasmtime::Error::from_anyhow)?;
            Ok(pool.publish(bytes))
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
        "mc_poll",
        |caller: Caller<'_, State>, id: i64| -> wasmtime::Result<i32> {
            Ok(pool(&caller).map_or(2, |pool| pool.poll(id as u64)))
        },
    )?;
    linker.func_wrap(
        HOST_MODULE,
        "mc_take",
        |mut caller: Caller<'_, State>, id: i64| -> wasmtime::Result<i64> {
            let bytes = pool(&caller)
                .and_then(|pool| pool.take(id as u64))
                .ok_or_else(|| wasmtime::Error::from_anyhow(anyhow!("mc_take of a record that is not done")))?;
            let bridge = Bridge::from_caller(&mut caller);
            let mut allocations = Vec::new();
            let ptr = bridge
                .write(&mut caller, &bytes, &mut allocations)
                .map_err(wasmtime::Error::from_anyhow)?;
            Ok(((ptr as i64) << 32) | bytes.len() as i64)
        },
    )?;
    linker.func_wrap(
        HOST_MODULE,
        "mc_forget",
        |caller: Caller<'_, State>, id: i64| -> wasmtime::Result<()> {
            if let Some(pool) = pool(&caller) {
                pool.forget(id as u64);
            }
            Ok(())
        },
    )?;
    linker.func_wrap(
        HOST_MODULE,
        "mc_share_put",
        |mut caller: Caller<'_, State>, run: i64, task: i64, at: i64, ptr: i32, len: i32| -> wasmtime::Result<()> {
            let Some(pool) = pool(&caller) else { return Ok(()) };
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
    // Only a speculation reads shared values from the transport.
    linker.func_wrap(
        HOST_MODULE,
        "mc_share_get",
        |_: Caller<'_, State>, _: i64, _: i64, _: i64| -> wasmtime::Result<i64> { Ok(0) },
    )?;
    Ok(())
}

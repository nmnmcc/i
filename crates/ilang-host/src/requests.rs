//! The request table of an instance (B.4, B.5): the async host functions
//! that the module has started and the host has not answered, keyed by the
//! request id that the module chose.
//!
//! Each request keeps its work future here, in the `Store`, not in the
//! future of the call that waits. A request ends when it is answered, when
//! the module cancels it, or when the host drops every request (the entry
//! point ended, a trap, a host failure, or the waiting call was dropped).
//! Dropping a request drops its work future at once.
//!
//! Each request has its own waker. A wake puts the request in a queue, so a
//! wait polls only the work that was woken, and answers come in the order
//! the work finishes.

use std::{
    collections::{HashMap, VecDeque},
    future::poll_fn,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    task::{Context, Poll, Wake, Waker},
};

use anyhow::{Result, bail};

use crate::{ServiceFuture, Signature, Value};

/// The import that started a request.
pub(crate) struct Request {
    pub namespace: String,
    pub label: String,
    pub signature: Signature,
}

/// What the work of a request gave: the answer (a value or an error of
/// `? E`), or a failure of the host.
pub(crate) type Reply = Result<std::result::Result<Value, Value>>;

struct Entry {
    request: Request,
    /// Tells a request apart from an earlier one with the same id.
    sequence: u64,
    work: ServiceFuture,
    notify: Arc<Notify>,
}

/// The requests whose work has woken, and the waker of the wait.
#[derive(Default)]
struct Woken {
    ready: VecDeque<(u32, u64)>,
    waiter: Option<Waker>,
}

/// The waker of one request.
struct Notify {
    id: u32,
    sequence: u64,
    /// Whether the request is in the queue already.
    queued: AtomicBool,
    woken: Arc<Mutex<Woken>>,
}

impl Wake for Notify {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        if self.queued.swap(true, Ordering::AcqRel) {
            return;
        }
        let waiter = {
            let mut woken = self.woken.lock().unwrap_or_else(|e| e.into_inner());
            woken.ready.push_back((self.id, self.sequence));
            woken.waiter.take()
        };
        if let Some(waiter) = waiter {
            waiter.wake();
        }
    }
}

#[derive(Default)]
pub(crate) struct Requests {
    /// Whether a request may start now (B.4): true only while the host runs
    /// an async entry point, a resume or `ilang_continue`, and not inside
    /// an import.
    accepting: bool,
    outstanding: HashMap<u32, Entry>,
    next_sequence: u64,
    woken: Arc<Mutex<Woken>>,
}

impl Requests {
    /// Returns the flag and sets it to false. Each import wrapper calls it
    /// first and puts the value back when it returns (B.4).
    pub fn take_accepting(&mut self) -> bool {
        std::mem::replace(&mut self.accepting, false)
    }

    pub fn set_accepting(&mut self, value: bool) {
        self.accepting = value;
    }

    pub fn is_empty(&self) -> bool {
        self.outstanding.is_empty()
    }

    /// Whether a request with this id may start: `admitted` is the flag
    /// that the import wrapper took, and the id is 0 or more and not
    /// outstanding (B.4). The host checks this before it calls the service.
    pub fn admit(&self, admitted: bool, id: i32) -> Result<u32> {
        if !admitted {
            bail!("the module started request {id} outside an async call");
        }
        let Ok(id) = u32::try_from(id) else {
            bail!("the module started request {id}, which is below 0");
        };
        if self.outstanding.contains_key(&id) {
            bail!("the module started request {id} twice");
        }
        Ok(id)
    }

    /// Adds a request that `admit` accepted, with the work of its service.
    pub fn insert(&mut self, id: u32, request: Request, work: ServiceFuture) {
        let sequence = self.next_sequence;
        self.next_sequence += 1;
        let notify = Arc::new(Notify {
            id,
            sequence,
            queued: AtomicBool::new(false),
            woken: Arc::clone(&self.woken),
        });
        // The work is polled for the first time when the host waits.
        Waker::from(Arc::clone(&notify)).wake();
        self.outstanding.insert(
            id,
            Entry {
                request,
                sequence,
                work,
                notify,
            },
        );
    }

    /// Polls the work that has woken, and gives the first request that is
    /// done, removed from the table; `None` when nothing is outstanding.
    ///
    /// Each call polls only the requests that were in the queue when it
    /// began. Work that wakes itself while it is polled goes back in the
    /// queue and wakes the waiter, so it is polled on the next call, after
    /// the executor has had a turn (its timers and IO, and other tasks).
    fn poll_next(&mut self, context: &Context<'_>) -> Poll<Option<(u32, Request, Reply)>> {
        if self.outstanding.is_empty() {
            return Poll::Ready(None);
        }
        // The waker is stored first, so a wake that comes while the work is
        // polled is not lost.
        let mut batch = {
            let mut woken = self.lock();
            woken.waiter = Some(context.waker().clone());
            std::mem::take(&mut woken.ready)
        };
        while let Some((id, sequence)) = batch.pop_front() {
            let Some(entry) = self.outstanding.get_mut(&id) else {
                continue;
            };
            if entry.sequence != sequence {
                continue;
            }
            entry.notify.queued.store(false, Ordering::Release);
            let waker = Waker::from(Arc::clone(&entry.notify));
            if let Poll::Ready(reply) = entry.work.as_mut().poll(&mut Context::from_waker(&waker)) {
                let entry = self
                    .outstanding
                    .remove(&id)
                    .expect("the entry was just polled");
                // The rest of the batch was woken before anything that
                // woke since, so it goes back first, in its order.
                if !batch.is_empty() {
                    let mut woken = self.lock();
                    batch.append(&mut woken.ready);
                    woken.ready = batch;
                }
                return Poll::Ready(Some((id, entry.request, reply)));
            }
        }
        Poll::Pending
    }

    /// The next request whose work is done, or `None` when nothing is
    /// outstanding.
    pub async fn next(&mut self) -> Option<(u32, Request, Reply)> {
        poll_fn(|context| self.poll_next(context)).await
    }

    /// A request whose work is done now, without waiting for one.
    pub fn ready(&mut self) -> Option<(u32, Request, Reply)> {
        match self.poll_next(&Context::from_waker(Waker::noop())) {
            Poll::Ready(done) => done,
            Poll::Pending => None,
        }
    }

    /// Drops the outstanding request `id` and its work, now (B.4).
    pub fn cancel(&mut self, admitted: bool, id: i32) -> Result<()> {
        if !admitted {
            bail!("the module cancelled request {id} outside an async call");
        }
        let entry = u32::try_from(id)
            .ok()
            .and_then(|id| self.outstanding.remove(&id));
        if entry.is_none() {
            bail!("the module cancelled request {id}, which is not outstanding");
        }
        Ok(())
    }

    /// Drops every request and its work, now.
    pub fn drop_all(&mut self) {
        let outstanding = std::mem::take(&mut self.outstanding);
        drop(outstanding);
        self.lock().ready.clear();
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Woken> {
        self.woken.lock().unwrap_or_else(|e| e.into_inner())
    }
}

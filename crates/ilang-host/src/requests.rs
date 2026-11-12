//! Outstanding ABI v3 requests. The table owns shared slots so cancellation
//! drops work immediately, even before its polling wrapper is polled again.

use std::{
    collections::HashMap,
    future::Future,
    pin::Pin,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    task::{Context, Poll, Waker},
};

use anyhow::{Result, bail};
use futures_util::{StreamExt, future::FutureExt, stream::FuturesUnordered};

use crate::{ServiceFuture, Signature, Value};

pub(crate) struct Request {
    pub namespace: String,
    pub label: String,
    pub signature: Signature,
}

pub(crate) type Reply = Result<std::result::Result<Value, Value>>;

struct Slot {
    work: Option<ServiceFuture>,
    waker: Option<Waker>,
}

type SharedSlot = Arc<Mutex<Slot>>;

struct Polling {
    id: u32,
    sequence: u64,
    slot: SharedSlot,
    epoch: Arc<AtomicU64>,
    last_poll: Option<u64>,
}

impl Future for Polling {
    type Output = (u32, u64, Option<Reply>);

    fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        let epoch = self.epoch.load(Ordering::Relaxed);
        let repeated = self.last_poll == Some(epoch);
        self.last_poll = Some(epoch);
        let mut slot = self.slot.lock().unwrap_or_else(|e| e.into_inner());
        if slot.work.is_none() {
            return Poll::Ready((self.id, self.sequence, None));
        }
        slot.waker = Some(context.waker().clone());
        // FuturesUnordered may revisit self-waking work within one poll.
        // Give the executor a turn before polling that work again.
        if repeated {
            context.waker().wake_by_ref();
            return Poll::Pending;
        }
        let work = slot.work.as_mut().expect("the work was checked above");
        match work.as_mut().poll(context) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(reply) => {
                slot.work.take();
                Poll::Ready((self.id, self.sequence, Some(reply)))
            }
        }
    }
}

#[derive(Default)]
pub(crate) struct Requests {
    accepting: bool,
    outstanding: HashMap<u32, (Request, u64, SharedSlot)>,
    work: FuturesUnordered<Polling>,
    next_sequence: u64,
    epoch: Arc<AtomicU64>,
}

impl Requests {
    /// Every import saves this flag, disables admission during its body,
    /// then restores the saved value.
    pub fn take_accepting(&mut self) -> bool {
        std::mem::replace(&mut self.accepting, false)
    }

    pub fn set_accepting(&mut self, value: bool) {
        self.accepting = value;
    }

    pub fn is_empty(&self) -> bool {
        self.outstanding.is_empty()
    }

    /// Checks, before the service's start closure is called, that a request
    /// may start now and that `id` is not outstanding.
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

    pub fn insert(&mut self, id: u32, request: Request, work: ServiceFuture) {
        let sequence = self.next_sequence;
        self.next_sequence = self.next_sequence.wrapping_add(1);
        let slot = Arc::new(Mutex::new(Slot {
            work: Some(work),
            waker: None,
        }));
        self.work.push(Polling {
            id,
            sequence,
            slot: Arc::clone(&slot),
            epoch: Arc::clone(&self.epoch),
            last_poll: None,
        });
        self.outstanding.insert(id, (request, sequence, slot));
    }

    fn poll_next(&mut self, context: &mut Context<'_>) -> Poll<Option<(u32, Request, Reply)>> {
        self.epoch.fetch_add(1, Ordering::Relaxed);
        while !self.outstanding.is_empty() {
            match self.work.poll_next_unpin(context) {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(None) => break,
                Poll::Ready(Some((id, sequence, reply))) => {
                    let Some(reply) = reply else { continue };
                    if !self
                        .outstanding
                        .get(&id)
                        .is_some_and(|(_, current, _)| *current == sequence)
                    {
                        continue;
                    }
                    let (request, _, _) = self.outstanding.remove(&id).expect("a live request");
                    return Poll::Ready(Some((id, request, reply)));
                }
            }
        }
        // No wrapper or stale answer survives an ended set of requests.
        self.work.clear();
        Poll::Ready(None)
    }

    pub async fn next(&mut self) -> Option<(u32, Request, Reply)> {
        std::future::poll_fn(|context| self.poll_next(context)).await
    }

    pub fn ready(&mut self) -> Option<(u32, Request, Reply)> {
        self.next().now_or_never().flatten()
    }

    pub fn cancel(&mut self, admitted: bool, id: i32) -> Result<()> {
        if !admitted {
            bail!("the module cancelled request {id} outside an async call");
        }
        let entry = u32::try_from(id)
            .ok()
            .and_then(|id| self.outstanding.remove(&id));
        let Some((_, _, slot)) = entry else {
            bail!("the module cancelled request {id}, which is not outstanding");
        };
        Self::drop_slot(&slot);
        Ok(())
    }

    fn drop_slot(slot: &SharedSlot) {
        let (work, waker) = {
            let mut slot = slot.lock().unwrap_or_else(|e| e.into_inner());
            (slot.work.take(), slot.waker.take())
        };
        // Drop outside the lock: a future's destructor may wake its wrapper.
        drop(work);
        if let Some(waker) = waker {
            waker.wake();
        }
    }

    pub fn drop_all(&mut self) {
        for (_, _, slot) in self.outstanding.values() {
            Self::drop_slot(slot);
        }
        self.outstanding.clear();
        self.work.clear();
    }
}

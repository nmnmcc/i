//! Run-owned resources and host-wide, non-reusing Int identities.
//!
//! Resource implementations own safe OS references. Cleanup starters must return
//! promptly, arrange any asynchronous cleanup independently of request futures,
//! and complete their receipt when cleanup ends. No callback runs under the table
//! lock. Dropping a close waiter never cancels cleanup.

use std::{
    any::Any,
    collections::BTreeMap,
    fmt,
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex, Weak},
    task::{Context, Poll, Waker},
};

use crate::BigInt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResourceError {
    Closed,
    Busy,
    Memory,
}

impl fmt::Display for ResourceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Closed => "closed",
            Self::Busy => "busy",
            Self::Memory => "memory exhausted",
        })
    }
}
impl std::error::Error for ResourceError {}

/// An isolated cleanup failure. It is reported to the embedding, never to i.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CleanupFailure {
    pub phase: &'static str,
    pub message: String,
}
type CleanupErrors = Arc<Mutex<Vec<CleanupFailure>>>;
fn isolate_cleanup(errors: &CleanupErrors, phase: &'static str, action: impl FnOnce()) {
    let mut failure = std::panic::catch_unwind(std::panic::AssertUnwindSafe(action)).err();
    while let Some(panic) = failure {
        let message = panic
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| panic.downcast_ref::<&str>().map(|s| (*s).to_owned()))
            .unwrap_or_else(|| "non-string panic".into());
        errors
            .lock()
            .unwrap()
            .push(CleanupFailure { phase, message });
        // Panic payload destructors are embedding callbacks too. Drop them
        // outside the diagnostic lock and isolate finite panic chains.
        failure = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| drop(panic))).err();
    }
}

/// Reuses this host across programs, instances and tests. Clones share identities.
#[derive(Clone)]
pub struct ResourceHost(Arc<Mutex<Allocator>>, CleanupErrors);
struct Allocator {
    next: BigInt,
    limit: Option<BigInt>,
}
impl Default for ResourceHost {
    fn default() -> Self {
        Self(
            Arc::new(Mutex::new(Allocator {
                next: 1.into(),
                limit: None,
            })),
            Default::default(),
        )
    }
}
impl ResourceHost {
    /// Drains this host's cleanup diagnostics; cleanup itself remains error-free.
    pub fn take_cleanup_errors(&self) -> Vec<CleanupFailure> {
        std::mem::take(&mut *self.1.lock().unwrap())
    }
    /// A small ceiling for deterministic exhaustion tests; never a wrap point.
    pub fn with_limit(limit: BigInt) -> Self {
        Self(
            Arc::new(Mutex::new(Allocator {
                next: 1.into(),
                limit: Some(limit),
            })),
            Default::default(),
        )
    }
    pub fn context(&self) -> ResourceContext {
        ResourceContext {
            host: self.clone(),
            table: Arc::new(Mutex::new(Table::default())),
        }
    }
    fn allocate(&self) -> Result<BigInt, ResourceError> {
        let mut allocator = self.0.lock().unwrap();
        if allocator
            .limit
            .as_ref()
            .is_some_and(|limit| allocator.next > *limit)
        {
            return Err(ResourceError::Memory);
        }
        let id = allocator.next.clone();
        allocator.next += 1;
        Ok(id)
    }
}

#[derive(Default)]
struct Signal {
    state: Mutex<Option<Vec<Waker>>>,
    errors: CleanupErrors,
}
impl Signal {
    fn pending(errors: CleanupErrors) -> Arc<Self> {
        Arc::new(Self {
            state: Mutex::new(Some(Vec::new())),
            errors,
        })
    }
    fn complete(&self) {
        let wakes = self.state.lock().unwrap().take().unwrap_or_default();
        for wake in wakes {
            isolate_cleanup(&self.errors, "wake", || wake.wake());
        }
    }
}
/// A shared, cancellation-independent cleanup completion receipt.
#[derive(Clone, Default)]
pub struct CloseReceipt(Arc<Signal>);
impl Future for CloseReceipt {
    type Output = ();
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        let mut signal = self.0.state.lock().unwrap();
        let Some(wakers) = signal.as_mut() else {
            return Poll::Ready(());
        };
        if !wakers.iter().any(|w| w.will_wake(cx.waker())) {
            wakers.push(cx.waker().clone());
        }
        Poll::Pending
    }
}
impl CloseReceipt {
    pub fn is_complete(&self) -> bool {
        self.0.state.lock().unwrap().is_none()
    }
}

/// Completion authority given to the independent cleanup job. Dropping it also
/// completes the receipt, including when a synchronous cleanup starter unwinds.
pub struct CleanupDone {
    receipt: CloseReceipt,
    table: Weak<Mutex<Table>>,
    id: BigInt,
}
impl Drop for CleanupDone {
    fn drop(&mut self) {
        if let Some(table) = self.table.upgrade() {
            let mut table = table.lock().unwrap();
            table.closing.remove(&self.id);
            table.jobs.remove(&self.id);
        }
        self.receipt.0.complete();
    }
}
impl CleanupDone {
    pub fn receipt(&self) -> CloseReceipt {
        self.receipt.clone()
    }
    pub fn complete(self) {}
}

type Cleanup = Box<dyn FnOnce(CleanupDone) + Send>;
struct Entry {
    kind: String,
    value: Arc<dyn Any + Send + Sync>,
    leases: BTreeMap<String, Arc<Signal>>,
    cleanup: Cleanup,
}
#[derive(Default)]
struct Table {
    ended: bool,
    entries: BTreeMap<BigInt, Entry>,
    closing: BTreeMap<BigInt, (String, CloseReceipt)>,
    jobs: BTreeMap<BigInt, CloseReceipt>,
    work: BTreeMap<usize, Arc<dyn CancelWork>>,
}

impl Drop for Table {
    fn drop(&mut self) {
        for work in std::mem::take(&mut self.work).into_values() {
            work.cancel();
        }
        for (id, entry) in std::mem::take(&mut self.entries) {
            for lease in entry.leases.into_values() {
                lease.complete();
            }
            (entry.cleanup)(CleanupDone {
                receipt: CloseReceipt::default(),
                table: Weak::new(),
                id,
            });
        }
    }
}

/// One immutable invocation identity, shared by all its workers and request roots.
#[derive(Clone)]
pub struct ResourceContext {
    host: ResourceHost,
    table: Arc<Mutex<Table>>,
}
impl ResourceContext {
    pub fn is_live(&self) -> bool {
        !self.table.lock().unwrap().ended
    }

    /// Registers a host-owned creation/cleanup job before starting OS work.
    /// Its independent owner keeps this token until even late-created resources
    /// have been cleaned. Teardown joins the receipt, not a request future.
    pub fn cleanup_job(&self) -> Result<CleanupDone, ResourceError> {
        let id = self.host.allocate()?;
        let mut table = self.table.lock().unwrap();
        if table.ended {
            return Err(ResourceError::Closed);
        }
        let receipt = CloseReceipt(Signal::pending(self.host.1.clone()));
        table.jobs.insert(id.clone(), receipt.clone());
        Ok(CleanupDone {
            receipt,
            table: Arc::downgrade(&self.table),
            id,
        })
    }

    /// Inserts before exposing a reply. On exhaustion or a late creation, cleanup
    /// still starts and owns the resource, but no number is exposed or reused.
    pub fn insert<T: Any + Send + Sync>(
        &self,
        kind: impl Into<String>,
        value: T,
        cleanup: impl FnOnce(Arc<T>, CleanupDone) + Send + 'static,
    ) -> Result<BigInt, ResourceError> {
        let value = Arc::new(value);
        let cleanup: Cleanup = Box::new({
            let value = value.clone();
            let errors = self.host.1.clone();
            move |done| isolate_cleanup(&errors, "cleanup", || cleanup(value, done))
        });
        let id = match self.host.allocate() {
            Ok(id) => id,
            Err(error) => {
                cleanup(CleanupDone {
                    receipt: CloseReceipt::default(),
                    table: Weak::new(),
                    id: 0.into(),
                });
                return Err(error);
            }
        };
        let mut table = self.table.lock().unwrap();
        if table.ended {
            drop(table);
            cleanup(CleanupDone {
                receipt: CloseReceipt::default(),
                table: Weak::new(),
                id,
            });
            return Err(ResourceError::Closed);
        }
        table.entries.insert(
            id.clone(),
            Entry {
                kind: kind.into(),
                value,
                leases: BTreeMap::new(),
                cleanup,
            },
        );
        Ok(id)
    }

    /// Acquiring a named lane checks both the resource kind and Rust type before
    /// downcasting. Use separate read/write lanes or one shared cursor lane.
    pub fn acquire<T: Any + Send + Sync>(
        &self,
        id: &BigInt,
        kind: &str,
        lane: &str,
    ) -> Result<Lease<T>, ResourceError> {
        let mut table = self.table.lock().unwrap();
        if table.ended {
            return Err(ResourceError::Closed);
        }
        let entry = table
            .entries
            .get_mut(id)
            .filter(|e| e.kind == kind)
            .ok_or(ResourceError::Closed)?;
        let value = entry
            .value
            .clone()
            .downcast::<T>()
            .map_err(|_| ResourceError::Closed)?;
        if entry.leases.contains_key(lane) {
            return Err(ResourceError::Busy);
        }
        let revoked = Signal::pending(self.host.1.clone());
        entry.leases.insert(lane.into(), revoked.clone());
        Ok(Lease {
            table: Arc::downgrade(&self.table),
            id: id.clone(),
            lane: lane.into(),
            revoked,
            value,
        })
    }

    /// Consumes an idle resource atomically, for operations such as upgrading a
    /// TCP stream. Existing operations make the upgrade busy; after success no
    /// new operation can acquire the old identity.
    pub fn take_idle<T: Any + Send + Sync>(
        &self,
        id: &BigInt,
        kind: &str,
    ) -> Result<Arc<T>, ResourceError> {
        let mut table = self.table.lock().unwrap();
        if table.ended {
            return Err(ResourceError::Closed);
        }
        let entry = table
            .entries
            .get(id)
            .filter(|e| e.kind == kind)
            .ok_or(ResourceError::Closed)?;
        let value = entry
            .value
            .clone()
            .downcast::<T>()
            .map_err(|_| ResourceError::Closed)?;
        if !entry.leases.is_empty() {
            return Err(ResourceError::Busy);
        }
        let (cleanup, done) = self
            .remove(&mut table, id, kind)
            .expect("idle resource exists");
        drop(table);
        cleanup(done);
        Ok(value)
    }

    fn remove(&self, table: &mut Table, id: &BigInt, kind: &str) -> Option<(Cleanup, CleanupDone)> {
        if !table
            .entries
            .get(id)
            .is_some_and(|entry| entry.kind == kind)
        {
            return None;
        }
        let entry = table.entries.remove(id).unwrap();
        // Revocation is visible under the same lock as removal. Wake outside it.
        let receipt = CloseReceipt(Signal::pending(self.host.1.clone()));
        table
            .closing
            .insert(id.clone(), (entry.kind, receipt.clone()));
        let cleanup = Box::new(move |done| {
            for signal in entry.leases.into_values() {
                signal.complete();
            }
            (entry.cleanup)(done);
        });
        Some((
            cleanup,
            CleanupDone {
                receipt,
                table: Arc::downgrade(&self.table),
                id: id.clone(),
            },
        ))
    }

    /// Unknown, foreign, wrong-kind and closed numbers are harmless. Concurrent
    /// closes of the right kind join the same receipt until cleanup completes.
    pub fn close(&self, id: &BigInt, kind: &str) -> CloseReceipt {
        let mut table = self.table.lock().unwrap();
        let work = self.remove(&mut table, id, kind);
        let receipt = table
            .closing
            .get(id)
            .filter(|(k, _)| k == kind)
            .map(|(_, r)| r.clone())
            .unwrap_or_default();
        drop(table);
        if let Some((cleanup, done)) = work {
            cleanup(done);
        }
        receipt
    }

    /// Closes admission synchronously and starts all cleanup outside the lock.
    /// The returned receipts include cleanup that was already in progress.
    pub fn teardown(&self) -> Vec<CloseReceipt> {
        let mut table = self.table.lock().unwrap();
        table.ended = true;
        let entries: Vec<_> = table
            .entries
            .iter()
            .map(|(id, e)| (id.clone(), e.kind.clone()))
            .collect();
        let work: Vec<_> = entries
            .into_iter()
            .filter_map(|(id, kind)| self.remove(&mut table, &id, &kind))
            .collect();
        let receipts = table
            .closing
            .values()
            .map(|(_, receipt)| receipt.clone())
            .chain(table.jobs.values().cloned())
            .collect();
        let pending_work = std::mem::take(&mut table.work);
        drop(table);
        for pending in pending_work.into_values() {
            pending.cancel();
        }
        for (cleanup, done) in work {
            cleanup(done);
        }
        receipts
    }
}

/// A safe resource reference plus a revocable operation lane. Commit is ordered
/// with close, so a late success cannot be published. Dropping releases the lane.
pub struct Lease<T> {
    table: Weak<Mutex<Table>>,
    id: BigInt,
    lane: String,
    revoked: Arc<Signal>,
    value: Arc<T>,
}
impl<T> Lease<T> {
    pub fn resource(&self) -> &Arc<T> {
        &self.value
    }
    pub fn closed(&self) -> CloseReceipt {
        CloseReceipt(self.revoked.clone())
    }
    pub fn commit<A>(self, answer: A) -> Result<A, ResourceError> {
        let table = self.table.upgrade().ok_or(ResourceError::Closed)?;
        let mut table = table.lock().unwrap();
        if table.ended {
            return Err(ResourceError::Closed);
        }
        let entry = table
            .entries
            .get_mut(&self.id)
            .ok_or(ResourceError::Closed)?;
        if !entry
            .leases
            .get(&self.lane)
            .is_some_and(|s| Arc::ptr_eq(s, &self.revoked))
        {
            return Err(ResourceError::Closed);
        }
        entry.leases.remove(&self.lane);
        Ok(answer)
    }
}
impl<T> Drop for Lease<T> {
    fn drop(&mut self) {
        if let Some(table) = self.table.upgrade() {
            let mut table = table.lock().unwrap();
            if let Some(entry) = table.entries.get_mut(&self.id)
                && entry
                    .leases
                    .get(&self.lane)
                    .is_some_and(|s| Arc::ptr_eq(s, &self.revoked))
            {
                entry.leases.remove(&self.lane);
            }
        }
    }
}

/// Owns an undelivered creator reply. Publication explicitly disarms this guard;
/// otherwise dropping the reply closes the resource.
pub struct ProvisionalResource {
    context: ResourceContext,
    id: BigInt,
    kind: String,
    delivered: bool,
}
impl ProvisionalResource {
    pub fn new(context: ResourceContext, id: BigInt, kind: impl Into<String>) -> Self {
        Self {
            context,
            id,
            kind: kind.into(),
            delivered: false,
        }
    }
    pub fn id(&self) -> &BigInt {
        &self.id
    }
    pub fn deliver(mut self) -> Result<BigInt, ResourceError> {
        let table = self.context.table.lock().unwrap();
        if table.ended {
            return Err(ResourceError::Closed);
        }
        if !table
            .entries
            .get(&self.id)
            .is_some_and(|e| e.kind == self.kind)
        {
            return Err(ResourceError::Closed);
        }
        self.delivered = true;
        Ok(self.id.clone())
    }
}
impl Drop for ProvisionalResource {
    fn drop(&mut self) {
        if !self.delivered {
            self.context.close(&self.id, &self.kind);
        }
    }
}
impl Drop for ResourceContext {
    fn drop(&mut self) {
        if Arc::strong_count(&self.table) == 1 {
            self.teardown();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn insert(context: &ResourceContext, closed: &Arc<AtomicUsize>) -> BigInt {
        let closed = closed.clone();
        context
            .insert("test", 42_u32, move |_, done| {
                closed.fetch_add(1, Ordering::SeqCst);
                done.complete();
            })
            .unwrap()
    }
    #[test]
    fn numbers_are_monotone_unbounded_and_never_reissued() {
        let host = ResourceHost::default();
        host.0.lock().unwrap().next = BigInt::from(1) << 200;
        let count = Arc::new(AtomicUsize::new(0));
        let first = host.context();
        let a = insert(&first, &count);
        first.close(&a, "test");
        let b = insert(&host.context(), &count);
        assert_eq!(b, a + 1);
        assert_eq!(count.load(Ordering::SeqCst), 2);
    }
    #[test]
    fn exhaustion_disposes_before_exposure_and_never_recycles() {
        let host = ResourceHost::with_limit(1.into());
        let context = host.context();
        let count = Arc::new(AtomicUsize::new(0));
        let id = insert(&context, &count);
        context.close(&id, "test");
        for _ in 0..2 {
            let count = count.clone();
            assert_eq!(
                context.insert("test", (), move |_, _| {
                    count.fetch_add(1, Ordering::SeqCst);
                }),
                Err(ResourceError::Memory)
            );
        }
        assert_eq!(count.load(Ordering::SeqCst), 3);
    }
    #[test]
    fn context_kind_type_lanes_and_stale_numbers_are_checked() {
        let host = ResourceHost::default();
        let context = host.context();
        let other = host.context();
        let id = insert(&context, &Arc::new(AtomicUsize::new(0)));
        for number in [0.into(), (-1).into(), 999.into()] {
            assert!(matches!(
                context.acquire::<u32>(&number, "test", "read"),
                Err(ResourceError::Closed)
            ));
            assert!(context.close(&number, "test").is_complete());
        }
        assert!(matches!(
            other.acquire::<u32>(&id, "test", "read"),
            Err(ResourceError::Closed)
        ));
        assert!(other.close(&id, "test").is_complete());
        assert!(matches!(
            context.acquire::<u32>(&id, "wrong", "read"),
            Err(ResourceError::Closed)
        ));
        assert!(matches!(
            context.acquire::<String>(&id, "test", "read"),
            Err(ResourceError::Closed)
        ));
        let read = context.acquire::<u32>(&id, "test", "read").unwrap();
        assert_eq!(**read.resource(), 42);
        assert!(matches!(
            context.acquire::<u32>(&id, "test", "read"),
            Err(ResourceError::Busy)
        ));
        context
            .acquire::<u32>(&id, "test", "write")
            .unwrap()
            .commit(())
            .unwrap();
        drop(read);
        context
            .acquire::<u32>(&id, "test", "read")
            .unwrap()
            .commit(7)
            .unwrap();
        context.close(&id, "test");
        assert!(matches!(
            context.acquire::<u32>(&id, "test", "read"),
            Err(ResourceError::Closed)
        ));
    }
    #[test]
    fn close_revokes_outstanding_work_and_late_success() {
        let context = ResourceHost::default().context();
        let count = Arc::new(AtomicUsize::new(0));
        let id = insert(&context, &count);
        let lease = context.acquire::<u32>(&id, "test", "read").unwrap();
        let wake = lease.closed();
        context.close(&id, "wrong");
        assert!(!wake.is_complete());
        context.close(&id, "test");
        assert!(wake.is_complete());
        assert_eq!(lease.commit(1), Err(ResourceError::Closed));
        assert!(context.close(&id, "test").is_complete());
        assert_eq!(count.load(Ordering::SeqCst), 1);
    }
    #[test]
    fn take_idle_requires_all_lanes_and_consumes_identity() {
        let context = ResourceHost::default().context();
        let id = insert(&context, &Arc::new(AtomicUsize::new(0)));
        let read = context.acquire::<u32>(&id, "test", "read").unwrap();
        assert!(matches!(
            context.take_idle::<u32>(&id, "test"),
            Err(ResourceError::Busy)
        ));
        drop(read);
        let write = context.acquire::<u32>(&id, "test", "write").unwrap();
        assert!(matches!(
            context.take_idle::<u32>(&id, "test"),
            Err(ResourceError::Busy)
        ));
        drop(write);
        assert_eq!(*context.take_idle::<u32>(&id, "test").unwrap(), 42);
        assert!(matches!(
            context.acquire::<u32>(&id, "test", "read"),
            Err(ResourceError::Closed)
        ));
    }
    #[test]
    fn concurrent_closes_and_teardown_join_cleanup_without_tombstones() {
        let context = ResourceHost::default().context();
        let pending = Arc::new(Mutex::new(None));
        let save = pending.clone();
        let id = context
            .insert("test", (), move |_, done| {
                *save.lock().unwrap() = Some(done)
            })
            .unwrap();
        let a = context.close(&id, "test");
        let b = context.close(&id, "test");
        assert!(Arc::ptr_eq(&a.0, &b.0));
        assert!(context.close(&id, "wrong").is_complete());
        assert_eq!(context.teardown().len(), 1);
        assert!(!a.is_complete());
        drop(a); // A cancelled close cannot cancel cleanup.
        pending.lock().unwrap().take().unwrap().complete();
        assert!(b.is_complete());
        assert!(context.table.lock().unwrap().closing.is_empty());
        assert!(context.teardown().is_empty());
        let disposed = Arc::new(AtomicUsize::new(0));
        let count = disposed.clone();
        assert_eq!(
            context.insert("test", (), move |_, _| {
                count.fetch_add(1, Ordering::SeqCst);
            }),
            Err(ResourceError::Closed)
        );
        assert_eq!(disposed.load(Ordering::SeqCst), 1);
    }
    #[test]
    fn provisional_and_delivered_values_keep_context_ownership() {
        let context = ResourceHost::default().context();
        let count = Arc::new(AtomicUsize::new(0));
        let id = insert(&context, &count);
        drop(ProvisionalResource::new(context.clone(), id, "test"));
        assert_eq!(count.load(Ordering::SeqCst), 1);
        let id = insert(&context, &count);
        ProvisionalResource::new(context.clone(), id, "test")
            .deliver()
            .unwrap();
        context.teardown();
        assert_eq!(count.load(Ordering::SeqCst), 2);
    }
    #[test]
    fn concurrent_runs_share_only_the_allocator() {
        let host = ResourceHost::default();
        std::thread::scope(|scope| {
            let a = host.context();
            let b = host.context();
            let id_a = insert(&a, &Arc::new(AtomicUsize::new(0)));
            let id_b = insert(&b, &Arc::new(AtomicUsize::new(0)));
            assert_ne!(id_a, id_b);
            scope.spawn(move || {
                a.teardown();
            });
            scope.spawn(move || {
                b.acquire::<u32>(&id_b, "test", "read")
                    .unwrap()
                    .commit(())
                    .unwrap();
            });
        });
    }
}

#[cfg(test)]
#[test]
fn teardown_joins_a_creation_job_before_a_resource_exists() {
    let context = ResourceHost::default().context();
    let job = context.cleanup_job().unwrap();
    let receipt = job.receipt();
    assert_eq!(context.teardown().len(), 1);
    assert!(!receipt.is_complete());
    assert!(matches!(context.cleanup_job(), Err(ResourceError::Closed)));
    let done = Arc::new(Mutex::new(None));
    let save = done.clone();
    assert_eq!(
        context.insert("late", (), move |_, cleanup| *save.lock().unwrap() =
            Some((job, cleanup))),
        Err(ResourceError::Closed)
    );
    assert!(!receipt.is_complete());
    drop(done.lock().unwrap().take());
    assert!(receipt.is_complete());
    assert!(context.teardown().is_empty());
}

trait CancelWork: Send + Sync {
    fn cancel(&self);
}
struct PendingWork<T> {
    future: Option<Pin<Box<dyn Future<Output = T> + Send>>>,
    wake: Option<Waker>,
    cancelled: bool,
}
struct FutureCell<T> {
    pending: Mutex<PendingWork<T>>,
    errors: CleanupErrors,
}
impl<T> CancelWork for FutureCell<T> {
    fn cancel(&self) {
        let (future, wake) = {
            let mut pending = self.pending.lock().unwrap();
            pending.cancelled = true;
            (pending.future.take(), pending.wake.take())
        };
        // Destructors and poll callbacks never run under a host lock. Work
        // already inside poll sees cancellation when that finite poll returns.
        isolate_cleanup(&self.errors, "cancel", || drop(future));
        if let Some(wake) = wake {
            isolate_cleanup(&self.errors, "wake", || wake.wake());
        }
    }
}
/// Request work owned independently of a Store. Teardown drops its underlying
/// future immediately, even if no worker ever polls this wrapper again.
pub struct ResourceFuture<T> {
    cell: Arc<FutureCell<T>>,
    table: Weak<Mutex<Table>>,
    key: usize,
}
impl<T> Future for ResourceFuture<T> {
    type Output = Option<T>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut future = {
            let mut pending = self.cell.pending.lock().unwrap();
            if pending.cancelled {
                return Poll::Ready(None);
            }
            pending.wake = Some(cx.waker().clone());
            let Some(future) = pending.future.take() else {
                return Poll::Ready(None);
            };
            future
        };
        let result = future.as_mut().poll(cx);
        {
            let mut pending = self.cell.pending.lock().unwrap();
            if pending.cancelled {
                drop(pending);
                isolate_cleanup(&self.cell.errors, "cancel", || drop(future));
                return Poll::Ready(None);
            }
            if result.is_pending() {
                pending.future = Some(future);
                return Poll::Pending;
            }
            pending.wake = None;
        }
        isolate_cleanup(&self.cell.errors, "cancel", || drop(future));
        if let Some(table) = self.table.upgrade() {
            table.lock().unwrap().work.remove(&self.key);
        }
        result.map(Some)
    }
}
impl<T> Drop for ResourceFuture<T> {
    fn drop(&mut self) {
        if let Some(table) = self.table.upgrade() {
            table.lock().unwrap().work.remove(&self.key);
        }
        self.cell.cancel();
    }
}
impl ResourceContext {
    pub fn own_future<T: Send + 'static>(
        &self,
        future: impl Future<Output = T> + Send + 'static,
    ) -> Result<ResourceFuture<T>, ResourceError> {
        let cell = Arc::new(FutureCell {
            pending: Mutex::new(PendingWork {
                future: Some(Box::pin(future)),
                wake: None,
                cancelled: false,
            }),
            errors: self.host.1.clone(),
        });
        let key = Arc::as_ptr(&cell) as usize;
        let mut table = self.table.lock().unwrap();
        if table.ended {
            drop(table);
            cell.cancel();
            return Err(ResourceError::Closed);
        }
        table.work.insert(key, cell.clone());
        Ok(ResourceFuture {
            cell,
            table: Arc::downgrade(&self.table),
            key,
        })
    }
}

#[cfg(test)]
#[test]
fn controller_drops_request_work_without_a_store_or_another_poll() {
    use std::sync::atomic::{AtomicBool, Ordering};
    struct Dropped(Arc<AtomicBool>);
    impl Drop for Dropped {
        fn drop(&mut self) {
            self.0.store(true, Ordering::SeqCst);
        }
    }
    let context = ResourceHost::default().context();
    let dropped = Arc::new(AtomicBool::new(false));
    let guard = Dropped(dropped.clone());
    let work = context
        .own_future(async move {
            let _guard = guard;
            std::future::pending::<()>().await;
        })
        .unwrap();
    context.teardown();
    assert!(dropped.load(Ordering::SeqCst));
    assert!(crate::block_on(work).is_none());
}

#[cfg(test)]
#[test]
fn a_host_future_can_reenter_its_controller_without_a_lock_cycle() {
    let context = ResourceHost::default().context();
    let controller = context.clone();
    let work = context
        .own_future(async move {
            controller.teardown();
            42
        })
        .unwrap();
    assert_eq!(crate::block_on(work), None);
}

#[cfg(test)]
mod bg_review_tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct OnDrop(Option<Box<dyn FnOnce() + Send>>);
    impl Drop for OnDrop {
        fn drop(&mut self) {
            self.0.take().unwrap()();
        }
    }
    #[test]
    fn bg_review_reentrant_cancellation_cannot_publish_after_teardown() {
        let host = ResourceHost::default();
        let context = host.context();
        let id = context.insert("file", (), |_, _| {}).unwrap();
        let lease = context.acquire::<()>(&id, "file", "read").unwrap();
        let provisional = ProvisionalResource::new(context.clone(), id.clone(), "file");
        let controller = context.clone();
        let checked = Arc::new(AtomicUsize::new(0));
        let done = checked.clone();
        let guard = OnDrop(Some(Box::new(move || {
            assert!(!controller.is_live());
            assert!(matches!(
                controller.acquire::<()>(&id, "file", "write"),
                Err(ResourceError::Closed)
            ));
            assert_eq!(lease.commit(42), Err(ResourceError::Closed));
            assert_eq!(provisional.deliver(), Err(ResourceError::Closed));
            let receipts = controller.teardown();
            assert_eq!(receipts.len(), 1);
            assert!(!receipts[0].is_complete());
            done.fetch_add(1, Ordering::SeqCst);
        })));
        let _work = context
            .own_future(async move {
                let _guard = guard;
                std::future::pending::<()>().await;
            })
            .unwrap();
        let receipts = context.teardown();
        assert_eq!(checked.load(Ordering::SeqCst), 1);
        assert!(receipts.iter().all(CloseReceipt::is_complete));
        assert!(host.take_cleanup_errors().is_empty());
    }

    #[test]
    fn bg_review_cleanup_panics_do_not_skip_cancellation_wakes_or_cleanup() {
        let host = ResourceHost::default();
        let context = host.context();
        let cancelled = Arc::new(AtomicUsize::new(0));
        let mut work = Vec::new();
        for panic in [true, false] {
            let count = cancelled.clone();
            let guard = OnDrop(Some(Box::new(move || {
                count.fetch_add(1, Ordering::SeqCst);
                if panic {
                    panic!("cancel failed");
                }
            })));
            work.push(
                context
                    .own_future(async move {
                        let _guard = guard;
                        std::future::pending::<()>().await;
                    })
                    .unwrap(),
            );
        }
        let cleaned = Arc::new(AtomicUsize::new(0));
        let wakes = Arc::new(AtomicUsize::new(0));
        struct Wake {
            panic: bool,
            count: Arc<AtomicUsize>,
        }
        impl std::task::Wake for Wake {
            fn wake(self: Arc<Self>) {
                if self.panic {
                    panic!("wake failed");
                }
                self.count.fetch_add(1, Ordering::SeqCst);
            }
        }
        let mut leases = Vec::new();
        for panic in [true, false] {
            let count = cleaned.clone();
            let id = context
                .insert("file", (), move |_, _| {
                    count.fetch_add(1, Ordering::SeqCst);
                    if panic {
                        panic!("cleanup failed");
                    }
                })
                .unwrap();
            let lease = context.acquire::<()>(&id, "file", "read").unwrap();
            let waker = Waker::from(Arc::new(Wake {
                panic,
                count: wakes.clone(),
            }));
            assert!(
                Pin::new(&mut lease.closed())
                    .poll(&mut Context::from_waker(&waker))
                    .is_pending()
            );
            leases.push(lease);
        }
        let receipts = context.teardown();
        assert_eq!(cancelled.load(Ordering::SeqCst), 2);
        assert_eq!(cleaned.load(Ordering::SeqCst), 2);
        assert_eq!(wakes.load(Ordering::SeqCst), 1);
        assert!(receipts.iter().all(CloseReceipt::is_complete));
        let errors = host.take_cleanup_errors();
        for phase in ["cancel", "wake", "cleanup"] {
            assert_eq!(errors.iter().filter(|e| e.phase == phase).count(), 1);
        }
        assert!(host.take_cleanup_errors().is_empty());
        assert_eq!(
            context.insert("late", (), |_, _| panic!("late cleanup failed")),
            Err(ResourceError::Closed)
        );
        assert_eq!(host.take_cleanup_errors()[0].message, "late cleanup failed");
    }
}

#[cfg(test)]
#[test]
fn bg_review_a_panicking_panic_payload_cannot_skip_later_cleanup() {
    struct Payload;
    impl Drop for Payload {
        fn drop(&mut self) {
            panic!("panic payload destructor failed");
        }
    }
    let host = ResourceHost::default();
    let context = host.context();
    context
        .insert("file", (), |_, _| std::panic::panic_any(Payload))
        .unwrap();
    let cleaned = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let mark = cleaned.clone();
    context
        .insert("file", (), move |_, _| {
            mark.store(true, std::sync::atomic::Ordering::SeqCst);
        })
        .unwrap();
    let receipts = context.teardown();
    assert!(cleaned.load(std::sync::atomic::Ordering::SeqCst));
    assert!(receipts.iter().all(CloseReceipt::is_complete));
    let errors = host.take_cleanup_errors();
    assert_eq!(errors.len(), 2);
    assert_eq!(errors[1].message, "panic payload destructor failed");
}

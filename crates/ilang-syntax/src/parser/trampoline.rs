//! Stack of scoped parser continuations. A caller is never polled while its
//! child runs, so the child may reborrow the parser without retaining a native
//! stack frame for its caller.
use std::cell::RefCell;
use std::future::{Future, poll_fn};
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

type Task = Pin<Box<dyn Future<Output = ()> + 'static>>;

pub(super) struct Scheduler(Rc<RefCell<Vec<Task>>>);
impl Clone for Scheduler {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
impl Scheduler {
    /// Calls a child and suspends the caller until the child has completed.
    ///
    /// # Safety
    /// The future must be awaited directly by a continuation driven by `run`,
    /// without cancellation, and its result must not borrow from the child.
    pub(super) async unsafe fn call<'b, T: 'b, F, Fut>(&'b self, f: F) -> T
    where
        F: FnOnce(Self) -> Fut + 'b,
        Fut: Future<Output = T> + 'b,
    {
        let result = Rc::new(RefCell::new(None));
        let target = result.clone();
        let scheduler = self.clone();
        let task: Pin<Box<dyn Future<Output = ()> + 'b>> =
            Box::pin(async move { *target.borrow_mut() = Some(f(scheduler).await) });
        // SAFETY: `run` is the only executor. It retains the suspended caller
        // below this child and always drops the child before re-polling or
        // dropping the caller, including during unwinding. Neither parser
        // futures nor this scheduler escape `run`; a parser continuation can
        // only await this call directly and cannot cancel it.
        let task: Task = unsafe { std::mem::transmute(task) };
        self.0.borrow_mut().push(task);
        poll_fn(move |_| match result.borrow_mut().take() {
            Some(value) => Poll::Ready(value),
            None => Poll::Pending,
        })
        .await
    }
}
/// Drop suspended child continuations before their borrowing parents.
struct Stack(Vec<Task>);
impl Drop for Stack {
    fn drop(&mut self) {
        while let Some(task) = self.0.pop() {
            drop_task(task);
        }
    }
}

// If a destructor panics, allowing Vec to drop the remaining ancestors in
// unspecified order could invalidate their borrowed children. Abort instead.
fn drop_task(task: Task) {
    if std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| drop(task))).is_err() {
        std::process::abort();
    }
}

/// Drives only parser continuations which directly await `call` and do not
/// retain the scheduler or references into the parser in the returned value.
///
/// # Safety
/// Every child must satisfy `call`'s scoped-borrow contract; no continuation
/// may cancel a child or let the scheduler escape this invocation.
pub(super) unsafe fn run<'a, T: 'a, F, Fut>(f: F) -> T
where
    F: FnOnce(Scheduler) -> Fut + 'a,
    Fut: Future<Output = T> + 'a,
{
    let scheduler = Scheduler(Rc::new(RefCell::new(Vec::new())));
    let result = Rc::new(RefCell::new(None));
    let target = result.clone();
    let root_scheduler = scheduler.clone();
    let root: Pin<Box<dyn Future<Output = ()> + 'a>> = Box::pin(async move {
        *target.borrow_mut() = Some(f(root_scheduler).await);
    });
    // SAFETY: `run` does not return until this root future has finished, and
    // all children and the root are dropped before any borrowed input expires.
    let root: Task = unsafe { std::mem::transmute(root) };
    let mut stack = Stack(vec![root]);
    let mut cx = Context::from_waker(Waker::noop());
    while let Some(mut task) = stack.0.pop() {
        let polled =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| task.as_mut().poll(&mut cx)));
        match polled {
            Ok(Poll::Ready(())) => {}
            Ok(Poll::Pending) => {
                let mut children = std::mem::take(&mut *scheduler.0.borrow_mut());
                assert_eq!(
                    children.len(),
                    1,
                    "parser continuation must suspend on one child"
                );
                stack.0.push(task);
                stack.0.push(children.pop().unwrap());
            }
            Err(payload) => {
                let children = Stack(std::mem::take(&mut *scheduler.0.borrow_mut()));
                drop(children);
                drop_task(task);
                drop(stack);
                std::panic::resume_unwind(payload);
            }
        }
    }
    Rc::try_unwrap(result)
        .ok()
        .expect("parser result still shared")
        .into_inner()
        .expect("parser did not return")
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Counter(usize);
    impl Counter {
        async fn nested(&mut self, scheduler: Scheduler, remaining: usize) {
            if remaining != 0 {
                unsafe { scheduler.call(|s| self.nested(s, remaining - 1)) }.await;
            }
            self.0 += 1;
        }
    }
    #[test]
    fn scoped_reborrows_are_resumed_without_native_recursion() {
        std::thread::Builder::new()
            .stack_size(256 * 1024)
            .spawn(|| {
                let mut value = Counter(0);
                unsafe { run(|s| value.nested(s, 10_000)) };
                assert_eq!(value.0, 10_001);
            })
            .unwrap()
            .join()
            .unwrap();
    }
    #[test]
    fn unwind_drops_children_before_borrowing_parents() {
        struct Mark {
            depth: usize,
            dropped: Rc<RefCell<Vec<usize>>>,
        }
        impl Drop for Mark {
            fn drop(&mut self) {
                self.dropped.borrow_mut().push(self.depth);
            }
        }
        async fn descend(s: Scheduler, depth: usize, dropped: Rc<RefCell<Vec<usize>>>) {
            let _mark = Mark {
                depth,
                dropped: dropped.clone(),
            };
            if depth == 0 {
                panic!("expected child failure");
            }
            unsafe { s.call(|s| descend(s, depth - 1, dropped)) }.await;
        }
        let dropped = Rc::new(RefCell::new(Vec::new()));
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            unsafe { run(|s| descend(s, 1_000, dropped.clone())) };
        }));
        assert!(result.is_err());
        assert_eq!(*dropped.borrow(), (0..=1_000).collect::<Vec<_>>());
    }
    #[test]
    fn child_borrows_data_owned_by_suspended_parent() {
        struct ReadOnDrop<'a>(&'a str, Rc<RefCell<Vec<String>>>);
        impl Drop for ReadOnDrop<'_> {
            fn drop(&mut self) {
                self.1.borrow_mut().push(self.0.to_owned());
            }
        }
        async fn child(s: Scheduler, value: &str, observed: Rc<RefCell<Vec<String>>>, fail: bool) {
            let _read = ReadOnDrop(value, observed);
            unsafe { s.call(|_| async move { assert!(!fail, "expected failure") }) }.await;
        }
        for fail in [false, true] {
            let observed = Rc::new(RefCell::new(Vec::new()));
            let child_observed = observed.clone();
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| unsafe {
                run(|s| async move {
                    let owned = String::from("alive");
                    s.call(|s| child(s, &owned, child_observed, fail)).await;
                })
            }));
            assert_eq!(result.is_err(), fail);
            assert_eq!(&*observed.borrow(), &["alive"]);
        }
    }
    #[test]
    #[cfg(unix)]
    fn panic_in_cleanup_aborts_before_dropping_ancestors() {
        use std::os::unix::process::ExitStatusExt;
        const CHILD: &str = "ILANG_TRAMPOLINE_ABORT_CHILD";
        if std::env::var_os(CHILD).is_some() {
            struct Bomb;
            impl Drop for Bomb {
                fn drop(&mut self) {
                    panic!("destructor failed");
                }
            }
            unsafe {
                run(|s| async move {
                    s.call(|s| async move {
                        let _bomb = Bomb;
                        s.call(|_| async { panic!("poll failed") }).await;
                    })
                    .await;
                });
            }
            panic!("cleanup unexpectedly returned");
        }
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "parser::trampoline::tests::panic_in_cleanup_aborts_before_dropping_ancestors",
            ])
            .env(CHILD, "1")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .unwrap();
        assert_eq!(status.signal(), Some(6));
    }
}

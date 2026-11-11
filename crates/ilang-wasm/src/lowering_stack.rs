//! Stack of scoped lowering continuations. A caller is never polled while its
//! child runs, so the child may reborrow lowering state without retaining a native
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
        // dropping the caller, including during unwinding. Neither lowering
        // futures nor this scheduler escape `run`; a lowering continuation can
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
        while self.0.pop().is_some() {}
    }
}

pub(super) fn run<'a, T: 'a, F, Fut>(f: F) -> T
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
                    "lowering continuation must suspend on one child"
                );
                stack.0.push(task);
                stack.0.push(children.pop().unwrap());
            }
            Err(payload) => {
                let mut children = std::mem::take(&mut *scheduler.0.borrow_mut());
                while children.pop().is_some() {}
                drop(task);
                drop(stack);
                std::panic::resume_unwind(payload);
            }
        }
    }
    Rc::try_unwrap(result)
        .ok()
        .expect("lowering result still shared")
        .into_inner()
        .expect("lowering did not return")
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
                run(|s| value.nested(s, 10_000));
                assert_eq!(value.0, 10_001);
            })
            .unwrap()
            .join()
            .unwrap();
    }
    #[test]
    fn unwinding_drops_child_before_its_borrowing_parent() {
        struct Guard(&'static str, Rc<RefCell<Vec<&'static str>>>);
        impl Drop for Guard {
            fn drop(&mut self) {
                self.1.borrow_mut().push(self.0);
            }
        }
        let drops = Rc::new(RefCell::new(Vec::new()));
        let observed = drops.clone();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            run(|scheduler| async move {
                let parent = Guard("parent", observed.clone());
                unsafe {
                    scheduler.call(|_| async {
                        let _borrow = &parent;
                        let _child = Guard("child", observed.clone());
                        panic!("test unwind");
                    })
                }
                .await;
            });
        }));
        assert!(result.is_err());
        assert_eq!(&*drops.borrow(), &["child", "parent"]);
    }
}

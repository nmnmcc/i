//! A work-stack executor for mutually recursive datum conversions.
use std::cell::RefCell;
use std::future::{Future, poll_fn};
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};

type Task<'a> = Pin<Box<dyn Future<Output = ()> + 'a>>;

pub(super) struct Scheduler<'a>(Rc<RefCell<Vec<Task<'a>>>>);
impl Clone for Scheduler<'_> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
impl<'a> Scheduler<'a> {
    pub(super) async fn call<T: 'a, F, Fut>(&self, f: F) -> T
    where
        F: FnOnce(Self) -> Fut + 'a,
        Fut: Future<Output = T> + 'a,
    {
        let result = Rc::new(RefCell::new(None));
        let target = result.clone();
        let scheduler = self.clone();
        self.0.borrow_mut().push(Box::pin(async move {
            *target.borrow_mut() = Some(f(scheduler).await);
        }));
        poll_fn(move |_| match result.borrow_mut().take() {
            Some(value) => Poll::Ready(value),
            None => Poll::Pending,
        })
        .await
    }
}
struct Noop;
impl Wake for Noop {
    fn wake(self: Arc<Self>) {}
}

pub(super) fn run<'a, T: 'a, F, Fut>(f: F) -> T
where
    F: FnOnce(Scheduler<'a>) -> Fut + 'a,
    Fut: Future<Output = T> + 'a,
{
    let scheduler = Scheduler(Rc::new(RefCell::new(Vec::new())));
    let result = Rc::new(RefCell::new(None));
    let target = result.clone();
    let root_scheduler = scheduler.clone();
    let mut stack: Vec<Task<'a>> = vec![Box::pin(async move {
        *target.borrow_mut() = Some(f(root_scheduler).await);
    })];
    let waker = Waker::from(Arc::new(Noop));
    let mut cx = Context::from_waker(&waker);
    while let Some(mut task) = stack.pop() {
        if task.as_mut().poll(&mut cx).is_pending() {
            let children = std::mem::take(&mut *scheduler.0.borrow_mut());
            assert!(
                !children.is_empty(),
                "datum conversion suspended without a child"
            );
            stack.push(task);
            stack.extend(children.into_iter().rev());
        }
    }
    Rc::try_unwrap(result)
        .ok()
        .expect("datum conversion result still shared")
        .into_inner()
        .expect("datum conversion did not return")
}

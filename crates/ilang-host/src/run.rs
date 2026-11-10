//! The thread that runs a program: one `ilang` thread with a big stack and
//! a tokio `current_thread` runtime, for each call of [`block_on`].

use std::{cell::Cell, future::Future};

use crate::THREAD_STACK;

thread_local! {
    /// Set on the threads that `block_on` starts.
    static ON_ILANG_THREAD: Cell<bool> = const { Cell::new(false) };
    /// The Wasm calls of the fast path that are running on this thread.
    static WASM_DEPTH: Cell<u32> = const { Cell::new(0) };
}

/// Shuts the runtime down without waiting for its blocking pool, also when
/// the future panics. Dropping a `Runtime` normally waits, with no time
/// limit, for every `spawn_blocking` task.
struct Background(Option<tokio::runtime::Runtime>);

impl Drop for Background {
    fn drop(&mut self) {
        if let Some(runtime) = self.0.take() {
            runtime.shutdown_background();
        }
    }
}

/// Run a future to its end on a new thread named `ilang`, with a stack of
/// [`THREAD_STACK`] bytes and a tokio `current_thread` runtime with time
/// and IO enabled. IO is there whenever tokio is built with its IO driver,
/// which a service that uses tokio IO turns on through its own tokio
/// features (for example `net`). Every Wasm call that the future makes runs directly on that
/// thread, so a whole program uses one thread. A panic of the future is
/// resumed on the caller. When the future is done, the runtime is shut down
/// without waiting for work that a service handed to `spawn_blocking`.
///
/// It also works when it is called from inside a tokio runtime, because it
/// starts a thread of its own. It needs a `Send` future; with a future that
/// is not `Send`, await [`Instance::call_async`](crate::Instance::call_async)
/// on your own executor.
pub fn block_on<F>(future: F) -> F::Output
where
    F: Future + Send,
    F::Output: Send,
{
    std::thread::scope(|scope| {
        let thread = std::thread::Builder::new()
            .name("ilang".into())
            .stack_size(THREAD_STACK)
            .spawn_scoped(scope, || {
                ON_ILANG_THREAD.set(true);
                let runtime = Background(Some(
                    tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .expect("a tokio runtime"),
                ));
                runtime
                    .0
                    .as_ref()
                    .expect("the runtime is there until the end")
                    .block_on(future)
            })
            .expect("a thread for i");
        thread
            .join()
            .unwrap_or_else(|panic| std::panic::resume_unwind(panic))
    })
}

/// Whether a Wasm call may run directly on this thread: on the `ilang`
/// thread, and not inside another Wasm call (so not from a host function).
/// A store entered from inside a host function gets its own Wasm stack
/// limit, so it needs a native stack of its own.
pub(crate) fn fast_path() -> bool {
    ON_ILANG_THREAD.get() && WASM_DEPTH.get() == 0
}

/// Runs `call` with the Wasm depth of this thread raised by one, also when
/// it panics.
pub(crate) fn in_wasm<T>(call: impl FnOnce() -> T) -> T {
    struct Depth;
    impl Drop for Depth {
        fn drop(&mut self) {
            WASM_DEPTH.set(WASM_DEPTH.get() - 1);
        }
    }
    WASM_DEPTH.set(WASM_DEPTH.get() + 1);
    let _depth = Depth;
    call()
}

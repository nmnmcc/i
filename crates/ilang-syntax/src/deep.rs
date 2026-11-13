//! Deep trees. A tree made by a macro can be much deeper than the host
//! stack, and the passes over the tree of syntax three are plain recursive
//! functions. [`deep`] runs such a pass on a stack that is large enough;
//! dropping a tree needs no care, because `Expr`, `Pattern` and `Type` drop
//! their children with a work list (`ast.rs`).

use std::cell::Cell;

/// The reserved stack of a deep pass. The pages are used only as far as the
/// recursion goes.
const STACK: usize = 4 << 30;

thread_local! {
    static DEEP: Cell<bool> = const { Cell::new(false) };
}

/// Runs `f` on a stack large enough for any tree a macro can build within
/// its budget. A call inside another `deep` call runs `f` directly.
pub fn deep<T: Send>(f: impl FnOnce() -> T + Send) -> T {
    if DEEP.with(Cell::get) {
        return f();
    }
    std::thread::scope(|scope| {
        let handle = std::thread::Builder::new()
            .stack_size(STACK)
            .spawn_scoped(scope, || {
                DEEP.with(|d| d.set(true));
                f()
            })
            .expect("a thread for a deep syntax pass");
        match handle.join() {
            Ok(value) => value,
            Err(panic) => std::panic::resume_unwind(panic),
        }
    })
}

//! Machine-independent accounting for one compiler macro invocation.
//!
//! The Wasm accounting hooks and the compiler's Syntax services share one
//! handle. An exhausted budget is a host failure, never an i error value.

use std::fmt;
use std::sync::{Arc, Mutex};

pub const WORK_LIMIT: u64 = 100_000_000;
pub const ALLOCATION_LIMIT: u64 = 1 << 28;
pub const FRAME_LIMIT: u64 = 10_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Limit {
    Work,
    Allocation,
    Frames,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Exceeded(pub Limit);

impl fmt::Display for Exceeded {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self.0 {
            Limit::Work => "work",
            Limit::Allocation => "allocation",
            Limit::Frames => "frame",
        };
        write!(f, "macro {name} limit exceeded")
    }
}

impl std::error::Error for Exceeded {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Limits {
    pub work: u64,
    pub allocation: u64,
    pub frames: u64,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            work: WORK_LIMIT,
            allocation: ALLOCATION_LIMIT,
            frames: FRAME_LIMIT,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Usage {
    pub work: u64,
    pub allocation: u64,
    pub frames: u64,
}

#[derive(Debug)]
struct State {
    limits: Limits,
    usage: Usage,
    frames: Vec<u64>,
    exceeded: Option<Limit>,
}

/// One budget per invocation. Clones share counters with compiler services.
#[derive(Clone, Debug)]
pub struct MacroBudget(Arc<Mutex<State>>);

impl Default for MacroBudget {
    fn default() -> Self {
        Self::new(Limits::default())
    }
}

impl MacroBudget {
    pub(crate) fn same_instance(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
    /// Limits are selected by the compiler; packages cannot override them.
    pub fn new(limits: Limits) -> Self {
        Self(Arc::new(Mutex::new(State {
            limits,
            usage: Usage::default(),
            frames: Vec::new(),
            exceeded: None,
        })))
    }

    pub fn usage(&self) -> Usage {
        self.0.lock().expect("macro budget lock poisoned").usage
    }

    pub fn exceeded(&self) -> Option<Limit> {
        self.0.lock().expect("macro budget lock poisoned").exceeded
    }

    /// Charge before doing the corresponding operation or allocating memory.
    pub fn charge(&self, work: u64, allocation: u64) -> Result<(), Exceeded> {
        let mut s = self.0.lock().expect("macro budget lock poisoned");
        if let Some(limit) = s.exceeded {
            return Err(Exceeded(limit));
        }
        let w = s.usage.work.checked_add(work);
        let a = s.usage.allocation.checked_add(allocation);
        let failure = if w.is_none_or(|v| v > s.limits.work) {
            Some(Limit::Work)
        } else if a.is_none_or(|v| v > s.limits.allocation) {
            Some(Limit::Allocation)
        } else {
            None
        };
        if let Some(limit) = failure {
            s.exceeded = Some(limit);
            return Err(Exceeded(limit));
        }
        s.usage.work = w.expect("checked work");
        s.usage.allocation = a.expect("checked allocation");
        Ok(())
    }

    /// Called on entering an i frame before executing its body.
    pub fn enter_frame(&self, units: u64) -> Result<(), Exceeded> {
        let mut s = self.0.lock().expect("macro budget lock poisoned");
        if let Some(limit) = s.exceeded {
            return Err(Exceeded(limit));
        }
        let next = s.usage.frames.checked_add(units);
        if next.is_none_or(|n| n > s.limits.frames) {
            s.exceeded = Some(Limit::Frames);
            return Err(Exceeded(Limit::Frames));
        }
        s.usage.frames = next.expect("checked frame units");
        s.frames.push(units);
        Ok(())
    }

    pub fn leave_frame(&self) {
        assert!(self.try_leave_frame(), "macro frame stack underflow");
    }

    pub(crate) fn try_leave_frame(&self) -> bool {
        let mut s = self.0.lock().expect("macro budget lock poisoned");
        let Some(units) = s.frames.pop() else {
            return false;
        };
        s.usage.frames -= units;
        true
    }

    /// Unwinding a handler restores the saved shadow-stack depth.
    pub fn restore_frames(&self, depth: u64) {
        assert!(
            self.try_restore_frames(depth),
            "macro frame restore grows the stack"
        );
    }

    pub(crate) fn try_restore_frames(&self, depth: u64) -> bool {
        let mut s = self.0.lock().expect("macro budget lock poisoned");
        if depth > s.frames.len() as u64 {
            return false;
        }
        while s.frames.len() > depth as usize {
            let units = s.frames.pop().expect("macro frame stack underflow");
            s.usage.frames -= units;
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn independent_budgets_share_only_their_own_clones() {
        let a = MacroBudget::new(Limits {
            work: 2,
            allocation: 3,
            frames: 2,
        });
        let b = MacroBudget::default();
        a.clone().charge(2, 3).unwrap();
        assert_eq!(
            a.usage(),
            Usage {
                work: 2,
                allocation: 3,
                frames: 0
            }
        );
        assert_eq!(b.usage(), Usage::default());
        assert_eq!(a.charge(1, 0), Err(Exceeded(Limit::Work)));
        assert_eq!(a.charge(0, 0), Err(Exceeded(Limit::Work)));
    }

    #[test]
    fn allocation_and_frame_limits_are_checked_before_the_effect() {
        let b = MacroBudget::new(Limits {
            work: 10,
            allocation: 2,
            frames: 2,
        });
        b.charge(1, 2).unwrap();
        assert_eq!(b.charge(0, 1), Err(Exceeded(Limit::Allocation)));
        assert_eq!(b.usage().allocation, 2);
        let b = MacroBudget::new(Limits {
            work: 10,
            allocation: 2,
            frames: 2,
        });
        b.enter_frame(1).unwrap();
        b.enter_frame(1).unwrap();
        b.restore_frames(1);
        b.leave_frame();
        assert_eq!(b.usage().frames, 0);
        b.enter_frame(1).unwrap();
        b.enter_frame(1).unwrap();
        assert_eq!(b.enter_frame(1), Err(Exceeded(Limit::Frames)));
    }

    #[test]
    fn restore_uses_stack_depth_not_weighted_frame_units() {
        let b = MacroBudget::new(Limits {
            work: 10,
            allocation: 10,
            frames: 10,
        });
        b.enter_frame(2).unwrap();
        b.enter_frame(3).unwrap();
        b.enter_frame(4).unwrap();
        assert_eq!(b.usage().frames, 9);
        b.restore_frames(1);
        assert_eq!(b.usage().frames, 2);
        b.leave_frame();
        assert_eq!(b.usage().frames, 0);
    }

    #[test]
    fn merged_compiler_services_cannot_silently_change_the_budget() {
        let budget = MacroBudget::default();
        let mut a = crate::HostImports::default();
        a.set_macro_budget(budget.clone());
        let mut b = crate::HostImports::default();
        b.set_macro_budget(budget);
        a.merge(b).unwrap();
        let mut c = crate::HostImports::default();
        c.set_macro_budget(MacroBudget::default());
        assert!(a.merge(c).is_err());
    }
}

use crate::runtime::Runtime;
use std::cell::Cell;
use std::ops::{Deref, DerefMut};
use std::{mem, ptr};

thread_local! {
     static RUNTIME: Cell<Option<&'static mut Runtime>> = const { Cell::new(None) };
}

#[must_use = "deref in order to access the runtime"]
pub struct RuntimeGuard {
    runtime: &'static mut Runtime,
}

impl Drop for RuntimeGuard {
    fn drop(&mut self) {
        let runtime = unsafe { ptr::read(&raw const self.runtime) };
        RUNTIME.set(Some(runtime));
    }
}

impl Deref for RuntimeGuard {
    type Target = Runtime;

    fn deref(&self) -> &Self::Target {
        self.runtime
    }
}

impl DerefMut for RuntimeGuard {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.runtime
    }
}

pub fn try_runtime() -> Option<RuntimeGuard> {
    Some(RuntimeGuard {
        runtime: RUNTIME.try_with(Cell::take).ok()??,
    })
}

#[cold]
const fn no_runtime<T>() -> T {
    panic!("no runtime present");
}

pub fn runtime() -> RuntimeGuard {
    try_runtime().unwrap_or_else(no_runtime)
}

impl Runtime {
    pub fn run_in<O, F: FnOnce() -> O>(&mut self, fun: F) -> O {
        let prev = RUNTIME.replace(Some(unsafe { mem::transmute(self) }));
        let ret = fun();
        RUNTIME.replace(prev).unwrap_or_else(no_runtime);
        ret
    }
}

use super::Runtime;
use crate::Error;
use crate::runtime::AsyncApplication;
use crate::tls::runtime;
use async_local_executor::{Executor, spawn_local};
use winit::event_loop::EventLoop;

#[inline]
pub fn run_app<E, F>(future: F) -> Result<(), Error<E>>
where
    F: IntoFuture<Output = Result<(), E>> + 'static,
{
    let event_loop = EventLoop::new().map_err(Error::EventLoop)?;
    let proxy = event_loop.create_proxy();
    let mut executor = Executor::new(move || proxy.send_event(()).unwrap());
    let executor_guard = executor.enter();
    let handle = spawn_local(async move {
        let result = future.await;
        runtime().event_loop().exit();
        async_local_executor::exit();
        result
    });
    let mut runtime = Runtime::new();
    let runtime_guard = runtime.enter();
    event_loop
        .run_app(&mut AsyncApplication)
        .map_err(Error::EventLoop)?;
    drop(runtime_guard);
    drop(executor_guard);
    let Some(result) = handle.result() else {
        return Ok(());
    };
    result.map_err(Error::App)
}

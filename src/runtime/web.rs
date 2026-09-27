use super::Runtime;
use crate::Error;
use crate::runtime::AsyncApplication;
use crate::tls::runtime;
use async_local_executor::{Executor, spawn_local};
use winit::event_loop::EventLoop;
use winit::platform::web::EventLoopExtWebSys;

#[inline]
pub async fn run_app<E, F>(future: F) -> Result<(), Error<E>>
where
    E: 'static,
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
    event_loop.spawn_app(AsyncApplication);
    let res = handle.await.map_err(Error::App);
    drop(runtime_guard);
    drop(executor_guard);
    res
}

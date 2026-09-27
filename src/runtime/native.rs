use super::Runtime;
use crate::Error;
use crate::tls::runtime;
use async_local_executor::{Executor, JoinHandle, TaskHandle, spawn_local};
use winit::application::ApplicationHandler;
use winit::error::EventLoopError;
use winit::event::{DeviceEvent, DeviceId, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::WindowId;

struct NativeApplication;

impl ApplicationHandler<TaskHandle> for NativeApplication {
    fn suspended(&mut self, _: &ActiveEventLoop) {
        runtime().suspended();
    }

    fn resumed(&mut self, _: &ActiveEventLoop) {
        runtime().resumed();
    }

    fn window_event(&mut self, _: &ActiveEventLoop, window_id: WindowId, event: WindowEvent) {
        runtime().window_event(window_id, event);
    }

    fn device_event(&mut self, _: &ActiveEventLoop, device_id: DeviceId, event: DeviceEvent) {
        runtime().device_event(device_id, event);
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, task_handle: TaskHandle) {
        runtime().set_event_loop(event_loop);
        task_handle.tick();
    }
}

fn run<E, F>(future: F) -> Result<JoinHandle<Result<(), E>>, EventLoopError>
where
    F: IntoFuture<Output = Result<(), E>> + 'static,
{
    let event_loop = EventLoop::with_user_event().build()?;
    let proxy = event_loop.create_proxy();
    let mut ex = Executor::new(move |task| proxy.send_event(task).unwrap());
    ex.run_in(|| {
        let handle = spawn_local(future);
        let mut runtime = Runtime::new();
        runtime
            .run_in(move || event_loop.run_app(&mut NativeApplication))
            .map(|()| handle)
    })
}

#[inline]
pub fn run_app<E, F>(future: F) -> Result<(), Error<E>>
where
    F: IntoFuture<Output = Result<(), E>> + 'static,
{
    let future = async move {
        let result = future.await;
        runtime().event_loop().exit();
        result
    };
    let handle = run(future).map_err(Error::EventLoop)?;
    let Some(result) = handle.result() else {
        return Ok(());
    };
    result.map_err(Error::App)
}

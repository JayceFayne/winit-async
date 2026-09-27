use super::Runtime;
use crate::Error;
use crate::tls::runtime;
use async_local_executor::{Executor, JoinHandle, TaskHandle, spawn_local};
use winit::application::ApplicationHandler;
use winit::error::EventLoopError;
use winit::event::{DeviceEvent, DeviceId, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::platform::web::EventLoopExtWebSys;
use winit::window::WindowId;

struct WebApplication {
    runtime: Runtime,
    executor: Executor,
}

impl ApplicationHandler<TaskHandle> for WebApplication {
    fn suspended(&mut self, _: &ActiveEventLoop) {
        self.runtime.suspended();
    }

    fn resumed(&mut self, _: &ActiveEventLoop) {
        self.runtime.resumed();
    }

    fn window_event(&mut self, _: &ActiveEventLoop, window_id: WindowId, event: WindowEvent) {
        self.runtime.window_event(window_id, event);
    }

    fn device_event(&mut self, _: &ActiveEventLoop, device_id: DeviceId, event: DeviceEvent) {
        self.runtime.device_event(device_id, event);
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, task_handle: TaskHandle) {
        self.runtime.set_event_loop(event_loop);
        self.runtime
            .run_in(|| self.executor.run_in(move || task_handle.tick()))
    }
}

fn spawn_app<E, F>(future: F) -> Result<JoinHandle<Result<(), E>>, EventLoopError>
where
    F: IntoFuture<Output = Result<(), E>> + 'static,
{
    let event_loop = EventLoop::with_user_event().build()?;
    let proxy = event_loop.create_proxy();
    let mut executor = Executor::new(move |task| proxy.send_event(task).unwrap());
    let handle = executor.run_in(|| spawn_local(future));
    let runtime = Runtime::new();
    let app = WebApplication { runtime, executor };
    event_loop.spawn_app(app);
    Ok(handle)
}

#[inline]
pub async fn run_app<E, F>(future: F) -> Result<(), Error<E>>
where
    E: 'static,
    F: IntoFuture<Output = Result<(), E>> + 'static,
{
    let future = async move {
        let result = future.await;
        runtime().event_loop().exit();
        result
    };
    let handle = spawn_app(future).map_err(Error::EventLoop)?;
    handle.await.map_err(Error::App)
}

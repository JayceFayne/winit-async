use crate::map::Map;
use crate::tls::runtime;
use async_local_channel::{spsc, watch};
use async_local_executor::{Executor, JoinHandle, TaskHandle, spawn_local};
use std::fmt::Debug;
use std::mem;
use winit::application::ApplicationHandler;
use winit::error::EventLoopError;
use winit::event::{DeviceEvent, DeviceId, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::WindowId;

#[derive(Debug, Default, Clone, Copy)]
enum State {
    #[default]
    Suspended,
    Resumed,
}

impl State {
    const fn is_resumed(self) -> bool {
        match self {
            State::Suspended => false,
            State::Resumed => true,
        }
    }

    const fn is_suspended(self) -> bool {
        !self.is_resumed()
    }
}

struct ChannelPair<T, R> {
    tx: T,
    rx: R,
}

impl<T, R> ChannelPair<T, R> {
    fn new((tx, rx): (T, R)) -> Self {
        Self { tx, rx }
    }
}

pub struct Runtime {
    event_loop: Option<&'static ActiveEventLoop>,
    resumed: ChannelPair<watch::Sender<()>, watch::InactiveReceiver<()>>,
    suspended: ChannelPair<watch::Sender<()>, watch::InactiveReceiver<()>>,
    window_events: Map<WindowId, spsc::Sender<WindowEvent>>,
    device_events: Option<spsc::Sender<(DeviceId, DeviceEvent)>>,
    state: State,
}

impl Runtime {
    fn new() -> Self {
        Self {
            event_loop: None,
            resumed: ChannelPair::new(watch::channel()),
            suspended: ChannelPair::new(watch::channel()),
            window_events: Map::new(),
            device_events: None,
            state: State::Suspended,
        }
    }

    pub fn event_loop(&self) -> &ActiveEventLoop {
        self.event_loop.unwrap()
    }

    fn set_event_loop(&mut self, event_loop: &ActiveEventLoop) {
        self.event_loop = Some(unsafe { mem::transmute(event_loop) });
    }

    fn resumed(&mut self) {
        self.state = State::Resumed;
        self.resumed.tx.send(()).unwrap();
    }

    pub fn resumed_rx(&mut self) -> Option<watch::Receiver<()>> {
        if self.state.is_resumed() {
            None
        } else {
            Some(self.resumed.rx.clone().activate())
        }
    }

    fn suspended(&mut self) {
        self.state = State::Suspended;
        self.suspended.tx.send(()).unwrap();
    }

    pub fn suspended_rx(&mut self) -> Option<watch::Receiver<()>> {
        if self.state.is_suspended() {
            None
        } else {
            Some(self.suspended.rx.clone().activate())
        }
    }

    pub fn device_event_receiver(&mut self) -> Option<spsc::Receiver<(DeviceId, DeviceEvent)>> {
        if self.device_events.is_some() {
            return None;
        }
        let (tx, rx) = spsc::channel();
        assert!(self.device_events.replace(tx).is_none());
        Some(rx.activate())
    }

    pub fn device_event_receiver_dropped(&mut self) {
        self.device_events = None;
    }

    pub fn window_event_receiver(
        &mut self,
        window_id: WindowId,
    ) -> Option<spsc::Receiver<WindowEvent>> {
        if self.window_events.contains_key(&window_id) {
            return None;
        }
        let (tx, rx) = spsc::channel();
        self.window_events.push(window_id, tx);
        Some(rx.activate())
    }

    fn device_event(&mut self, device_id: DeviceId, event: DeviceEvent) {
        if let Some(tx) = self.device_events.as_ref() {
            tx.send((device_id, event)).unwrap();
        }
    }

    pub fn window_event_receiver_dropped(&mut self, window_id: WindowId) {
        self.window_events.remove(&window_id);
    }

    fn window_event(&mut self, window_id: WindowId, event: WindowEvent) {
        if let Some(tx) = self.window_events.get(&window_id) {
            tx.send(event).unwrap();
        }
    }
}

struct RuntimeProxy;

impl ApplicationHandler<TaskHandle> for RuntimeProxy {
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

pub fn run<E, F>(future: F) -> Result<JoinHandle<Result<(), E>>, EventLoopError>
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
            .run_in(move || event_loop.run_app(&mut RuntimeProxy))
            .map(|()| handle)
    })
}

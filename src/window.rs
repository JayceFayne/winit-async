use crate::tls::{runtime, try_runtime};
use async_local_channel::spsc;
use std::pin::{Pin, pin};
use std::task::{Context, Poll};
use winit::event::WindowEvent;
use winit::window::{Window, WindowId};

pub struct WindowEvents {
    window_id: WindowId,
    rx: spsc::Receiver<WindowEvent>,
}

impl WindowEvents {
    #[inline]
    pub const fn next_event(&mut self) -> WindowEventFuture<'_> {
        WindowEventFuture { inner: self }
    }
}

pub struct WindowEventFuture<'a> {
    inner: &'a WindowEvents,
}

impl Future for WindowEventFuture<'_> {
    type Output = Option<WindowEvent>;

    #[inline]
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        pin!(self.inner.rx.recv()).poll(cx).map(Result::ok)
    }
}

impl Drop for WindowEvents {
    #[inline]
    fn drop(&mut self) {
        if let Some(mut runtime) = try_runtime() {
            runtime.window_event_receiver_dropped(self.window_id);
        }
    }
}

pub trait WindowExtAsync {
    fn events(&self) -> Option<WindowEvents>;
}

impl WindowExtAsync for Window {
    #[inline]
    fn events(&self) -> Option<WindowEvents> {
        let window_id = self.id();
        let rx = runtime().window_event_receiver(window_id)?;
        Some(WindowEvents { window_id, rx })
    }
}

use crate::tls::{runtime, try_runtime};
use async_local_channel::spsc;
use std::pin::{Pin, pin};
use std::task::{Context, Poll};
use winit::event::{DeviceEvent, DeviceId};

pub struct DeviceEventListener {
    rx: spsc::Receiver<(DeviceId, DeviceEvent)>,
}

impl DeviceEventListener {
    pub(crate) fn new() -> Option<Self> {
        let rx = runtime().device_event_receiver()?;
        Some(Self { rx })
    }

    #[inline]
    pub const fn next_event(&mut self) -> DeviceEventFuture<'_> {
        DeviceEventFuture { inner: self }
    }
}

pub struct DeviceEventFuture<'a> {
    inner: &'a DeviceEventListener,
}

impl Future for DeviceEventFuture<'_> {
    type Output = Option<(DeviceId, DeviceEvent)>;

    #[inline]
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        pin!(self.inner.rx.recv()).poll(cx).map(Result::ok)
    }
}

impl Drop for DeviceEventListener {
    #[inline]
    fn drop(&mut self) {
        if let Some(mut runtime) = try_runtime() {
            runtime.device_event_receiver_dropped();
        }
    }
}

#![doc = include_str!("../README.md")]
//#![warn(clippy::all, clippy::pedantic)]
#![allow(clippy::missing_transmute_annotations)]

mod device;
mod error;
mod map;
mod runtime;
mod tls;
mod window;

use crate::tls::runtime;
use runtime::run;
use std::future::pending;
use winit::error::OsError;
use winit::event_loop::{DeviceEvents, OwnedDisplayHandle};
use winit::monitor::MonitorHandle;
use winit::window::{CustomCursor, CustomCursorSource, Theme, Window, WindowAttributes};

pub use crate::device::DeviceEventListener;
pub use async_local_executor::{JoinHandle, TaskHandle, spawn_local};
pub use error::Error;
pub use window::{WindowEventFuture, WindowEvents, WindowExtAsync};
pub use winit;

#[inline]
pub fn create_window(window_attributes: WindowAttributes) -> Result<Window, OsError> {
    runtime().event_loop().create_window(window_attributes)
}

#[inline]
pub fn create_custom_cursor(custom_cursor: CustomCursorSource) -> CustomCursor {
    runtime().event_loop().create_custom_cursor(custom_cursor)
}

#[inline]
pub fn available_monitors() -> Vec<MonitorHandle> {
    runtime().event_loop().available_monitors().collect()
}

#[inline]
pub fn primary_monitor() -> Option<MonitorHandle> {
    runtime().event_loop().primary_monitor()
}

#[inline]
pub fn listen_device_events(allowed: DeviceEvents) {
    runtime().event_loop().listen_device_events(allowed)
}

#[inline]
pub fn system_theme() -> Option<Theme> {
    runtime().event_loop().system_theme()
}

#[inline]
pub async fn exit() -> ! {
    runtime().event_loop().exit();
    pending::<()>().await;
    unreachable!()
}

#[inline]
pub fn owned_display_handle() -> OwnedDisplayHandle {
    runtime().event_loop().owned_display_handle()
}

#[inline]
pub async fn resumed() {
    if let Some(rx) = { runtime().resumed_rx() } {
        rx.recv().await.unwrap();
    }
}

#[inline]
pub async fn suspended() {
    if let Some(rx) = { runtime().suspended_rx() } {
        rx.recv().await.unwrap();
    }
}

#[inline]
pub fn device_events() -> Option<DeviceEventListener> {
    DeviceEventListener::new()
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

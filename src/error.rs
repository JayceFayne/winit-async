use std::convert::Infallible;
use std::fmt::{Debug, Display};
use winit::error::EventLoopError;

pub enum Error<E = Infallible> {
    EventLoop(EventLoopError),
    App(E),
}

impl<E: Debug> Debug for Error<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EventLoop(err) => Debug::fmt(err, f),
            Self::App(err) => Debug::fmt(err, f),
        }
    }
}

impl<E: Display> Display for Error<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EventLoop(err) => Display::fmt(err, f),
            Self::App(err) => Display::fmt(err, f),
        }
    }
}

# winit-async &emsp; [![Action Badge]][actions] [![Version Badge]][crates.io] [![License Badge]][license] [![Docs Badge]][docs]

[Version Badge]: https://img.shields.io/crates/v/winit-async.svg
[crates.io]: https://crates.io/crates/winit-async
[Action Badge]: https://github.com/JayceFayne/winit-async/workflows/Rust/badge.svg
[actions]: https://github.com/JayceFayne/winit-async/actions
[License Badge]: https://img.shields.io/crates/l/winit-async.svg
[license]: https://github.com/JayceFayne/winit-async/blob/master/LICENSE.md
[Docs Badge]: https://docs.rs/winit-async/badge.svg
[docs]: https://docs.rs/winit-async

This crate integrates asynchronous task execution with the winit event loop, allowing `!Send` futures to be spawned and executed while asynchronously listening for winit events. Timers and I/O are intentionally out of scope for this crate. Use a separate integration for these capabilities. Since this crate is essentially a wrapper around winit, the [winit documentation](https://docs.rs/winit/latest/winit/) should be consulted for details on window creation, event handling, and platform-specific behavior.

## Usage

Examples of how to use the library can be found [here](./examples).

## Contributing

If you find any errors in winit-async or just want to add a new feature feel free to [submit a PR](https://github.com/jaycefayne/winit-async/pulls).

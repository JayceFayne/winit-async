use winit_async::{Error, run_app};

fn main() -> Result<(), Error> {
    run_app(async {
        println!("Hello, world!");
        Ok(())
    })
}

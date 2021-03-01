#![feature(type_alias_impl_trait)]

mod display_managers;
mod util;

use display_managers::x11::X11;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dim = util::Dimension {
        height: 20,
        width: 1920,
        x: 1920,
        y: 0,
    };

    let x11 = X11::new();
    let _window = x11.create_window(dim);
    x11.loop_events();
    Ok(())
}

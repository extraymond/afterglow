#![recursion_limit = "256"]

// pub mod component;
pub mod bus;
pub mod composer;
pub mod container;
pub mod messenger;
pub mod node;
pub mod renderer;

pub mod prelude {
    pub use crate::bus::*;
    pub use crate::container::*;
    pub use crate::messenger::*;
    pub use crate::renderer::*;

    pub use dodrio::{self, builder::text, bumpalo::format as bf, Node, RenderContext};
    pub use futures::channel::{
        mpsc::{self, UnboundedReceiver as Receiver, UnboundedSender as Sender},
        oneshot,
    };
    pub use futures::prelude::*;
    pub use typed_html::{self, dodrio};
    pub use wasm_bindgen::{self, prelude::*, JsCast};
    pub use wasm_bindgen_futures::*;
    pub use web_sys::Event;
}
use cfg_if::*;

cfg_if! {
    // When the `console_error_panic_hook` feature is enabled, we can call the
    // `set_panic_hook` function to get better error messages if we ever panic.
    if #[cfg(feature = "console_error_panic_hook")] {
        extern crate console_error_panic_hook;

    } else {
        #[inline]
        fn set_panic_hook() {}
    }
}

cfg_if! {
    // When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
    // allocator.
    if #[cfg(feature = "wee_alloc")] {
        extern crate wee_alloc;
        #[global_allocator]
        static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;
    }

}

#[cfg(test)]
mod tests {
    use super::prelude::*;
    use wasm_bindgen_test::*;
    wasm_bindgen_test_configure!(run_in_browser);

    pub fn init_test() {
        if let Err(msg) = femme::start(log::LevelFilter::Info) {
            log::error!("{}", msg);
        }
    }
}

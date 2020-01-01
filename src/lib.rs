pub mod component;
pub mod dyn_component;
pub mod dyn_hub;
pub mod router;
pub mod services;
pub mod styler;
pub mod threading;
pub mod utils;
pub mod views;

pub mod prelude {
    pub use crate::component::*;
    pub use dodrio::{Node, RenderContext};
    pub use futures::channel::mpsc::{
        self, UnboundedReceiver as Receiver, UnboundedSender as Sender,
    };
    pub use futures::{sink::SinkExt, stream::StreamExt};
    pub use typed_html::{self, dodrio};
    pub use wasm_bindgen::{prelude::*, JsCast};
    pub use wasm_bindgen_futures::futures_0_3::*;
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

    use super::use_panic_hook;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);
    pub fn init_test() {
        set_panic_hook();
        femme::start(log::LevelFilter::Info);
    }
}

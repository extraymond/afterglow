#![feature(trait_alias)]

pub mod component;
pub mod utils;
pub mod prelude {
    pub use dodrio::Node;
    pub use dodrio::RenderContext;
    pub use futures::sink::SinkExt;
    pub use typed_html::dodrio;
    pub use wasm_bindgen::JsCast;
    pub use wasm_bindgen_futures::futures_0_3::spawn_local;
    pub use web_sys::Event;
}

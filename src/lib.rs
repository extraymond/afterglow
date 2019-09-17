pub mod component;
pub mod utils;
pub mod prelude {
    pub use crate::component::*;
    pub use dodrio::{Node, RenderContext};
    pub use futures::{channel::mpsc, compat::Future01CompatExt, sink::SinkExt, stream::StreamExt};
    pub use typed_html::{self, dodrio};
    pub use wasm_bindgen::{prelude::*, JsCast};
    pub use wasm_bindgen_futures::futures_0_3::*;
    pub use web_sys::Event;
}

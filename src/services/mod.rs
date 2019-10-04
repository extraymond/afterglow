pub mod ws;
use futures::channel::{mpsc, oneshot};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::MessageEvent;

pub struct Service<T: ServiceConnection> {
    client: T,
    // _onopen: Closure<dyn FnMut()>,
    // _onclose: Closure<dyn FnMut()>,
    // _onmsg: Closure<dyn FnMut()>,
    // out_tx: mpsc::UnboundedSender<MessageEvent>,
}

pub trait ServiceConnection {
    fn create() -> Self;
}

pub trait ServiceOutput {
    type Msg;
    fn output(tx: mpsc::UnboundedSender<Self::Msg>) -> Self;
}

// impl<T> Service<T> {
//     fn create(client: T) -> Self {
//         Self { client }
//     }
// }

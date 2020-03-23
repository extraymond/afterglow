use async_executors::*;
use futures::channel::mpsc;
use futures::prelude::*;
use web_sys::{Worker, WorkerGlobalScope};

use gloo::events::EventListener;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::*;

pub struct Servant<T> {
    pub onmsg: EventListener,
    pub outer_rx: mpsc::UnboundedReceiver<web_sys::MessageEvent>,
    pub inner_tx: mpsc::UnboundedSender<T>,
    pub inner_rx: mpsc::UnboundedReceiver<T>,
    pub scope: WorkerGlobalScope,
}

impl<T> Servant<T> {
    pub fn new(scope: WorkerGlobalScope) -> Servant<T> {
        let target: &web_sys::EventTarget = scope.unchecked_ref();
        let (outer_tx, outer_rx) = mpsc::unbounded::<web_sys::MessageEvent>();
        let (inner_tx, inner_rx) = mpsc::unbounded::<T>();

        let onmsg = EventListener::new(&target, "message", move |e| {
            let e: web_sys::MessageEvent = e.clone().unchecked_into();
            let mut tx = outer_tx.clone();
            spawn_local(async move {
                let _ = tx.send(e).await;
            });
        });

        Servant {
            onmsg,
            outer_rx,
            inner_tx,
            inner_rx,
            scope,
        }
    }

    pub async fn handle_incoming(&mut self) {
        self.outer_rx
            .by_ref()
            .for_each(|msg| async move {
                log::info!("{:?} incoming messages", msg);
            })
            .await;
    }
}

pub fn init_worker() {
    let _ = femme::start(log::LevelFilter::Info);
    log::info!("module started");
    let global = js_sys::global();
    let scope = global.unchecked_into::<web_sys::WorkerGlobalScope>();
    let target = scope.unchecked_ref::<web_sys::DedicatedWorkerGlobalScope>();
    target.post_message(&JsValue::from("hello!")).unwrap();

    spawn_local(async move {
        let mut servant = Servant::<i32>::new(scope);
        servant.handle_incoming().await;
    });
}

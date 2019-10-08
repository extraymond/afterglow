use futures::channel::mpsc;
use futures::prelude::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::futures_0_3::spawn_local;
use web_sys::MessageEvent;
use web_sys::Worker;

pub enum Agent {
    Main(AgentStore),
    Worker(AgentStore),
}

pub struct AgentStore {
    worker: Option<Worker>,
    _onmsg: Option<Closure<dyn FnMut(MessageEvent)>>,
    _onerror: Option<Closure<dyn FnMut(MessageEvent)>>,
    msg_rx: Option<mpsc::UnboundedReceiver<MessageEvent>>,
    sab: Option<js_sys::SharedArrayBuffer>,
}

impl AgentStore {
    pub fn _onmsg(&mut self) {
        let (tx, rx) = mpsc::unbounded::<MessageEvent>();
        let tx_handle = tx.clone();
        let f = move |e: MessageEvent| {
            let mut tx = tx_handle.clone();
            let fut = async move {
                tx.send(e)
                    .await
                    .expect("unable to transfer msg to wasm module");
            };
            spawn_local(fut);
        };
        let closure = Closure::wrap(Box::new(f) as Box<dyn FnMut(MessageEvent)>);
        self.bind_onmsg(&closure);
        self._onmsg = Some(closure);
        self.msg_rx = Some(rx);
    }

    pub fn bind_onmsg(&self, cbk: &Closure<dyn FnMut(MessageEvent)>) {
        if let Some(worker) = &self.worker {
            worker.set_onmessage(Some(cbk.as_ref().unchecked_ref()));
        }
    }

    pub async fn listening(&mut self) {
        let mut msg_rx = self.msg_rx.take().expect("unable to take msg_rx");
        while let Some(msg) = msg_rx.next().await {
            let data = msg.data();
            
        }
    }

    pub fn init_sab(&mut self, size: u32) {
        let sab = js_sys::SharedArrayBuffer::new(size);
        self.sab = Some(sab);
    }
}

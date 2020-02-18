#![recursion_limit = "256"]

pub mod component;
// pub mod router;
// pub mod styler;
// pub mod utils;
// pub mod views;

pub mod prelude {
    pub use crate::component::*;
    pub use dodrio::{self, Node, RenderContext};
    pub use futures::channel::mpsc::{
        self, UnboundedReceiver as Receiver, UnboundedSender as Sender,
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
//
#[cfg(test)]
mod tests {

    use super::prelude::*;
    use anyhow::Result;
    use gloo::events::EventListener;
    use uuid::Uuid;
    use wasm_bindgen_test::*;
    wasm_bindgen_test_configure!(run_in_browser);
    use futures_timer::Delay;
    use std::time::Duration;

    fn setup_basic() -> Result<()> {
        let mut hub = MessageHub::new();
        #[derive(Default)]
        pub struct Model(i32, Option<String>);
        pub enum Msg {
            Init(Sender<web_sys::Element>),
            Add,
        }

        impl Component<Msg, ()> for Model {
            fn new(_: Sender<bool>) -> Self {
                Self::default()
            }

            fn mounted(mut tx: Sender<Msg>, _: Sender<()>, _: Sender<bool>) {
                let (ev_tx, mut rx) = mpsc::unbounded::<web_sys::Element>();
                spawn_local(async move {
                    let mut listeners = vec![];
                    while let Some(el) = rx.next().await {
                        log::info!("some node is up");
                        let el: web_sys::EventTarget = el.unchecked_into();
                        let on_hey = EventListener::new(&el, "hey", move |_| {
                            log::info!("i've got heyed");
                        });
                        log::info!("ready to be heyed");
                        listeners.push(on_hey);
                    }
                });
                spawn_local(async move {
                    tx.send(Msg::Init(ev_tx)).await;
                });
            }

            fn update(&mut self, msg: Msg) -> bool {
                match msg {
                    Msg::Init(mut tx) => {
                        let doc = web_sys::window()
                            .map(|win| win.document())
                            .flatten()
                            .unwrap();

                        while self.1.is_none() {
                            let uid = Uuid::new_v4();
                            let uid_string = uid.to_string();
                            if doc
                                .query_selector_all(&format!("[value={}]", uid_string))
                                .map(|nodes| nodes.get(0))
                                .ok()
                                .flatten()
                                .is_none()
                            {
                                log::info!("uid string: {}", uid_string);
                                let id = uid_string.clone();
                                let task = async move {
                                    loop {
                                        let nodes = doc
                                            .query_selector_all(&format!("[value='{}']", id))
                                            .unwrap();

                                        if let Some(node) = nodes.get(0) {
                                            tx.send(node.unchecked_into()).await;
                                            log::info!("find the node");
                                            break;
                                        }
                                        Delay::new(Duration::from_millis(100)).await;
                                    }
                                };
                                spawn_local(task);
                                self.1.replace(uid_string);
                                break;
                            }
                        }
                        true
                    }
                    Msg::Add => {
                        self.0 += 1;
                        true
                    }
                }
            }
        }

        impl Render<Msg, ()> for Model {
            fn render<'a>(
                &self,
                ctx: &mut RenderContext<'a>,
                data_tx: Sender<Msg>,
                _: Sender<()>,
                _: Sender<bool>,
            ) -> Node<'a> {
                use dodrio::{builder::*, bumpalo::format as bformat};
                let bump = ctx.bump;
                let uid = self.1.clone().unwrap_or_default();
                dodrio!(bump,
                    <div id="me" data-value={ self.1.clone().unwrap_or_default() }>
                    <button
                    onclick={move |_, _, e| {
                        let mut tx = data_tx.clone();
                        let fut = async move {
                            tx.send(Msg::Add).await.expect("unable to send");
                        };
                        spawn_local(fut);
                        let doc = web_sys::window().map(|win| win.document()).flatten().unwrap();
                        let nodes = doc.query_selector_all(&format!("[value='{}']", uid)).unwrap();
                        if let Some(el) = nodes.get(0) {
                            log::info!("ready to hey someone");
                            let target: web_sys::EventTarget = el.unchecked_into();
                            let event = web_sys::Event::new("hey").unwrap();
                            target.dispatch_event(&event).unwrap();
                        }

                    }}>"click me"</button>
                    <p>{ vec![text(bformat!(in bump, "{}", self.0).into_bump_str())]}</p>

                    </div>)
            }
        }

        hub.bind_root_el(Model::default(), None);
        hub.mount_hub_rx();

        Ok(())
    }
    pub fn init_test() {
        if let Err(msg) = femme::start(log::LevelFilter::Info) {
            log::error!("{}", msg);
        }
    }

    #[wasm_bindgen_test]
    fn test_simpl() {
        init_test();
        setup_basic().expect("to work");
    }
}

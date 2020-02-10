pub mod component;
// pub mod router;
pub mod styler;
// pub mod utils;
// pub mod views;

pub mod prelude {
    pub use crate::component::*;
    pub use dodrio::{Node, RenderContext};
    pub use futures::channel::mpsc::{
        self, UnboundedReceiver as Receiver, UnboundedSender as Sender,
    };
    pub use futures::prelude::*;
    pub use typed_html::{self, dodrio};
    pub use wasm_bindgen::{prelude::*, JsCast};
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
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    fn setup_basic() -> Result<()> {
        let mut hub = MessageHub::new();
        #[derive(Default)]
        pub struct Model(i32);
        pub enum Msg {
            Add,
        }

        impl Component<Msg, ()> for Model {
            fn new(_: Sender<bool>) -> Self {
                Self::default()
            }

            fn update(&mut self, msg: Msg) -> bool {
                match msg {
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
                use dodrio::{builder::text, bumpalo::format as bformat};
                let bump = ctx.bump;
                let val = bformat!(in bump, "{}", self.0);
                dodrio!(bump,
                    <div>
                    <button onclick={move |_, _, _| {
                        let mut tx = data_tx.clone();
                        let fut = async move {
                            tx.send(Msg::Add).await.expect("unable to send");
                        };
                        spawn_local(fut);

                    }}>"click me"</button>
                    <p>{ vec![text(val.into_bump_str())]}</p>

                    </div>)
            }
        }

        hub.bind_root_el(Model::default(), None);
        hub.mount_hub_rx();

        Ok(())
    }
    pub fn init_test() {
        femme::start(log::LevelFilter::Info);
    }

    #[wasm_bindgen_test]
    fn test_simpl() {
        init_test();
        log::info!("setup");
        setup_basic().expect("to work");
    }
}

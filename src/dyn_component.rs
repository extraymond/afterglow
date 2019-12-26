use crate::prelude::{mpsc, spawn_local, Receiver, Sender};
use async_trait::async_trait;
use dodrio::RootRender;
use dodrio::{Node, RenderContext, Vdom, VdomWeak};
use futures::lock::Mutex;
use futures::prelude::*;
use std::rc::Rc;
use typed_html::dodrio;

pub struct Entity<T> {
    /// data container
    data: Rc<Mutex<T>>,
    /// allow data to trigger msg.
    msg_tx: Sender<Box<dyn MsgHandler<T>>>,
    /// allow trigger mutation.
    render_tx: Sender<bool>,
}

impl<T: 'static> Entity<T> {
    fn sync_data(&self) -> futures::lock::MutexGuard<T> {
        loop {
            if let Some(data) = self.data.try_lock() {
                break data;
            }
        }
    }

    fn event_capture(&self) -> Box<dyn Fn(&mut dyn RootRender, VdomWeak, web_sys::Event)> {
        let tx = self.render_tx.clone();
        let data = self.data.clone();
        Box::new(move |_, _, evt| {
            let tx = tx.clone();
            let data = data.clone();
            Self::event_handle(evt, data, tx);
        })
    }

    fn event_handle(evt: web_sys::Event, data: Rc<Mutex<T>>, tx: Sender<bool>) {
        let fut = async move {
            log::info!("hello!!");
            let mut data = data.lock().await;
        };
        spawn_local(fut);
    }
}

pub trait MsgHandler<T> {
    fn update(&self, target: T) -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::init_test;
    use wasm_bindgen_test::wasm_bindgen_test;

    struct Dummy {
        value: i32,
    }

    enum Msg {
        Add,
        Minus,
    }

    #[async_trait]
    impl MsgHandler<Dummy> for Msg {
        fn update(&self, mut target: Dummy) -> bool {
            match self {
                Msg::Add => {
                    target.value += 1;
                    true
                }
                Msg::Minus => {
                    target.value -= 1;
                    true
                }
            }
        }
    }

    impl dodrio::Render for Entity<Dummy> {
        fn render<'a>(&self, ctx: &mut RenderContext<'a>) -> Node<'a> {
            let bump = ctx.bump;
            let data = self.sync_data();
            let value = dodrio::bumpalo::format!(in bump,"inside {}", data.value);
            dodrio!(bump,
            <div>
            <div class="button"
            onclick={self.event_capture()}
            >{ dodrio::builder::text(value.into_bump_str())}</div>
            <link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/bulma/0.7.5/css/bulma.min.css"/>
            </div>
            )
        }
    }

    async fn remote(rx: futures::channel::oneshot::Receiver<bool>) {
        let (remote, remote_handle) = rx.remote_handle();
        let task1 = async {
            log::info!("task1 completed");
            assert_eq!(remote.await, ());
        };

        let task2 = async {
            let rv = remote_handle.await.unwrap();
            log::info!("task2 completed");
            assert_eq!(rv, true);
        };

        let combined = futures::future::join(task1, task2);
        combined.await;
    }

    async fn shared_steam() {
        let (tx, mut rx) = mpsc::unbounded::<usize>();

        let stream_future = rx.into_future();
        let (msg, rx) = stream_future.await;
    }

    // #[wasm_bindgen_test]
    async fn test_remote() {
        init_test();
        let (tx, rx) = futures::channel::oneshot::channel::<bool>();
        tx.send(true).expect("unable to send");
        remote(rx).await;
    }

    #[wasm_bindgen_test]
    async fn test_dummy() {
        init_test();
        let (msg_tx, rx) = mpsc::unbounded::<Box<dyn MsgHandler<Dummy>>>();

        let (render_tx, mut rx) = mpsc::unbounded::<bool>();

        let entity = Entity {
            data: Rc::new(Mutex::new(Dummy { value: 0 })),
            msg_tx,
            render_tx,
        };

        let body = web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .body()
            .unwrap();

        let vdom = Vdom::new(&body, entity);
        while let Some(msg) = rx.next().await {
            if msg {
                vdom.weak().render().await.expect("unable to rerender");
            }
        }
    }
}

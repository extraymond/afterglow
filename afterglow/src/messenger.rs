use crate::prelude::*;
use async_trait::async_trait;
use dodrio::{RootRender, VdomWeak};
use futures::channel::mpsc::unbounded;
use futures::lock::Mutex;
use std::rc::Rc;

pub type Message<T> = Box<dyn Messenger<Target = T>>;
pub type MessageSender<T> = Sender<Message<T>>;
pub type MessageReceiver<T> = Receiver<Message<T>>;

pub trait Messenger {
    type Target;

    fn update(
        &self,
        target: &mut Self::Target,
        sender: MessageSender<Self::Target>,
        render_tx: Sender<()>,
    ) -> bool {
        false
    }

    fn dispatch(self, sender: &MessageSender<Self::Target>)
    where
        Self: Sized + 'static,
    {
        let mut sender = sender.clone();
        let fut = async move {
            sender.send(Box::new(self)).await;
        };
        spawn_local(fut);
    }

    fn dispatch_async(
        self,
        sender: &MessageSender<Self::Target>,
        pending_rx: Option<oneshot::Receiver<()>>,
    ) -> oneshot::Receiver<()>
    where
        Self: Sized + 'static,
    {
        let (tx, rx) = oneshot::channel::<()>();
        let mut sender = sender.clone();
        let fut = async move {
            if let Some(rx) = pending_rx {
                let _ = rx.await;
            }

            let _ = sender.send(Box::new(self)).await;
            let _ = tx.send(());
        };
        spawn_local(fut);
        rx
    }
}

pub fn consume<T, M>(
    convert: impl Fn(Event) -> M + 'static,
    sender: &MessageSender<T>,
) -> impl Fn(&mut dyn RootRender, VdomWeak, Event) + 'static
where
    M: Messenger<Target = T> + 'static,
    T: 'static,
{
    let sender = sender.clone();
    move |_, _, event| {
        let msg = convert(event);
        msg.dispatch(&sender);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    pub struct Data {
        button: bool,
    }

    pub struct Data2 {
        button: bool,
    }

    pub enum Msg {
        Flipit,
    }

    pub enum Msg2 {
        Secret,
    }

    impl Messenger for Msg {
        type Target = Data;

        fn update(
            &self,
            target: &mut Self::Target,
            sender: MessageSender<Self::Target>,
            render_tx: Sender<()>,
        ) -> bool {
            target.button = !target.button;
            true
        }
    }

    impl Messenger for Msg2 {
        type Target = Data;

        fn update(
            &self,
            target: &mut Self::Target,
            sender: Sender<Box<dyn Messenger<Target = Self::Target>>>,
            render_tx: Sender<()>,
        ) -> bool {
            log::info!("not sure what to do, {}", target.button);
            false
        }
    }

    pub struct Container<T> {
        data: Rc<Mutex<T>>,
    }

    impl Container<Data> {
        fn start_handling(&self) {
            let (render_tx, _) = unbounded::<()>();
            let (tx, mut rx) = unbounded::<Message<Data>>();
            let data = self.data.clone();
            let tx_handle = tx.clone();
            let fut = async move {
                while let Some(msg) = rx.next().await {
                    let mut content = data.lock().await;
                    msg.update(&mut content, tx_handle.clone(), render_tx.clone());
                    log::info!("content value: {}", content.button);
                }
            };

            spawn_local(fut);

            let tx_handle = tx.clone();
            let mut tx = tx_handle.clone();
            let mutater = async move {
                tx.send(Box::new(Msg::Flipit)).await;
            };
            spawn_local(mutater);

            let mut tx = tx_handle.clone();
            let mutater = async move {
                tx.send(Box::new(Msg2::Secret)).await;
            };
            spawn_local(mutater);
        }
    }
}

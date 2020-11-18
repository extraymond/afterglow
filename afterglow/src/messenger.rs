use crate::prelude::*;
use async_std::task;
use dodrio::{RootRender, VdomWeak};

pub type Message<T> = Box<dyn Messenger<Target = T>>;
pub type MessageSender<T> = Sender<(Message<T>, oneshot::Sender<()>)>;
pub type MessageReceiver<T> = Receiver<(Message<T>, oneshot::Sender<()>)>;

pub trait Messenger {
    type Target;

    fn update(
        self: Box<Self>,
        target: &mut Self::Target,
        sender: &MessageSender<Self::Target>,
        render_tx: &Sender<((), oneshot::Sender<()>)>,
    ) -> bool {
        false
    }

    /// disptach a msg toward it's target.
    fn dispatch(self, sender: &MessageSender<Self::Target>) -> task::JoinHandle<()>
    where
        Self: Sized + 'static,
    {
        let mut sender = sender.clone();

        task::spawn_local(async move {
            let (tx, rx) = oneshot::channel::<()>();
            let _ = sender.send((Box::new(self), tx)).await;
            let _ = rx.await;
        })
    }
}

/// convert a msg into a closure to satisfy dodrio's internal renderer
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
        spawn_local(msg.dispatch(&sender));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::channel::mpsc::unbounded;
    use futures::lock::Mutex;
    use std::rc::Rc;

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
            self: Box<Self>,
            target: &mut Self::Target,
            sender: &MessageSender<Self::Target>,
            render_tx: &Sender<((), oneshot::Sender<()>)>,
        ) -> bool {
            target.button = !target.button;
            true
        }
    }

    impl Messenger for Msg2 {
        type Target = Data;

        fn update(
            self: Box<Self>,
            target: &mut Self::Target,
            sender: &Sender<(
                Box<dyn Messenger<Target = Self::Target>>,
                oneshot::Sender<()>,
            )>,
            render_tx: &Sender<((), oneshot::Sender<()>)>,
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
            let (render_tx, _) = unbounded::<((), oneshot::Sender<()>)>();
            let (tx, mut rx) = unbounded::<(Message<Data>, oneshot::Sender<()>)>();
            let data = self.data.clone();
            let tx_handle = tx.clone();
            let fut = async move {
                while let Some((msg, ready)) = rx.next().await {
                    let mut content = data.lock().await;
                    msg.update(&mut content, &tx_handle, &render_tx);
                    let _ = ready.send(());
                    log::info!("content value: {}", content.button);
                }
            };

            spawn_local(fut);
        }
    }
}

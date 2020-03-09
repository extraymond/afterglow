use crate::prelude::*;
use futures::channel::mpsc::unbounded;
use futures::lock::Mutex;
use std::sync::Arc;

pub trait Messenger {
    type Target;

    fn update(
        &self,
        target: &mut Self::Target,
        sender: Sender<Box<dyn Messenger<Target = Self::Target>>>,
    ) -> bool {
        false
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
            sender: Sender<Box<dyn Messenger<Target = Self::Target>>>,
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
        ) -> bool {
            log::info!("not sure what to do, {}", target.button);
            false
        }
    }

    pub struct Container<T> {
        data: Arc<Mutex<T>>,
    }

    impl Container<Data> {
        fn start_handling(&self) {
            let (tx, mut rx) = unbounded::<Box<dyn Messenger<Target = Data>>>();
            let data = self.data.clone();
            let tx_handle = tx.clone();
            let fut = async move {
                while let Some(msg) = rx.next().await {
                    let mut content = data.lock().await;
                    msg.update(&mut content, tx_handle.clone());
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

use crate::prelude::*;
use futures::lock::Mutex;
use std::rc::Rc;

pub struct Bus<T> {
    pub sender: Sender<T>,
    pub subs_tx: Sender<Sender<T>>,
    pub txs: Rc<Mutex<Vec<Sender<T>>>>,
}

impl<T: Copy + 'static> Bus<T> {
    pub fn new() -> Self {
        let (sender, rx) = mpsc::unbounded::<T>();
        let (subs_tx, subs_rx) = mpsc::unbounded::<Sender<T>>();
        let txs = Rc::new(Mutex::new(vec![]));

        spawn_local(Bus::handle_register(subs_rx, txs.clone()));
        spawn_local(Bus::handle_broadcast(rx, txs.clone()));

        Bus {
            sender,
            subs_tx,
            txs,
        }
    }
    pub async fn handle_register(mut rx: Receiver<Sender<T>>, txs: Rc<Mutex<Vec<Sender<T>>>>) {
        while let Some(tx) = rx.next().await {
            let mut txs = txs.lock().await;
            txs.push(tx);
        }
    }

    pub async fn handle_broadcast(mut rx: Receiver<T>, txs: Rc<Mutex<Vec<Sender<T>>>>) {
        while let Some(msg) = rx.next().await {
            let txs = txs.lock().await;
            stream::iter(txs.iter())
                .for_each(|tx| async move {
                    tx.clone().send(msg.clone()).await;
                })
                .await;
        }
    }

    pub fn mount_proxy<A: 'static>(&mut self, remote_tx: Sender<Box<dyn Messenger<Target = A>>>)
    where
        T: Into<Box<dyn Messenger<Target = A>>>,
    {
        let (tx, rx) = mpsc::unbounded::<T>();
        let mut subs_tx = self.subs_tx.clone();
        spawn_local(async move {
            subs_tx.send(tx).await;
            Bus::init_proxy(rx, remote_tx).await;
        });
    }

    pub async fn init_proxy<A>(
        mut bus_rx: Receiver<T>,
        mut msg_tx: Sender<Box<dyn Messenger<Target = A>>>,
    ) where
        T: Into<Box<dyn Messenger<Target = A>>>,
    {
        while let Some(msg) = bus_rx.next().await {
            let out_img: Box<dyn Messenger<Target = A>> = msg.into();
            msg_tx.send(out_img).await.unwrap();
        }
    }
}

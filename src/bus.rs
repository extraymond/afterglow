use crate::prelude::*;
use futures::lock::Mutex;
use std::rc::Rc;

pub struct Bus<T> {
    pub sender: Sender<T>,
    pub subs_tx: Sender<Sender<T>>,
    pub txs: Rc<Mutex<Vec<Sender<T>>>>,
}

#[derive(Clone)]
pub struct BusService<T> {
    pub bus: Rc<Mutex<Bus<T>>>,
    pub bus_tx: Sender<T>,
}

impl<T: Clone + 'static> Default for BusService<T> {
    fn default() -> Self {
        BusService::new()
    }
}

impl<T: Clone + 'static> BusService<T> {
    pub fn new() -> Self {
        let bus = Bus::new();
        BusService {
            bus_tx: bus.sender.clone(),
            bus: Rc::new(Mutex::new(bus)),
        }
    }

    pub fn register<A: 'static>(&self, remote_tx: MessageSender<A>)
    where
        T: Into<Option<Message<A>>>,
    {
        let bus = self.bus.clone();
        spawn_local(async move {
            let mut bus = bus.lock().await;
            bus.mount_proxy(remote_tx);
        });
    }

    pub fn publish(&self, msg: impl Into<T>) {
        let bus_msg: T = msg.into();
        let mut bus_tx = self.bus_tx.clone();
        spawn_local(async move {
            let _ = bus_tx.send(bus_msg).await;
        });
    }
}

impl<T: Clone + 'static> Bus<T> {
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
                .for_each(|tx| {
                    let mut tx = tx.clone();
                    let msg = msg.clone();
                    async move {
                        let _ = tx.send(msg).await;
                    }
                })
                .await;
        }
    }

    pub fn mount_proxy<A: 'static>(&mut self, remote_tx: MessageSender<A>)
    where
        T: Into<Option<Message<A>>>,
    {
        let (tx, rx) = mpsc::unbounded::<T>();
        let mut subs_tx = self.subs_tx.clone();
        spawn_local(async move {
            let _ = subs_tx.send(tx).await;
            Bus::init_proxy(rx, remote_tx).await;
        });
    }

    pub async fn init_proxy<A>(mut bus_rx: Receiver<T>, mut msg_tx: MessageSender<A>)
    where
        T: Into<Option<Message<A>>>,
    {
        while let Some(msg) = bus_rx.next().await {
            if let Some(out_msg) = msg.into() {
                let _ = msg_tx.send(out_msg).await;
            }
        }
    }
}

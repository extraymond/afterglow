use crate::prelude::*;
use futures::lock::Mutex;
use futures::prelude::*;
use std::rc::Rc;

/// A bus that can be subscribed to.
pub struct Bus<T> {
    pub sender: Sender<T>,
    pub subs_tx: Sender<Sender<(T, oneshot::Sender<()>)>>,
    pub txs: Rc<Mutex<Vec<Sender<(T, oneshot::Sender<()>)>>>>,
}

/// A sharable bus for containers to consume
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

    /// Register to the bus by sending the Sender into the bus
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

    /// Publish to the bus which will notify all members' sender
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
        let (subs_tx, subs_rx) = mpsc::unbounded::<Sender<(T, oneshot::Sender<()>)>>();
        let txs = Rc::new(Mutex::new(vec![]));

        spawn_local(Bus::handle_register(subs_rx, txs.clone()));
        spawn_local(Bus::handle_broadcast(rx, txs.clone()));

        Bus {
            sender,
            subs_tx,
            txs,
        }
    }

    /// listen for incoming registration, register by adding it's sender into Vec<Sender>
    pub async fn handle_register(
        mut rx: Receiver<Sender<(T, oneshot::Sender<()>)>>,
        txs: Rc<Mutex<Vec<Sender<(T, oneshot::Sender<()>)>>>>,
    ) {
        while let Some(tx) = rx.next().await {
            let mut txs = txs.lock().await;
            txs.push(tx);
        }
    }

    /// listen for incoming mesages, proxy mesages to the members.
    pub async fn handle_broadcast(
        rx: Receiver<T>,
        txs: Rc<Mutex<Vec<Sender<(T, oneshot::Sender<()>)>>>>,
    ) {
        rx.then(|msg| {
            let txs = txs.clone();
            let msg = msg.clone();
            async move {
                let txs = txs.lock().await;
                if !txs.is_empty() {
                    stream::iter(txs.iter())
                        .for_each(|tx| {
                            let mut tx = tx.clone();
                            let msg = msg.clone();
                            async move {
                                let (inner_tx, inner_rx) = oneshot::channel::<()>();
                                let _ = tx.send((msg, inner_tx)).await;
                                let _ = inner_rx.await;
                            }
                        })
                        .await;
                }
            }
        })
        .for_each(|_| async {
            log::trace!("broadcast done");
        })
        .await;
    }

    /// allow container to mount to the bus by registrating it's sender
    pub fn mount_proxy<A: 'static>(&mut self, remote_tx: MessageSender<A>)
    where
        T: Into<Option<Message<A>>>,
    {
        let (tx, rx) = mpsc::unbounded::<(T, oneshot::Sender<()>)>();
        let mut subs_tx = self.subs_tx.clone();
        spawn_local(async move {
            let _ = subs_tx.send(tx).await;
            Bus::init_proxy(rx, remote_tx).await;
        });
    }

    /// on behalf of the container, convert the broadcast message into a consumable form and trigger the messenger for the container
    pub async fn init_proxy<A>(bus_rx: Receiver<(T, oneshot::Sender<()>)>, msg_tx: MessageSender<A>)
    where
        T: Into<Option<Message<A>>>,
    {
        bus_rx
            .filter_map(|(msg, tx)| {
                let msg_tx = msg_tx.clone();
                async move {
                    if let Some(inner_msg) = msg.into() {
                        Some((msg_tx, inner_msg, tx))
                    } else {
                        let _ = tx.send(());
                        None
                    }
                }
            })
            .for_each(|(mut msg_tx, inner_msg, tx)| async move {
                let (_tx, _rx) = oneshot::channel::<()>();
                let _ = msg_tx.send((inner_msg, _tx)).await;
                let _ = _rx.await;
                let _ = tx.send(());
            })
            .await;
    }
}

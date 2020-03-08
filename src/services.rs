use crate::prelude::*;
use async_trait::async_trait;
use futures::lock::Mutex;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

pub struct Service<T> {
    pub name: String,
    pub rx: Receiver<T>,
    pub tx: Sender<T>,
    pub listeners: Rc<Mutex<Vec<Sender<T>>>>,
}

impl<T> Service<T>
where
    T: Copy,
{
    pub fn new(name: String) -> Self {
        let (tx, rx) = mpsc::unbounded::<T>();
        Service {
            name,
            rx,
            tx,
            listeners: Rc::new(Mutex::new(vec![])),
        }
    }

    pub async fn broadcast(&self, msg: T) {
        let mut txs = self.listeners.lock().await;
        futures::future::join_all(txs.iter_mut().map(|tx| async move {
            tx.send(msg.clone()).await.unwrap();
        }))
        .await;
    }

    pub async fn register(&mut self) -> Receiver<T> {
        let (tx, rx) = mpsc::unbounded::<T>();
        let mut txs = self.listeners.lock().await;
        txs.push(tx);
        rx
    }

    pub async fn start(&mut self) {
        while let Some(msg) = self.rx.next().await {
            self.broadcast(msg).await;
        }
    }
}

pub trait Register {
    type Medium;
    fn create(&self) -> Service<Self::Medium>;
}

// impl<T> Register for Service<T> {
//     type Medium = T;
// }

pub struct ServiceRegistry<T> {
    pub services: HashMap<String, Box<dyn Register<Medium = T>>>,
}

// impl ServiceRegistry {
//     pub fn register<T>(&mut self, name: String) {
//         if self.services.get(&name).is_some() {}
//     }
// }

use crate::prelude::spawn_local;
use crate::prelude::{mpsc, Receiver, Sender};
use core::fmt;
use dodrio::Vdom;
use failure::{Error, Fail};
use futures::compat::Future01CompatExt;
use futures::{SinkExt, StreamExt};

pub struct Hub {
    vdom: Option<Vdom>,
    hub_tx: Sender<bool>,
    hub_rx: Receiver<bool>,
}

impl Default for Hub {
    fn default() -> Self {
        Self::new()
    }
}

impl Hub {
    pub fn new() -> Self {
        let (hub_tx, hub_rx) = mpsc::unbounded::<bool>();
        Hub {
            vdom: None,
            hub_tx,
            hub_rx,
        }
    }

    pub async fn render(&self) -> Result<(), Error> {
        if let Some(vdom) = &self.vdom {
            vdom.weak().render().compat().await?;
        } else {
            log::warn!("Unable to render without a mounted vdom");
        }

        Ok(())
    }

    pub async fn link_el(&self) {
        let (tx, mut rx) = mpsc::unbounded::<bool>();
        let mut hub_tx = self.hub_tx.clone();

        let receive_update = async move {
            while let Some(_) = rx.next().await {
                if hub_tx.send(true).await.is_err() {
                    break;
                }
            }
        };
        spawn_local(receive_update);
    }

    pub async fn rendering(&mut self) -> Result<(), Error> {
        while let Some(_) = self.hub_rx.next().await {
            if self.render().await.is_err() {
                break;
            }
        }

        Ok(())
    }
}
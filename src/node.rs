use crate::prelude::*;
use async_trait::async_trait;

#[async_trait(?Send)]
pub trait Node {
    type Input;
    type Output;

    async fn operate(&self, mut up_rx: Receiver<()>, down_txs: Vec<Sender<()>>) {
        while let Some(_) = up_rx.next().await {
            stream::iter(down_txs.iter())
                .for_each(|mut tx| async move {
                    tx.send(()).await;
                })
                .await;
        }
    }
}

pub mod ws;
use crate::prelude::*;
use async_trait::async_trait;
use futures::channel::oneshot;
use futures::prelude::*;
use futures::stream;
use wasm_bindgen::convert::FromWasmAbi;
use web_sys::MessageEvent;

use futures::lock::Mutex;
use std::rc::Rc;

pub struct ServiceStore<In, Out> {
    /// stored callback for open notification.
    _onopen: Option<Closure<dyn FnMut()>>,
    /// stored callback for close notification.
    _onclose: Option<Closure<dyn FnMut()>>,
    /// stored callback for incoming msg handling.
    _onmsg: Option<Closure<dyn FnMut(In)>>,
    /// sender to send msg out.
    pub out_tx: Option<mpsc::UnboundedSender<Out>>,
    /// be able to subscribe to incoming msg.
    pub subscribers: Rc<Mutex<Vec<mpsc::UnboundedSender<In>>>>,
}

impl<In, Out> ServiceStore<In, Out> {
    pub fn new() -> ServiceStore<In, Out> {
        ServiceStore {
            _onopen: None,
            _onclose: None,
            _onmsg: None,
            out_tx: None,
            subscribers: Rc::new(Mutex::new(vec![])),
        }
    }
}

impl<In, Out> ServiceStore<In, Out> {
    fn set_onopen(&mut self, cbk: Closure<dyn FnMut()>) -> Result<(), failure::Error> {
        self._onopen = Some(cbk);
        Ok(())
    }

    fn set_onclose(&mut self, cbk: Closure<dyn FnMut()>) -> Result<(), failure::Error> {
        self._onopen = Some(cbk);
        Ok(())
    }

    fn set_onmsg(&mut self, cbk: Closure<dyn FnMut(In)>) -> Result<(), failure::Error> {
        self._onmsg = Some(cbk);
        Ok(())
    }
}

/// Generic server that handles incoming/output msg.
pub struct Service<In, Out> {
    pub client: Box<dyn ServiceInterface<In, Out>>,
    pub store: ServiceStore<In, Out>,
}

#[async_trait(?Send)]
pub trait ServiceConnect<In: 'static, Out: 'static> {
    async fn dial(&mut self) -> Result<(), failure::Error>;
    async fn pre_connect(&self) -> Result<(), failure::Error> {
        Ok(())
    }

    async fn bind_connect(
        &self,
        store: &mut ServiceStore<In, Out>,
    ) -> Result<oneshot::Receiver<()>, failure::Error> {
        let (tx, rx) = oneshot::channel::<()>();
        let mut tx = Some(tx);
        let f = move || {
            let tx = tx.take().expect("already taken");
            tx.send(()).expect("unable to notify open");
        };
        let cbk = Closure::wrap(Box::new(f) as Box<dyn FnMut()>);
        self.mount_onopen(&cbk);
        store.set_onopen(cbk)?;
        Ok(rx)
    }

    fn mount_onopen(&self, cbk: &Closure<dyn FnMut()>);

    async fn connect(&mut self, store: &mut ServiceStore<In, Out>) -> Result<(), failure::Error> {
        self.pre_connect().await?;
        self.dial().await?;
        let rx = self.bind_connect(store).await?;
        rx.await?;
        Ok(())
    }
}

#[async_trait(?Send)]
pub trait ServiceDisconnect<In: 'static, Out: 'static> {
    async fn pre_disconnect(&self) -> Result<(), failure::Error> {
        Ok(())
    }

    async fn bind_disconnect(
        &self,
        store: &mut ServiceStore<In, Out>,
    ) -> Result<oneshot::Receiver<()>, failure::Error> {
        let (tx, rx) = oneshot::channel::<()>();
        let mut tx = Some(tx);
        let f = move || {
            let tx = tx.take().expect("already taken");
            tx.send(()).expect("unable to notify close");
        };
        let cbk = Closure::wrap(Box::new(f) as Box<dyn FnMut()>);
        store.set_onclose(cbk)?;
        Ok(rx)
    }

    async fn disconnect(&self, store: &mut ServiceStore<In, Out>) -> Result<(), failure::Error> {
        self.pre_disconnect().await?;
        let rx = self.bind_disconnect(store).await?;
        rx.await?;
        Ok(())
    }
}

#[async_trait(?Send)]
pub trait ServiceMsg {
    type In: FromWasmAbi + 'static + Clone;
    type Out: FromWasmAbi + 'static;

    fn mount_onmsg(&self, cbk: &Closure<dyn FnMut(Self::In)>);

    async fn bind_onmsg(
        &self,
        store: &mut ServiceStore<Self::In, Self::Out>,
    ) -> Result<mpsc::UnboundedReceiver<Self::In>, failure::Error> {
        let (tx, rx) = mpsc::unbounded::<Self::In>();
        let tx_handle = tx.clone();
        let f = move |msg: Self::In| {
            let mut tx = tx_handle.clone();
            let fut = async move {
                tx.send(msg).await.expect("unable to send");
            };
            spawn_local(fut);
        };
        let cbk = Closure::wrap(Box::new(f) as Box<dyn FnMut(Self::In)>);
        self.mount_onmsg(&cbk);
        store.set_onmsg(cbk)?;

        Ok(rx)
    }

    async fn processing(
        &self,
        store: &mut ServiceStore<Self::In, Self::Out>,
    ) -> Result<(), failure::Error> {
        let mut rx = self.bind_onmsg(store).await?;
        while let Some(msg) = rx.next().await {
            log::info!("incoming");
            self.broadcast(msg, store).await?;
        }
        Ok(())
    }

    async fn listening(
        &self,
        mut rx: mpsc::UnboundedReceiver<String>,
    ) -> Result<(), failure::Error> {
        while let Some(msg) = rx.next().await {
            self.sending(&msg).await?;
        }

        Ok(())
    }

    async fn sending(&self, msg: &str) -> Result<(), failure::Error>;

    async fn broadcast(
        &self,
        msg: Self::In,
        store: &ServiceStore<Self::In, Self::Out>,
    ) -> Result<(), failure::Error> {
        let subscriptions = store.subscribers.lock().await;
        stream::iter(subscriptions.iter())
            .for_each_concurrent(None, |mut tx| {
                let msg = msg.clone();
                async move {
                    tx.send(msg).await.expect("unable to broadcast to remote");
                }
            })
            .await;
        Ok(())
    }

    async fn combined(
        &self,
        mut store: &mut ServiceStore<Self::In, Self::Out>,
        rx: Receiver<String>,
    ) {
        let joined = futures::future::join(self.processing(&mut store), self.listening(rx));
        let _ = joined.await;
    }
}

pub trait ServiceInterface<In, Out>:
    ServiceConnect<In, Out> + ServiceDisconnect<In, Out> + ServiceMsg<In = In, Out = Out>
where
    In: FromWasmAbi + 'static + Clone,
    Out: FromWasmAbi + 'static,
{
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::tests::init_test;

    use wasm_bindgen_test::wasm_bindgen_test;

    #[wasm_bindgen_test]
    async fn test_dummy_service() {
        init_test();

        let client = ws::Client {
            client: None,
            url: "ws://127.0.0.1:5000".to_string(),
        };

        let store: ServiceStore<MessageEvent, String> = ServiceStore::new();

        let mut server: Service<MessageEvent, String> = Service {
            client: Box::new(client),
            store,
        };

        let (tx, mut rx) = mpsc::unbounded::<MessageEvent>();
        let listening = async {
            while let Some(msg) = rx.next().await {
                log::info!("incoming msg!!!!, {:?}", msg.data());
            }
        };
        {
            let mut subscriptions = server.store.subscribers.lock().await;
            subscriptions.push(tx);
        }

        server
            .client
            .connect(&mut server.store)
            .await
            .expect("open failed");
        log::info!("connected");
        let task = server.client.processing(&mut server.store);
        let joined = futures::future::join(listening, task);
        let delayd = async {
            crate::utils::sleep(10000).await.expect("can't sleep");
            log::info!("wait complete");
        };
        // delayd.await;
        let mega_joined = futures::future::join(delayd, joined);
        let _ = mega_joined.await;
    }
}

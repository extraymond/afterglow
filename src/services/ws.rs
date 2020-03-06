use super::*;
use async_trait::async_trait;

use web_sys::{MessageEvent, WebSocket};

pub struct Client {
    pub client: Option<WebSocket>,
    pub url: String,
}

#[async_trait(?Send)]
impl ServiceConnect<MessageEvent, String> for Client {
    fn mount_onopen(&self, cbk: &Closure<dyn FnMut()>) {
        if let Some(target) = &self.client {
            target.set_onopen(Some(cbk.as_ref().unchecked_ref()));
        }
    }
    async fn dial(&mut self) -> Result<(), failure::Error> {
        let client = WebSocket::new(&self.url).expect("unable to dial");
        self.client = Some(client);
        Ok(())
    }
}

impl ServiceDisconnect<MessageEvent, String> for Client {
    fn mount_onclose(&self, cbk: &Closure<dyn FnMut()>) {
        if let Some(target) = &self.client {
            target.set_onclose(Some(cbk.as_ref().unchecked_ref()));
        }
    }
}

#[async_trait(?Send)]
impl ServiceMsg for Client {
    type In = MessageEvent;
    type Out = String;

    async fn sending(&self, msg: &str) -> Result<(), failure::Error> {
        if let Some(client) = &self.client {
            client.send_with_str(msg).expect("unable to send msg");
        }

        Ok(())
    }

    fn mount_onmsg(&self, cbk: &Closure<dyn FnMut(Self::In)>) {
        if let Some(target) = &self.client {
            target.set_onmessage(Some(cbk.as_ref().unchecked_ref()));
        }
    }
}

impl ServiceInterface<MessageEvent, String> for Client {}

#[cfg(test)]
pub mod test {
    use super::*;
    use crate::tests::init_test;

    use wasm_bindgen_test::wasm_bindgen_test;

    // #[wasm_bindgen_test]
    async fn test_dummy_service() {
        init_test();

        let client = ws::Client {
            client: None,
            url: "ws://127.0.0.1:5000".to_string(),
        };

        let mut store: ServiceStore<MessageEvent, String> = ServiceStore::new();

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
        let rx = server
            .client
            .bind_disconnect(&mut server.store)
            .expect("not able to get onclose rx");
        rx.await.expect("not closing signal");
        // use futures::future::{AbortHandle, Abortable, Aborted};
        // let task = { server.client.processing(&mut server.store) };
        // let (handle, registrator) = AbortHandle::new_pair();
        // let fut = Abortable::new(task, registrator);
        // let dc = async {
        //     log::info!("wait for close signal");
        //     dc.await.expect("can't receive closing event");
        //     handle.abort();
        //     log::info!("receive closing signal");
        // };
        // let joined = futures::future::join(fut, dc);
        // joined.await;
    }
}

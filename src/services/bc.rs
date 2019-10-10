use super::*;

use web_sys::BroadcastChannel;

pub struct Client {
    client: Option<BroadcastChannel>,
    channel_id: String,
}

#[async_trait(?Send)]
impl ServiceConnect<MessageEvent, MessageEvent> for Client {
    async fn dial(&mut self) -> Result<(), failure::Error> {
        let channel = BroadcastChannel::new(&self.channel_id).expect("unable to create channel");
        self.client = Some(channel);
        Ok(())
    }

    fn mount_onopen(&self, _: &Closure<dyn FnMut()>) {}
}

impl ServiceDisconnect<MessageEvent, MessageEvent> for Client {
    fn mount_onclose(&self, _: &Closure<dyn FnMut()>) {}
}

#[async_trait(?Send)]
impl ServiceMsg for Client {
    type In = MessageEvent;
    type Out = MessageEvent;

    fn mount_onmsg(&self, cbk: &Closure<dyn FnMut(Self::In)>) {
        if let Some(client) = &self.client {
            client.set_onmessage(Some(cbk.as_ref().unchecked_ref()));
        }
    }

    async fn sending(&self, msg: &str) -> Result<(), failure::Error> {
        if let Some(client) = &self.client {
            client
                .post_message(&JsValue::from(msg))
                .expect("unable to send msg");
        }

        Ok(())
    }
}

impl ServiceInterface<MessageEvent, MessageEvent> for Client {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::init_test;
    use wasm_bindgen_test::wasm_bindgen_test;

    #[wasm_bindgen_test]
    async fn test_bc() {
        init_test();

        let client = Client {
            client: None,
            channel_id: "unique".to_string(),
        };

        let store: ServiceStore<MessageEvent, MessageEvent> = ServiceStore::new();

        let mut server: Service<MessageEvent, MessageEvent> = Service {
            client: Box::new(client),
            store,
        };

        let (tx, mut rx) = mpsc::unbounded::<MessageEvent>();
        let listening = async {
            while let Some(msg) = rx.next().await {
                log::info!("incoming msg!!!!, {:?}", msg.data());
            }
        };

        server.client.sending("hey!").await;
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
    }
}

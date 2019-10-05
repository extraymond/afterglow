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

impl ServiceDisconnect<MessageEvent, String> for Client {}

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

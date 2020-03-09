use crate::prelude::*;
use async_trait::async_trait;
use futures::lock::Mutex;
use std::convert::{TryFrom, TryInto};
use std::rc::Rc;

pub trait Link {
    type A: Messenger<Target = Self::C>;
    type B: Messenger<Target = Self::D>;
    type C;
    type D;

    fn init(mut rx: Receiver<Self::A>, data: Rc<Mutex<Self::D>>)
    where
        Self::A: Into<Option<Self::B>>,
    {
        let fut = async move {
            while let Some(msg) = rx.next().await {
                let msg: Option<Self::B> = msg.into();
                if let Some(msg) = msg {
                    let mut data = data.lock().await;
                    msg.update(&mut data);
                }
            }
        };
    }
}

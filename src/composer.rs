use crate::prelude::*;
use async_trait::async_trait;
use futures::lock::Mutex;
use std::rc::Rc;

#[async_trait(?Send)]
pub trait Worklets {
    type Target;

    async fn operate(
        &self,
        data: Rc<Mutex<Self::Target>>,
        msg: Message<Self::Target>,
        sender: MessageSender<Self::Target>,
        render_tx: Sender<()>,
    ) where
        Self: Sized,
    {
        let mut data = data.lock().await;
        msg.update(&mut data, sender, render_tx);
    }
}

#[cfg(test)]
mod test {
    pub struct JobA;

    #[async_trait(?Send)]
    impl Worklets for JobA {
        type Target = i32;
    }

    pub async fn compose() {
        // let data_handle = Rc::new(Mutex::new(0_i32));
        // log::info!("initial value, {:?}", data_handle.lock().await);
        // let job1 = JobA.operate(data_handle.clone());
        // let job2 = JobA.operate(data_handle.clone());

        // stream::iter(&[job1, job2]).collect::<Vec<_>>().await;
        // log::info!("final value, {:?}", data_handle.lock().await);
    }
}

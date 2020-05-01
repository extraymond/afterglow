use anyhow::Result;
use futures::channel::{
    mpsc::{unbounded, UnboundedReceiver as Receiver, UnboundedSender as Sender},
    oneshot,
};
use futures::lock::Mutex;
use futures::prelude::*;
use gloo::events::EventListener;
use std::rc::Rc;
use wasm_bindgen::{prelude::*, JsCast};
use wasm_bindgen_futures::*;
use web_sys::{Blob, BlobPropertyBag, Url, Worker};

pub struct Pool {
    href: String,
    name_of_resource: String,
    entry_point: String,
    pub workers: Vec<WorkerHandle>,
}

impl Pool {
    pub fn new(name_of_resource: &str, entry_point: &str) -> Pool {
        let win = web_sys::window().unwrap();
        let url = win.location();
        let href = url.href().unwrap();
        Pool {
            href,
            name_of_resource: name_of_resource.into(),
            entry_point: entry_point.into(),
            workers: vec![],
        }
    }

    pub fn worker_new(&mut self) -> Result<(), js_sys::Error> {
        let handle = WorkerHandle::new(&self)?;
        self.workers.push(handle);
        Ok(())
    }
}

pub struct WorkerHandle {
    pub worker: web_sys::Worker,
    pub state: Rc<Mutex<bool>>,
}

impl WorkerHandle {
    pub fn new(pool: &Pool) -> Result<Self, js_sys::Error> {
        let script_url = format!("{}{}", pool.href, pool.name_of_resource);
        let wasm_url = format!(
            "{}{}",
            pool.href,
            pool.name_of_resource.replace(".js", "_bg.wasm")
        );
        let array = js_sys::Array::new();
        array.push(
            &format!(
                r#"
                importScripts("{}");
                const {{ {} }} = wasm_bindgen;
                wasm_bindgen("{}").then(()=> {{
                    {}();
                }});
            "#,
                script_url, &pool.entry_point, wasm_url, &pool.entry_point
            )
            .into(),
        );
        let blob = Blob::new_with_str_sequence_and_options(
            &array,
            BlobPropertyBag::new().type_("application/javascript"),
        )
        .unwrap();
        let url = Url::create_object_url_with_blob(&blob).unwrap();

        let worker = Worker::new(&url)?;
        let (tx, mut rx) = unbounded::<web_sys::MessageEvent>();
        let onmsg = EventListener::new(&worker, "message", move |e| {
            let mut tx = tx.clone();
            let e = e.clone().unchecked_into();
            spawn_local(async move {
                let _ = tx.send(e).await;
            });
        });

        let state = Rc::new(Mutex::new(false));
        let state_clone = state.clone();
        let worker_clone = worker.clone();
        spawn_local(async move {
            let _ = rx.next().await;
            log::info!("worker's first message");

            let mut state = state_clone.lock().await;
            *state = true;
            rx.for_each(|_| async {
                log::info!("incoming");
            })
            .await;
        });
        onmsg.forget();
        Ok(WorkerHandle {
            worker,
            state: Rc::new(Mutex::new(false)),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;
    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_pool() {
        let mut pool = Pool::new("afterglow_worker.js", "init_child");
        assert!(pool.worker_new().is_ok());
    }
}

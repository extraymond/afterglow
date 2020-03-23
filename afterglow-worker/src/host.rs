use gloo::events::EventListener;
use wasm_bindgen::{prelude::*, JsCast};
use web_sys::{Blob, BlobPropertyBag, Url, Worker};

pub fn worker_new(resource_path: &str, name_of_resource: &str, entry_point: &str) -> Worker {
    let script_url = format!("{}{}", resource_path, name_of_resource);
    let wasm_url = format!(
        "{}{}",
        resource_path,
        name_of_resource.replace(".js", "_bg.wasm")
    );
    log::info!("script url: {}", script_url);
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
            script_url, &entry_point, wasm_url, &entry_point
        )
        .into(),
    );
    let blob = Blob::new_with_str_sequence_and_options(
        &array,
        BlobPropertyBag::new().type_("application/javascript"),
    )
    .unwrap();
    let url = Url::create_object_url_with_blob(&blob).unwrap();
    log::info!("{:?}", url);

    Worker::new(&url).expect("failed to spawn worker")
}

pub fn init_host() {
    let _ = femme::start(log::LevelFilter::Info);
    let win = web_sys::window().unwrap();
    let url = win.location();
    let href = url.href().unwrap();
    let worker = worker_new(&href, "afterglow_worker.js", "worker_start");
    let msg = JsValue::from_str("hello worker");

    let target: &web_sys::EventTarget = worker.unchecked_ref();
    let onmsg = EventListener::new(&target, "message", |e| {
        log::info!("worker called");
    });
    onmsg.forget();
    worker.post_message(&msg).unwrap();
}

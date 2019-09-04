use js_sys::Promise;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::futures_0_3::JsFuture;
use web_sys::{window, WorkerGlobalScope};

pub async fn timer(ms: i32) -> Result<(), JsValue> {
    let promise = Promise::new(&mut |yes, _| {
        let win = window().unwrap();
        win.set_timeout_with_callback_and_timeout_and_arguments_0(&yes, ms)
            .unwrap();
    });
    let js_fut = JsFuture::from(promise);
    js_fut.await?;
    Ok(())
}

pub async fn worker_timer(ms: i32) -> Result<(), JsValue> {
    let promise = Promise::new(&mut |yes, _| {
        let global = js_sys::global();
        let scope = global.dyn_into::<WorkerGlobalScope>().unwrap();
        scope
            .set_timeout_with_callback_and_timeout_and_arguments_0(&yes, ms)
            .unwrap();
    });
    let js_fut = JsFuture::from(promise);
    js_fut.await?;
    Ok(())
}

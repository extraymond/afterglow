

use js_sys::Promise;
use wasm_bindgen::{JsCast, JsValue};

use wasm_bindgen_futures::futures_0_3::JsFuture;


pub async fn sleep(ms: i32) -> Result<(), JsValue> {
    let promise = Promise::new(&mut |yes, _| {
        let global = js_sys::global();
        let scope = global.unchecked_into::<web_sys::Window>();

        scope
            .set_timeout_with_callback_and_timeout_and_arguments_0(&yes, ms)
            .unwrap();
    });
    let js_fut = JsFuture::from(promise);
    js_fut.await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;
}

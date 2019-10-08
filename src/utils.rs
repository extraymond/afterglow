use core::pin::Pin;
use core::task::{Context, Poll};
use js_sys::Promise;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::futures_0_3::spawn_local;
use wasm_bindgen_futures::futures_0_3::JsFuture;
use web_sys::{Window, WorkerGlobalScope};

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

    // #[wasm_bindgen_test]
    // async fn test_timer() {
    //     crate::tests::init_test();
    //     for i in 1..50 {
    //         let duration = 100 * i;
    //         let start = js_sys::Date::new_0().get_time();
    //         Timer::sleep(duration).await.expect("not able to sleep");
    //         let end = js_sys::Date::new_0().get_time();
    //         let offset = end - start;
    //         log::info!(
    //             "sleep for:{}, offset:{}",
    //             duration,
    //             offset - f64::from(duration)
    //         );
    //         assert!(offset - f64::from(duration) <= 10.0);
    //     }
    // }
    //
    // #[wasm_bindgen_test]
    // async fn test_timer_concurrent() {
    //     crate::tests::init_test();
    //     for max in 1..=20 {
    //         for frame_time in 10..=20 {
    //             let tasks = futures::future::join_all((1..=max).map(|i| {
    //                 let duration = frame_time * i;
    //                 Timer::sleep(duration)
    //             }));
    //             let start = js_sys::Date::new_0().get_time();
    //             tasks.await;
    //             let end = js_sys::Date::new_0().get_time();
    //             let offset = end - start;
    //             let off = offset - f64::from(max * frame_time);
    //             log::info!("total offset: {}", off);
    //             assert!(off < 200.0);
    //         }
    //     }
    // }
}

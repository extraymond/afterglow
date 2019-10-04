use core::pin::Pin;
use core::task::{Context, Poll};
use js_sys::Promise;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::futures_0_3::spawn_local;
use wasm_bindgen_futures::futures_0_3::JsFuture;
use web_sys::{Window, WorkerGlobalScope};

enum TimerScope {
    _window(Window),
    _worker(WorkerGlobalScope),
}

pub struct Timer {
    scope: TimerScope,
}

impl Timer {
    fn _init() -> Result<Self, JsValue> {
        let global = js_sys::global();
        let _scope = global.clone().unchecked_into::<Window>();
        if _scope.has_type::<Window>() {
            let scope = TimerScope::_window(_scope);
            Ok(Timer { scope })
        } else {
            let _scope = global.clone().dyn_into::<WorkerGlobalScope>()?;
            let scope = TimerScope::_worker(_scope);
            Ok(Timer { scope })
        }
    }

    async fn _sleep(&self, ms: i32) -> Result<(), JsValue> {
        let promise = Promise::new(&mut |yes, _| match &self.scope {
            TimerScope::_window(scope) => {
                scope
                    .set_timeout_with_callback_and_timeout_and_arguments_0(&yes, ms)
                    .unwrap();
            }
            TimerScope::_worker(scope) => {
                scope
                    .set_timeout_with_callback_and_timeout_and_arguments_0(&yes, ms)
                    .unwrap();
            }
        });
        let js_fut = JsFuture::from(promise);
        js_fut.await?;
        Ok(())
    }

    pub async fn sleep(ms: i32) -> Result<(), JsValue> {
        let timer = Timer::_init().expect("can't find correct scope to setTimeout");
        timer._sleep(ms).await?;
        Ok(())
    }
}

pub trait Sleeper {
    fn sleep(&self, ms: i32) -> dyn futures::future::Future<Output = Result<(), JsValue>>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    #[wasm_bindgen_test]
    async fn test_timer() {
        crate::tests::init_test();
        for i in 1..50 {
            let duration = 100 * i;
            let start = js_sys::Date::new_0().get_time();
            Timer::sleep(duration).await.expect("not able to sleep");
            let end = js_sys::Date::new_0().get_time();
            let offset = end - start;
            log::info!(
                "sleep for:{}, offset:{}",
                duration,
                offset - f64::from(duration)
            );
            assert!(offset - f64::from(duration) <= 10.0);
        }
    }

    #[wasm_bindgen_test]
    async fn test_timer_concurrent() {
        crate::tests::init_test();
        for max in 1..100 {
            for frame_time in 10..100 {
                let tasks = futures::future::join_all((1..=max).map(|i| {
                    let duration = frame_time * i;
                    Timer::sleep(duration)
                }));
                let start = js_sys::Date::new_0().get_time();
                tasks.await;
                let end = js_sys::Date::new_0().get_time();
                let offset = end - start;
                let off = offset - f64::from(max * frame_time);
                log::info!("total offset: {}", off);
                assert!(off < 200.0)
            }
        }
    }
}

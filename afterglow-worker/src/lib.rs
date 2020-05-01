// Helper traits to let client launc the service.
pub mod host;
// Helper traits to help design worker.
pub mod servant;

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::*;

#[wasm_bindgen]
pub fn host_start() {
    let _ = femme::start(log::LevelFilter::Info);
    // let mut pool = host::Pool::new("afterglow_worker.js", "worker_start");
    // pool.worker_new().expect("failed to start worker");

    spawn_local(async {
        if let Err(e) = gen_bitmap().await {
            log::warn!("{:?}", e);
        }
    });
}

pub async fn gen_bitmap() -> Result<(), js_sys::Error> {
    let window = web_sys::window().unwrap();
    let doc = window.document().unwrap();
    let cvs = doc.get_element_by_id("board").unwrap();
    let mut buffer = [0_u8; 300 * 300 * 4];
    for (pixel, buf) in buffer.chunks_mut(4).enumerate() {
        let x = pixel / 300;
        let y = pixel - 300 * x;
        if x < 100 {
            if y < 10 {
                buf[3] = 255;
            }
        }
        // log::info!("val:{:?}", buf);
        //  for every idx in array: it can be represented as rgba_idx * w_idx * y*idx
    }

    let img_data = web_sys::ImageData::new_with_u8_clamped_array_and_sh(
        wasm_bindgen::Clamped(&mut buffer),
        300,
        300,
    )?;

    // let promise = window.create_image_bitmap_with_image_data(&mut img_data)?;
    // let res = JsFuture::from(promise).await?;
    // let bitmap = res.unchecked_into::<web_sys::ImageBitmap>();

    let ctx = cvs
        .unchecked_into::<web_sys::HtmlCanvasElement>()
        .get_context("2d")?
        .map(|node| node.unchecked_into::<web_sys::CanvasRenderingContext2d>())
        .unwrap();
    // // ctx.fill_rect(0.0, 0.0, 10.0, 10.0);

    // // let buffer = ctx.get_image_data(0.0, 0.0, 10.0, 10.0).unwrap();
    ctx.put_image_data(&img_data, 0.0, 0.0)?;
    // ctx.draw_image_with_image_bitmap(&bitmap, 0.0, 0.0)?;
    //
    // log::info!("{:?}", buffer.data());
    Ok(())
}

#[wasm_bindgen]
pub fn worker_start() {
    servant::init_worker();
}

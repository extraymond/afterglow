// Helper traits to let client launc the service.
pub mod host;
// Helper traits to help design worker.
pub mod servant;

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn host_start() {
    host::init_host();
}

#[wasm_bindgen]
pub fn worker_start() {
    servant::init_worker();
}

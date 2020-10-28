pub mod bus;
pub mod container;
pub mod messenger;
pub mod prelude;
pub mod renderer;

#[cfg(test)]
mod tests {
    use wasm_bindgen_test::*;
    wasm_bindgen_test_configure!(run_in_browser);

    pub fn init_test() {
        let _ = femme::with_level(log::LevelFilter::Info);
    }
}

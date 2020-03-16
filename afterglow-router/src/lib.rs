use afterglow::prelude::*;
use std::collections::HashMap;
use std::marker::PhantomData;

pub struct Route<T, R>(pub PhantomData<(T, R)>)
where
    T: LifeCycle;
impl<T, R> Default for Route<T, R>
where
    T: LifeCycle,
    R: Renderer<Target = T, Data = T>,
{
    fn default() -> Self {
        Route(PhantomData::default())
    }
}

pub trait Routable {
    fn serve(&self, block: &web_sys::HtmlElement);
}

impl<T, R> Routable for Route<T, R>
where
    T: LifeCycle + 'static,
    R: Renderer<Target = T, Data = T> + Default + 'static,
{
    fn serve(&self, block: &web_sys::HtmlElement) {
        let mut entry = Entry::new();
        let data = T::new(entry.render_tx.clone());
        entry.mount_vdom(data, block, Box::new(R::default()));
    }
}

#[derive(Default)]
pub struct Router {
    pub routes: HashMap<String, Box<dyn Routable>>,
}

impl Router {
    pub fn at<T, R>(mut self, path: &str) -> Self
    where
        T: LifeCycle + 'static,
        R: Renderer<Target = T, Data = T> + Default + 'static,
    {
        let route = Box::new(Route::<T, R>::default());
        self.routes.insert(path.into(), route);
        self
    }

    pub fn route(&self, path: &str, block: &web_sys::HtmlElement) {
        if let Some(route) = self.routes.get(path) {
            route.serve(block);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    pub struct Model;
    impl LifeCycle for Model {
        fn new(render_tx: Sender<()>) -> Self {
            Model
        }
    }

    #[derive(Default)]
    pub struct View;
    impl Renderer for View {
        type Target = Model;
        type Data = Model;

        fn view<'a>(
            &self,
            target: &Self::Target,
            ctx: &mut RenderContext<'a>,
            sender: MessageSender<Self::Data>,
        ) -> Node<'a> {
            let bump = ctx.bump;
            dodrio!(bump, <div>"this is model!!!!!!!"</div>)
        }
    }

    #[derive(Default)]
    pub struct Dummy {
        pub arr: [[[i32; 10]; 10]; 10],
    }
    impl LifeCycle for Dummy {
        fn new(render_tx: Sender<()>) -> Self {
            Dummy::default()
        }
    }

    #[derive(Default)]
    pub struct DummyView;
    impl Renderer for DummyView {
        type Target = Dummy;
        type Data = Dummy;

        fn view<'a>(
            &self,
            target: &Self::Target,
            ctx: &mut RenderContext<'a>,
            sender: MessageSender<Self::Data>,
        ) -> Node<'a> {
            let bump = ctx.bump;
            dodrio!(bump, <div>"this is dummy!!!!!!!"</div>)
        }
    }

    use wasm_bindgen_test::*;
    wasm_bindgen_test_configure!(run_in_browser);
    use futures_timer::Delay;
    use std::time::Duration;

    #[wasm_bindgen_test]
    fn test_router() {
        let mut router = Router::default();
        router = router
            .at::<Model, View>("model")
            .at::<Dummy, DummyView>("dummy");
        let block: web_sys::HtmlElement = web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .body()
            .unwrap()
            .unchecked_into();

        spawn_local(async move {
            router.route("model", &block);
            Delay::new(Duration::from_secs(2)).await;
            router.route("dummy", &block);
        });
    }
}

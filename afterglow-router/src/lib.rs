use afterglow::prelude::*;
use std::collections::HashMap;
use std::marker::PhantomData;
use url::Url;

pub struct Route<T, R>(PhantomData<(T, R)>)
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

pub struct Router {
    pub routes: HashMap<String, Box<dyn Routable>>,
    rx: Receiver<web_sys::Event>,
    _onhashchange: EventListener,
}

impl Default for Router {
    fn default() -> Self {
        let win = web_sys::window()
            .unwrap()
            .unchecked_into::<web_sys::EventTarget>();

        let (tx, rx) = mpsc::unbounded::<web_sys::Event>();

        let _onhashchange = EventListener::new(&win, "hashchange", move |e| {
            let e = e.clone();
            let mut tx = tx.clone();
            spawn_local(async move {
                tx.send(e).await.unwrap();
            });
        });

        Router {
            routes: HashMap::new(),
            rx,
            _onhashchange,
        }
    }
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

    pub async fn handling(&mut self, block: &web_sys::HtmlElement) {
        while let Some(e) = self.rx.next().await {
            let e = e.unchecked_into::<web_sys::HashChangeEvent>();
            if let Ok(path) = Url::parse(&e.new_url()) {
                if let Some(frag) = path.fragment() {
                    self.route(frag, &block);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use wasm_bindgen_test::*;
    wasm_bindgen_test_configure!(run_in_browser);
    pub struct Model;
    impl LifeCycle for Model {
        fn new(render_tx: Sender<()>) -> Self {
            Model
        }
    }

    #[derive(Default)]
    pub struct Mega {
        model: Option<Container<Model>>,
        dummy: Option<Container<Dummy>>,
    }

    impl LifeCycle for Mega {
        fn new(render_tx: Sender<()>) -> Self {
            let model = Container::new(
                Model::new(render_tx.clone()),
                Box::new(View),
                render_tx.clone(),
            );
            let model = Some(model);
            let dummy = Container::new(
                Dummy::new(render_tx.clone()),
                Box::new(DummyView),
                render_tx.clone(),
            );
            let dummy = Some(dummy);

            Mega { model, dummy }
        }
    }

    #[derive(Default)]
    pub struct MegaView;
    impl Renderer for MegaView {
        type Target = Mega;
        type Data = Mega;

        fn view<'a>(
            &self,
            target: &Self::Target,
            ctx: &mut RenderContext<'a>,
            sender: MessageSender<Self::Data>,
        ) -> Node<'a> {
            let bump = ctx.bump;

            dodrio!(bump,
                <div class="card">
                    <div class="box">{ target.model.as_ref().map(|v| v.render(ctx))}</div>
                    <div class="box">{ target.dummy.as_ref().map(|v| v.render(ctx))}</div>
                    <a class="button" onclick={ consume(|_| { MegaMsg::RemoveMega},  &sender) }>"remove model"</a>
                </div>
            )
        }
    }

    pub enum MegaMsg {
        RemoveMega,
    }
    impl Messenger for MegaMsg {
        type Target = Mega;
        fn update(
            &self,
            target: &mut Self::Target,
            sender: MessageSender<Self::Target>,
            render_tx: Sender<()>,
        ) -> bool {
            target.model = None;
            true
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

    #[wasm_bindgen_test]
    fn test_router() {
        let _ = femme::start(log::LevelFilter::Info);
        let mut router = Router::default();
        router = router
            .at::<Model, View>("model")
            .at::<Dummy, DummyView>("dummy")
            .at::<Mega, MegaView>("mega");

        let block: web_sys::HtmlElement = web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .body()
            .unwrap()
            .unchecked_into();

        spawn_local(async move {
            router.route("mega", &block);
            router.handling(&block).await;
        });
    }
}

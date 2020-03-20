use afterglow::prelude::*;
use async_executors::*;
use async_trait::async_trait;
use std::cell::RefCell;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::rc::Rc;
use url::Url;

/// A route that stores type information about the container and it's default renderer
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

pub enum RouteEvent {
    RouteChanged([url::Url; 2]),
}

#[async_trait(?Send)]
/// Route
pub trait Routable {
    async fn serve(&self, tag: Option<&str>) -> Entry;
}

#[async_trait(?Send)]
impl<T, R> Routable for Route<T, R>
where
    T: LifeCycle + 'static,
    R: Renderer<Target = T, Data = T> + Default + 'static,
{
    async fn serve(&self, tag: Option<&str>) -> Entry {
        Entry::init_app::<T, R>(tag)
    }
}

pub struct Router {
    pub entry: Option<Entry>,
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
            entry: None,
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

    pub async fn routing(&mut self, path: &str, tag: Option<&str>) {
        if let Some(route) = self.routes.get(path) {
            if let Some(old_entry) = self.entry.as_mut() {
                let (tx, rx) = oneshot::channel::<()>();
                let _ = old_entry.msg_tx.send(EntryMessage::Eject(tx)).await;
                let _ = rx.await;
            }
            self.entry.replace(route.serve(tag).await);
        }
    }

    pub async fn handling(&mut self, tag: Option<&str>) {
        while let Some(e) = self.rx.next().await {
            let e = e.unchecked_into::<web_sys::HashChangeEvent>();
            if let Ok(path) = Url::parse(&e.new_url()) {
                if let Some(frag) = path.fragment() {
                    self.routing(frag, tag).await;
                }
            }
        }
    }

    pub fn init_router(&mut self) {}
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
        let _ = femme::start(log::LevelFilter::Trace);
        let mut router = Router::default();
        router = router
            .at::<Model, View>("model")
            .at::<Dummy, DummyView>("dummy")
            .at::<Mega, MegaView>("mega");

        spawn_local(async move {
            router.routing("mega", Some("app")).await;
            router.handling(Some("app")).await;
        });
    }
}

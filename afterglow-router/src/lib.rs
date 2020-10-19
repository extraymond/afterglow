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

#[derive(Clone)]
pub enum RouteEvent {
    Manual(String),
    Native(web_sys::Event),
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
    path: Option<String>,
    pub entry: Option<Entry>,
    pub routes: HashMap<String, Box<dyn Routable>>,
    rx: Receiver<RouteEvent>,
    onpopstate: EventListener,
    onroutechange: EventListener,
}

impl Default for Router {
    fn default() -> Self {
        let mut state = HashMap::<&str, &str>::new();
        state.insert("path", "");

        let win = web_sys::window()
            .unwrap()
            .unchecked_into::<web_sys::EventTarget>();

        let (tx, rx) = mpsc::unbounded::<RouteEvent>();

        let tx_clone = tx.clone();
        let onpopstate = EventListener::new(&win, "popstate", move |e| {
            let e = e.clone();
            let mut tx = tx_clone.clone();
            spawn_local(async move {
                tx.send(RouteEvent::Native(e)).await.unwrap();
            });
        });

        let onroutechange = EventListener::new(&win, "routechange", move |e| {
            let content = e
                .clone()
                .unchecked_into::<web_sys::CustomEvent>()
                .detail()
                .as_string()
                .unwrap();
            let mut tx = tx.clone();
            spawn_local(async move {
                tx.send(RouteEvent::Manual(content)).await.unwrap();
            });
        });

        Router {
            path: None,
            entry: None,
            routes: HashMap::new(),
            rx,
            onpopstate,
            onroutechange,
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

    pub async fn routing(&mut self, path: &str, tag: Option<&str>) -> bool {
        if let Some(route) = self.routes.get(path) {
            if let Some(old_entry) = self.entry.as_mut() {
                let (tx, rx) = oneshot::channel::<()>();
                let _ = old_entry.msg_tx.send(EntryMessage::Eject(tx)).await;
                let _ = rx.await;
            }
            self.entry.replace(route.serve(tag).await);
            true
        } else {
            false
        }
    }

    pub async fn handling(&mut self, tag: Option<&str>) {
        let win = web_sys::window().unwrap();
        emit_route("");

        let history = win.history().unwrap();

        while let Some(e) = self.rx.next().await {
            match e {
                // native is for going back and match valid path.
                RouteEvent::Native(e) => {
                    log::info!("browser routing");
                    let e = e.unchecked_into::<web_sys::PopStateEvent>();
                    if let Ok(state) =
                        serde_wasm_bindgen::from_value::<HashMap<String, String>>(e.state())
                    {
                        if let Some(path) = state.get("path") {
                            self.routing(path, tag).await;
                        }
                    }
                }

                // manual is used to push through new destination.
                RouteEvent::Manual(path) => {
                    log::info!("manual routing");
                    if self.routing(&path, tag).await {
                        let mut state = HashMap::new();
                        state.insert("path", path.clone());
                        history
                            .push_state_with_url(
                                &serde_wasm_bindgen::to_value(&state).unwrap(),
                                "",
                                Some(&path),
                            )
                            .unwrap();
                    }
                }
            }
        }
    }
}

pub fn emit_route(path: &str) {
    let win = web_sys::window().unwrap();
    let target = win.clone().unchecked_into::<web_sys::EventTarget>();

    let mut init = web_sys::CustomEventInit::new();
    init.detail(&JsValue::from_str(path));
    let event = web_sys::CustomEvent::new_with_event_init_dict("routechange", &init).unwrap();
    target
        .dispatch_event(&event.unchecked_into::<web_sys::Event>())
        .unwrap();
}

use afterglow::prelude::dodrio::{RootRender, VdomWeak};
pub fn route_to(path: &str) -> impl Fn(&mut dyn RootRender, VdomWeak, Event) + 'static {
    let path = path.to_string();
    move |_, _, _| {
        emit_route(&path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use wasm_bindgen_test::*;
    wasm_bindgen_test_configure!(run_in_browser);
    pub struct Model;
    impl LifeCycle for Model {
        fn new(render_tx: Sender<((), oneshot::Sender<()>)>) -> Self {
            Model
        }
    }

    #[derive(Default)]
    pub struct Mega {
        model: Option<Container<Model>>,
        dummy: Option<Container<Dummy>>,
    }

    impl LifeCycle for Mega {
        fn new(render_tx: Sender<((), oneshot::Sender<()>)>) -> Self {
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
            sender: &MessageSender<Self::Data>,
        ) -> Node<'a> {
            let bump = ctx.bump;

            dodrio!(bump,
                <div class="card">
                    <div class="box">{ target.model.as_ref().map(|v| v.render(ctx))}</div>
                    <a onclick={ route_to("dummy") }>"go to dummy"</a>
                    <div class="box">{ target.dummy.as_ref().map(|v| v.render(ctx))}</div>
                    <a class="button" onclick={ consume(|_| { MegaMsg::RemoveMega },  &sender) }>"remove model"</a>
                </div>
            )
        }
    }

    pub enum MegaMsg {
        RemoveMega,
        Clicked,
    }
    impl Messenger for MegaMsg {
        type Target = Mega;
        fn update(
            self: Box<Self>,
            target: &mut Self::Target,
            sender: &MessageSender<Self::Target>,
            render_tx: &Sender<((), oneshot::Sender<()>)>,
        ) -> bool {
            match *self {
                MegaMsg::RemoveMega => {
                    target.model = None;
                    return true;
                }
                _ => {}
            }
            false
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
            sender: &MessageSender<Self::Data>,
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
        fn new(render_tx: Sender<((), oneshot::Sender<()>)>) -> Self {
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
            sender: &MessageSender<Self::Data>,
        ) -> Node<'a> {
            let bump = ctx.bump;
            dodrio!(bump, <div onclick={ route_to("mega") } >"this is dummy!!!!!!!"</div>)
        }
    }

    #[wasm_bindgen_test]
    fn test_router() {
        let _ = femme::start(log::LevelFilter::Info);
        let mut router = Router::default();
        router = router
            .at::<Model, View>("model")
            .at::<Dummy, DummyView>("dummy")
            .at::<Mega, MegaView>("")
            .at::<Mega, MegaView>("mega");

        spawn_local(async move {
            router.handling(None).await;
        });
    }
}

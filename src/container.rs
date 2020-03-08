use crate::prelude::*;
use dodrio::Vdom;
use futures::lock::Mutex;
use gloo::events::EventListener;
use std::rc::Rc;

pub struct Container<T, P> {
    pub data: Rc<Mutex<T>>,
    pub sender: Sender<Box<dyn Messenger<Target = T>>>,
    pub renderer: Box<dyn Renderer<Target = T, Data = T>>,
    pub render_tx: Sender<()>,
    pub handlers: Vec<EventListener>,
    pub parent_sender: Option<Sender<Box<dyn Messenger<Target = P>>>>,
}

pub trait LifeCycle {
    fn new(render_tx: Sender<()>) -> Self;
    fn mounted(
        sender: Sender<Box<dyn Messenger<Target = Self>>>,
        render_tx: Sender<()>,
        handlers: &mut Vec<EventListener>,
    ) {
    }
}

impl<T, P> Container<T, P>
where
    T: LifeCycle,
{
    pub fn new(
        data: T,
        renderer: Box<dyn Renderer<Target = T, Data = T>>,
        render_tx: Sender<()>,
    ) -> Self
    where
        T: 'static,
    {
        let (sender, receiver) = mpsc::unbounded::<Box<dyn Messenger<Target = T>>>();
        let mut container = Container {
            data: Rc::new(Mutex::new(data)),
            sender,
            renderer,
            render_tx,
            handlers: vec![],
            parent_sender: None,
        };
        container.init_messenger(receiver);
        <T as LifeCycle>::mounted(
            container.sender.clone(),
            container.render_tx.clone(),
            &mut container.handlers,
        );
        container
    }

    pub fn init_messenger(&self, mut rx: Receiver<Box<dyn Messenger<Target = T>>>)
    where
        T: 'static,
    {
        let data = self.data.clone();
        let mut render_tx = self.render_tx.clone();
        let fut = async move {
            while let Some(msg) = rx.next().await {
                let mut data_inner = data.lock().await;
                if msg.update(&mut *data_inner) {
                    if render_tx.send(()).await.is_err() {
                        break;
                    }
                }
            }
        };
        spawn_local(fut);
    }
}

<<<<<<< HEAD
impl<T> Container<T>
=======
impl<T, P> Container<T, P>
>>>>>>> 90c1d009316e87bc4bbc9f7290becb4d06b0ef30
where
    T: LifeCycle,
{
    pub fn render<'a>(&self, ctx: &mut RenderContext<'a>) -> Node<'a> {
        let bump = ctx.bump;
        if let Some(data) = self.data.try_lock() {
            self.renderer.view(&*data, ctx, self.sender.clone())
        } else {
            dodrio!(bump, <template></template>)
        }
    }
}

<<<<<<< HEAD
=======
impl<T, P> Container<T, P>
where
    T: LifeCycle,
    P: LifeCycle,
{
    pub fn mount_parent_sender(&mut self, parent_sender: Sender<Box<dyn Messenger<Target = P>>>) {
        self.parent_sender.replace(parent_sender);
    }
}

>>>>>>> 90c1d009316e87bc4bbc9f7290becb4d06b0ef30
pub struct Entry {
    pub render_tx: Sender<()>,
    render_rx: Option<Receiver<()>>,
}

impl Entry {
    pub fn new() -> Self {
        let (render_tx, render_rx) = mpsc::unbounded::<()>();

        Entry {
            render_tx,
            render_rx: Some(render_rx),
        }
    }

    pub fn mount_vdom<T: LifeCycle, P: LifeCycle>(
        &mut self,
        data: T,
        block: &web_sys::HtmlElement,
        renderer: Box<dyn Renderer<Target = T, Data = T>>,
    ) where
        T: 'static,
        P: 'static,
    {
        let render_tx = self.render_tx.clone();
        let root_container = Container::<T, P>::new(data, renderer, render_tx.clone());
        let vdom = Vdom::new(&block, root_container);

        if let Some(mut render_rx) = self.render_rx.take() {
            let rendering = async move {
                loop {
                    if render_rx.next().await.is_some() {
                        vdom.weak().render().await.expect("unable to rerender");
                    } else {
                        break;
                    }
                }
            };
            spawn_local(rendering);
        }
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    pub struct Model {
        status: bool,
        embed: Option<Container<Model, Model>>,
    }

    impl LifeCycle for Model {
        fn new(render_tx: Sender<()>) -> Self {
            let embed_model = Model::new(render_tx.clone());
            let embed = Some(Container::new(
                embed_model,
                Box::new(MegaViewer {}),
                render_tx.clone(),
            ));
            Model {
                status: true,
                embed,
            }
        }

        fn mounted(
            mut sender: Sender<Box<dyn Messenger<Target = Self>>>,
            render_tx: Sender<()>,
            handlers: &mut Vec<EventListener>,
        ) {
            spawn_local(async move {
                sender.send(Box::new(ClickEvents::clicked)).await.unwrap();
            });
        }
    }

    pub enum ClickEvents {
        clicked,
    }

    impl Messenger for ClickEvents {
        type Target = Model;

        fn update(&self, target: &mut Self::Target) -> bool {
            match self {
                ClickEvents::clicked => {
                    target.status = !target.status;
                    true
                }
            }
        }
    }

    pub struct RenderAsBox;
    pub struct RenderAsCard;
    pub struct MegaViewer;

    impl Renderer for RenderAsBox {
        type Target = Model;
        type Data = Model;

        fn view<'a>(
            &self,
            target: &Self::Target,
            ctx: &mut RenderContext<'a>,
            sender: Sender<Box<dyn Messenger<Target = Self::Data>>>,
        ) -> Node<'a> {
            use dodrio::builder::text;
            let bump = ctx.bump;
            let value = bf!(in bump, "value in box {}", &target.status).into_bump_str();
            dodrio!(bump,
                <div class="box">
                { vec![text(value)] }
                <div class="button"
                        onclick={
                            let tx = sender.clone();
                            move |_, _, _| {
                                let mut tx = tx.clone();
                                spawn_local(async move {
                                    tx.send(Box::new(ClickEvents::clicked)).await.unwrap();
                                });
                            }
                        }
                        >
                        </div>
                </div>
            )
        }
    }

    impl Renderer for RenderAsCard {
        type Target = Model;
        type Data = Model;

        fn view<'a>(
            &self,
            target: &Self::Target,
            ctx: &mut RenderContext<'a>,
            sender: Sender<Box<dyn Messenger<Target = Self::Data>>>,
        ) -> Node<'a> {
            use dodrio::builder::text;
            let bump = ctx.bump;
            let value = bf!(in bump, "value in card {}", &target.status).into_bump_str();
            dodrio!(bump,
                <div class="card">
                    <div class="card-header">
                        <p class="card-header-title">"this is a card"</p>
                    </div>
                    <div class="card-content">
                        { vec![text(value)] }
                        <div class="button"
                        onclick={
                            let tx = sender.clone();
                            move |_, _, _| {
                                let mut tx = tx.clone();
                                spawn_local(async move {
                                    tx.send(Box::new(ClickEvents::clicked)).await.unwrap();
                                });
                            }
                        }
                        >


                        </div>
                    </div>
                </div>
            )
        }
    }

    impl Renderer for MegaViewer {
        type Target = Model;
        type Data = Model;

        fn view<'a>(
            &self,
            target: &Self::Target,
            ctx: &mut RenderContext<'a>,
            sender: Sender<Box<dyn Messenger<Target = Self::Data>>>,
        ) -> Node<'a> {
            let bump = ctx.bump;
            let card_view = RenderAsCard.view(target, ctx, sender.clone());
            let box_view = RenderAsBox.view(target, ctx, sender.clone());
            let embed_view = target
                .embed
                .as_ref()
                .map(|embed| {
                    let embed_sender = embed.sender.clone();
                    embed
                        .data
                        .try_lock()
                        .map(|model| RenderAsBox.view(&model, ctx, embed_sender))
                })
                .flatten();

            dodrio!(bump,
                <div class="card">
                { card_view }
                // { box_view }
                { embed_view }
                <link rel=typed_html::types::LinkType::StyleSheet href="https://cdnjs.cloudflare.com/ajax/libs/bulma/0.7.5/css/bulma.css"/>
                </div>
            )
        }
    }

    pub fn setup() {
        let embed_data = Model {
            status: false,
            embed: None,
        };

        let block: web_sys::HtmlElement = web_sys::window()
            .map(|win| win.document())
            .flatten()
            .map(|doc| doc.body().unwrap())
            .unwrap();

        let mut entry = Entry::new();
        let embed_container =
            Container::new(embed_data, Box::new(RenderAsBox), entry.render_tx.clone());
        let data = Model {
            status: true,
            embed: Some(embed_container),
        };

        entry.mount_vdom::<_, Model>(data, &block, Box::new(MegaViewer {}));
    }

    use wasm_bindgen_test::*;

    #[wasm_bindgen_test]
    pub fn test_setup() {
        crate::tests::init_test();
        log::info!("start testing");
        setup();
    }
}

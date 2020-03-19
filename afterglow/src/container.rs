use crate::prelude::*;
use dodrio::Vdom;
use futures::lock::Mutex;
use gloo::events::EventListener;
use std::rc::Rc;

pub struct Container<T>
where
    T: LifeCycle,
{
    pub data: Rc<Mutex<T>>,
    pub sender: MessageSender<T>,
    pub renderer: Render<T, T>,
    pub render_tx: Sender<()>,
    pub handlers: Vec<EventListener>,
}

pub trait LifeCycle {
    fn new(render_tx: Sender<()>) -> Self;
    fn mounted(
        sender: MessageSender<Self>,
        render_tx: Sender<()>,
        handlers: &mut Vec<EventListener>,
    ) {
    }

    fn destroyed(&self, sender: MessageSender<Self>, render_tx: Sender<()>) {}
}

impl<T> Drop for Container<T>
where
    T: LifeCycle,
{
    fn drop(&mut self) {
        self.data.try_lock().as_ref().map(|data| {
            data.destroyed(self.sender.clone(), self.render_tx.clone());
        });
    }
}

impl<T> Container<T>
where
    T: LifeCycle + 'static,
{
    pub fn new(data: T, renderer: Render<T, T>, render_tx: Sender<()>) -> Self {
        let (sender, receiver) = mpsc::unbounded::<(Message<T>, oneshot::Sender<()>)>();
        let mut container = Container {
            data: Rc::new(Mutex::new(data)),
            sender,
            renderer,
            render_tx,
            handlers: vec![],
        };
        <T as LifeCycle>::mounted(
            container.sender.clone(),
            container.render_tx.clone(),
            &mut container.handlers,
        );
        container.init_messenger(receiver, container.sender.clone());
        container
    }

    pub fn init_messenger(&self, rx: MessageReceiver<T>, tx: MessageSender<T>) {
        let data_handle = self.data.clone();
        let render_tx_handle = self.render_tx.clone();
        let tx_handle = tx.clone();
        let fut = async move {
            rx.then(|(msg, inner_tx)| {
                let data = data_handle.clone();
                let tx = tx_handle.clone();
                let render_tx = render_tx_handle.clone();
                async move {
                    let mut data_inner = data.lock().await;
                    let should_render = msg.update(&mut *data_inner, tx, render_tx.clone());
                    let _ = inner_tx.send(());
                    (should_render, render_tx.clone())
                }
            })
            .filter_map(|(render, render_tx)| async move {
                if render {
                    Some(render_tx)
                } else {
                    None
                }
            })
            .for_each_concurrent(std::usize::MAX, |mut render_tx| async move {
                render_tx.send(()).await;
            })
            .await;
        };
        spawn_local(fut);
    }
}

impl<T> Container<T>
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

    pub fn mount_vdom<T: LifeCycle + 'static>(
        &mut self,
        data: T,
        block: &web_sys::HtmlElement,
        renderer: Render<T, T>,
    ) {
        let render_tx = self.render_tx.clone();
        let root_container = Container::new(data, renderer, render_tx.clone());
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

    pub fn init_app<
        T: LifeCycle + 'static,
        R: Renderer<Target = T, Data = T> + Default + 'static,
    >(
        id: Option<&str>,
    ) -> Self {
        let mut entry = Entry::new();
        let doc = web_sys::window()
            .map(|win| win.document())
            .flatten()
            .expect("unable to find document");

        let block = id
            .map(|id| {
                match doc
                    .get_element_by_id(id)
                    .map(|block| block.unchecked_into::<web_sys::HtmlElement>())
                {
                    Some(doc) => doc.unchecked_into(),
                    None => {
                        let body = doc.body().expect("unable to get body");
                        let new_block: web_sys::Node = doc
                            .create_element("div")
                            .map(|el| {
                                el.set_id(id);
                                el.unchecked_into()
                            })
                            .expect("unable to create a block with given id");

                        body.append_child(&new_block)
                            .expect("unable to append block");

                        new_block.unchecked_into()
                    }
                }
            })
            .unwrap_or(doc.body().unwrap().unchecked_into::<web_sys::HtmlElement>());

        let data = T::new(entry.render_tx.clone());
        entry.mount_vdom(data, &block, Box::new(R::default()));
        entry
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    pub struct Model {
        status: bool,
        embed: Option<Container<Model>>,
    }

    impl LifeCycle for Model {
        fn new(render_tx: Sender<()>) -> Self {
            Model {
                status: true,
                embed: None,
            }
        }

        fn mounted(
            mut sender: MessageSender<Self>,
            render_tx: Sender<()>,
            handlers: &mut Vec<EventListener>,
        ) {
            let handle1 = ClickEvents::Clicked.dispatch(&sender);
            let handle2 = ClickEvents::Clicked.dispatch(&sender);
            // let combined = join(handle1, handle2);

            // spawn_local(async {
            //     combined.await;
            // });
        }
    }

    pub enum ClickEvents {
        Clicked,
    }

    impl Messenger for ClickEvents {
        type Target = Model;

        fn update(
            &self,
            target: &mut Self::Target,
            sender: MessageSender<Self::Target>,
            render_tx: Sender<()>,
        ) -> bool {
            match self {
                ClickEvents::Clicked => {
                    log::info!("clicked");
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
            sender: MessageSender<Self::Data>,
        ) -> Node<'a> {
            use dodrio::builder::text;
            let bump = ctx.bump;
            let value = bf!(in bump, "value in box {}", &target.status).into_bump_str();
            dodrio!(bump,
                <div class="box">
                { vec![text(value)] }
                <div class="button"
                        onclick={
                            crate::messenger::consume(|e| {ClickEvents::Clicked}, &sender)
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
            sender: MessageSender<Self::Data>,
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
                        onclick={ consume(|e| { ClickEvents::Clicked }, &sender) }
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
            sender: MessageSender<Self::Data>,
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
        let data = Model::new(entry.render_tx.clone());

        entry.mount_vdom(data, &block, Box::new(MegaViewer {}));
    }

    use wasm_bindgen_test::*;

    #[wasm_bindgen_test]
    pub fn test_setup() {
        crate::tests::init_test();
        log::info!("start testing");
        setup();
    }
}

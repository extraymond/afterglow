use dodrio::Node;
use dodrio::RenderContext;
use dodrio::{Render as DodRender, Vdom};
use futures::{
    channel::mpsc, compat::Future01CompatExt, lock::Mutex, sink::SinkExt, stream::StreamExt,
};

use std::rc::Rc;
use wasm_bindgen_futures::futures_0_3::spawn_local;

/// Top level entity.
pub struct Entity<T, M, C> {
    /// send true to let vdom trigger re-render.
    pub root_tx: mpsc::UnboundedSender<bool>,
    /// contained data, may or may not be a entity.
    pub data: Rc<Mutex<T>>,
    /// send msg to trigger data mutation.
    pub data_tx: mpsc::UnboundedSender<M>,
    pub self_tx: mpsc::UnboundedSender<C>,
}

impl<T, M, C> Entity<T, M, C> {
    /// creata a  entity that contains the data, and allow root to listen to whether to re-render.
    pub fn new(data: T, root_tx: mpsc::UnboundedSender<bool>) -> Entity<T, M, C>
    where
        T: Component<Msg = M, RootMsg = C> + 'static,
        M: 'static,
        C: 'static,
    {
        let (data_tx, data_rx) = mpsc::unbounded::<M>();
        let (self_tx, self_rx) = mpsc::unbounded::<C>();
        let el = Entity {
            data: Rc::new(Mutex::new(data)),
            data_tx,
            root_tx,
            self_tx,
        };
        el.mount_self_rx(self_rx);
        el.mount_data_rx(data_rx);
        el
    }

    /// after attaching data to the entity, listen to msges emit by data.
    fn mount_data_rx(&self, mut data_rx: mpsc::UnboundedReceiver<M>)
    where
        T: Component<Msg = M> + 'static,
        M: 'static,
        C: 'static,
    {
        let mut root_tx = self.root_tx.clone();
        let data_handle = self.data.clone();

        let data_to_el = async move {
            while let Some(msg) = data_rx.next().await {
                let mut data = data_handle.lock().await;
                if data.update(msg) {
                    root_tx.send(true).await.unwrap();
                }
            }
        };
        spawn_local(data_to_el);
    }

    fn mount_self_rx(&self, mut self_rx: mpsc::UnboundedReceiver<C>)
    where
        T: Component<Msg = M, RootMsg = C> + 'static,
        M: 'static,
        C: 'static,
    {
        let mut root_tx = self.root_tx.clone();
        let data_handle = self.data.clone();

        let self_to_el = async move {
            while let Some(msg) = self_rx.next().await {
                let mut data = data_handle.lock().await;
                if data.update_el(msg) {
                    root_tx.send(true).await.unwrap();
                }
            }
        };
        spawn_local(self_to_el);
    }
}

/// Default impl for Entity.
impl<T, M, C> DodRender for Entity<T, M, C>
where
    T: Render<M, C>,
{
    fn render<'a>(&self, ctx: &mut RenderContext<'a>) -> Node<'a> {
        let data = self.data.try_lock().unwrap();
        data.render(ctx, self.data_tx.clone(), self.self_tx.clone())
    }
}

/// Component depends on associated msg to trigger mutation.
pub trait Component {
    type Msg;
    type RootMsg;

    /// handle data updates, if needs rerender, will send true to the root queue.
    fn update(&mut self, msg: Self::Msg) -> bool;
    fn update_el(&mut self, msg: Self::RootMsg) -> bool;
}

pub trait Render<M, C> {
    fn render<'a>(
        &self,
        ctx: &mut RenderContext<'a>,
        self_tx: mpsc::UnboundedSender<M>,
        root_tx: mpsc::UnboundedSender<C>,
    ) -> Node<'a>;
}

impl<T, M, C> Component for Entity<T, M, C>
where
    T: Component<Msg = M, RootMsg = C> + 'static,
    M: 'static,
    C: 'static,
{
    type Msg = M;
    type RootMsg = C;
    fn update(&mut self, msg: Self::Msg) -> bool {
        let data_handle = self.data.clone();
        let fut = async move {
            let mut data = data_handle.lock().await;
            data.update(msg);
        };
        spawn_local(fut);
        false
    }

    fn update_el(&mut self, msg: Self::RootMsg) -> bool {
        let data_handle = self.data.clone();
        let fut = async move {
            let mut data = data_handle.lock().await;
            data.update_el(msg);
        };
        spawn_local(fut);
        false
    }
}

// Contains the root vdom. Let entity trigger mutation by creating a pair queue.
#[derive(Default)]
pub struct MessageHub {
    /// sharable vdom, so we can have multiple listener that triggers re-render.
    vdom: Option<Rc<Mutex<Vdom>>>,
}

impl MessageHub {
    /// create vdom from the top level entity, and start listening for re-render  signals
    /// from root el.
    pub fn bind_root_el<T, M, C>(&mut self, data: T)
    where
        Entity<T, M, C>: DodRender,
        T: 'static + Component<Msg = M, RootMsg = C>,
        M: 'static,
        C: 'static,
    {
        let body = web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .body()
            .unwrap();
        let (root_tx, root_rx) = self.create_el_pair();
        let vdom = Vdom::new(&body, Entity::new(data, root_tx));
        self.bind_vdom(vdom);
        self.mount_el_rx(root_rx);
    }

    /// create a entity.
    pub fn create_el<T, M, C>(
        &mut self,
        data: T,
    ) -> (Entity<T, M, C>, mpsc::UnboundedReceiver<bool>)
    where
        T: Component<Msg = M, RootMsg = C> + 'static,
        M: 'static,
        C: 'static,
    {
        let (root_tx, root_rx) = self.create_el_pair();
        (Entity::new(data, root_tx), root_rx)
    }
    /// create the queue.
    pub fn create_el_pair(&self) -> (mpsc::UnboundedSender<bool>, mpsc::UnboundedReceiver<bool>) {
        mpsc::unbounded::<bool>()
    }

    /// bind vdom to the hub, so we can trigger re-render directly.
    pub fn bind_vdom(&mut self, vdom: Vdom) {
        self.vdom = Some(Rc::new(Mutex::new(vdom)));
    }

    /// listen for re-render signals from entity, only re-render if necessary.
    pub fn mount_el_rx(&mut self, mut root_rx: mpsc::UnboundedReceiver<bool>) {
        let vdom_handle = self.vdom.clone().unwrap();
        let el_to_root = async move {
            while let Some(msg) = root_rx.next().await {
                if msg {
                    let vdom = vdom_handle.lock().await;
                    vdom.weak().render().compat().await.unwrap();
                }
            }
        };
        spawn_local(el_to_root);
    }
}

use dodrio::{Render as DodRender, Vdom};
use futures::lock::Mutex;

use crate::prelude::*;
use std::rc::Rc;

/// Top level entity.
pub struct Entity<T: 'static, M: 'static, C: 'static>
where
    T: Component<M, C>,
{
    /// send true to let vdom trigger re-render.
    root_tx: mpsc::UnboundedSender<bool>,
    /// contained data, may or may not be a entity.
    pub data: Rc<Mutex<T>>,
    /// send msg to trigger data mutation.
    pub data_tx: mpsc::UnboundedSender<M>,
    pub self_tx: mpsc::UnboundedSender<C>,
}

impl<T: Component<M, C>, M, C> Drop for Entity<T, M, C> {
    fn drop(&mut self) {
        self.data_tx.disconnect();
        self.self_tx.disconnect();
        self.root_tx.disconnect();
    }
}

impl<T, M, C> Entity<T, M, C>
where
    T: Component<M, C>,
{
    /// creata a  entity that contains the data, and allow root to listen to whether to re-render.
    pub fn new(data: T, root_tx: mpsc::UnboundedSender<bool>) -> Entity<T, M, C> {
        let (data_tx, data_rx) = mpsc::unbounded::<M>();
        let (self_tx, self_rx) = mpsc::unbounded::<C>();
        let el = Entity {
            data: Rc::new(Mutex::new(data)),
            data_tx: data_tx.clone(),
            self_tx: self_tx.clone(),
            root_tx: root_tx.clone(),
        };
        el.mount_self_rx(self_rx);
        el.mount_data_rx(data_rx);
        T::mounted(data_tx, self_tx, root_tx);
        el
    }

    /// after attaching data to the entity, listen to msges emit by data.
    fn mount_data_rx(&self, mut data_rx: mpsc::UnboundedReceiver<M>)
    where
        T: Component<M, C>,
    {
        let mut root_tx = self.root_tx.clone();
        let data_handle = self.data.clone();

        let data_to_el = async move {
            while let Some(msg) = data_rx.next().await {
                let mut data = data_handle.lock().await;
                if data.update(msg) && root_tx.send(true).await.is_err() {
                    break;
                }
            }
            root_tx.disconnect();
        };
        spawn_local(data_to_el);
    }

    fn mount_self_rx(&self, mut self_rx: mpsc::UnboundedReceiver<C>)
    where
        T: Component<M, C>,
    {
        let mut root_tx = self.root_tx.clone();
        let data_handle = self.data.clone();

        let self_to_el = async move {
            while let Some(msg) = self_rx.next().await {
                let mut data = data_handle.lock().await;
                if data.update_el(msg) && root_tx.send(true).await.is_err() {
                    break;
                }
            }
            root_tx.disconnect();
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
        let bump = ctx.bump;
        self.data
            .try_lock()
            .map(|data| {
                data.render(
                    ctx,
                    self.data_tx.clone(),
                    self.self_tx.clone(),
                    self.root_tx.clone(),
                )
            })
            .unwrap_or(dodrio!(bump, <template></template>))
    }
}

// Component depends on associated msg to trigger mutation.
pub trait Component<M, C> {
    // initiate data
    fn new(root_tx: mpsc::UnboundedSender<bool>) -> Self
    where
        Self: Sized;

    // create task after component mounted
    fn mounted(_data_tx: Sender<M>, _self_tx: Sender<C>, _root_tx: Sender<bool>)
    where
        Self: Sized,
    {
    }

    /// handle data updates, if needs rerender, will send true to the root queue.
    fn update(&mut self, _: M) -> bool {
        false
    }
    /// handle entity updates, if needs rerender, will send true to the root queue.
    fn update_el(&mut self, _: C) -> bool {
        false
    }
}

pub trait Render<M, C>: Component<M, C> {
    fn render<'a>(
        &self,
        ctx: &mut RenderContext<'a>,
        data_tx: mpsc::UnboundedSender<M>,
        self_tx: mpsc::UnboundedSender<C>,
        root_tx: mpsc::UnboundedSender<bool>,
    ) -> Node<'a>;
}

impl<T, M, C> Component<M, C> for Entity<T, M, C>
where
    T: Component<M, C>,
{
    fn new(mut root_tx: mpsc::UnboundedSender<bool>) -> Entity<T, M, C> {
        let data = T::new(root_tx.clone());
        let (data_tx, data_rx) = mpsc::unbounded::<M>();
        let (self_tx, self_rx) = mpsc::unbounded::<C>();
        let el = Entity {
            data: Rc::new(Mutex::new(data)),
            data_tx,
            root_tx: root_tx.clone(),
            self_tx,
        };
        let fut = async move {
            root_tx.send(true).await.unwrap();
        };

        spawn_local(fut);
        el.mount_self_rx(self_rx);
        el.mount_data_rx(data_rx);
        el
    }

    fn update(&mut self, msg: M) -> bool {
        let data_handle = self.data.clone();
        let fut = async move {
            let mut data = data_handle.lock().await;
            data.update(msg);
        };
        spawn_local(fut);
        false
    }

    fn update_el(&mut self, msg: C) -> bool {
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
pub struct MessageHub {
    /// sharable vdom, so we can have multiple listener that triggers re-render.
    pub vdom: Option<Vdom>,
    pub hub_tx: mpsc::UnboundedSender<HubMsg>,
    hub_rx: Option<mpsc::UnboundedReceiver<HubMsg>>,
}

impl MessageHub {
    /// create vdom from the top level entity, and start listening for re-render  signals
    /// from root el.
    pub fn new() -> Self {
        let (hub_tx, hub_rx) = mpsc::unbounded::<HubMsg>();
        let hub_rx = Some(hub_rx);
        let vdom = None;
        Self {
            hub_rx,
            hub_tx,
            vdom,
        }
    }

    pub fn bind_root_el<T, M: 'static, C: 'static>(&mut self, data: T, block: Option<&str>)
    where
        Entity<T, M, C>: DodRender,
        T: Component<M, C> + 'static,
    {
        let window = web_sys::window().expect("unable to get window");
        let document = window.document().expect("unable to get document");
        let block = if let Some(tag) = block {
            let block: web_sys::HtmlElement =
                document.create_element("div").unwrap().dyn_into().unwrap();
            block.set_id(tag);
            document
                .body()
                .expect("unable to get body")
                .append_child(&block)
                .expect("unable to append");
            block
        } else {
            document.body().expect("unable to get body")
        };

        let (root_tx, root_rx) = self.create_el_pair();
        let vdom = Vdom::new(&block, Entity::new(data, root_tx));
        self.bind_vdom(vdom);
        self.mount_el_rx(root_rx);
    }

    pub fn gen_root_el<T, M: 'static, C: 'static>(&mut self, block: Option<&str>)
    where
        Entity<T, M, C>: DodRender,
        T: Component<M, C> + 'static,
    {
        let window = web_sys::window().expect("unable to get window");
        let document = window.document().expect("unable to get document");
        let block = if let Some(tag) = block {
            let block: web_sys::HtmlElement =
                document.create_element("div").unwrap().dyn_into().unwrap();
            block.set_id(tag);
            document
                .body()
                .expect("unable to get body")
                .append_child(&block)
                .expect("unable to append");
            block
        } else {
            document.body().expect("unable to get body")
        };

        let (root_tx, root_rx) = self.create_el_pair();
        let vdom = Vdom::new(&block, Entity::new(T::new(root_tx.clone()), root_tx));
        self.bind_vdom(vdom);
        self.mount_el_rx(root_rx);
    }

    /// create a entity.
    pub fn create_el<T, M: 'static, C: 'static>(
        &mut self,
        data: T,
    ) -> (Entity<T, M, C>, mpsc::UnboundedReceiver<bool>)
    where
        T: Component<M, C> + 'static,
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
        self.vdom = Some(vdom);
    }

    /// listen for re-render signals from entity, only re-render if necessary.
    pub fn mount_el_rx(&mut self, mut root_rx: mpsc::UnboundedReceiver<bool>) {
        let mut hub_tx = self.hub_tx.clone();
        let el_to_root = async move {
            while let Some(msg) = root_rx.next().await {
                if msg {
                    if hub_tx.send(HubMsg::Render).await.is_err() {
                        break;
                    }
                }
            }
            hub_tx.disconnect();
        };
        spawn_local(el_to_root);
    }

    pub fn mount_hub_rx(&mut self) {
        let vdom = self.vdom.take().expect("unable to take vdom.");
        let mut hub_rx = self.hub_rx.take().expect("unable to take hub");
        let root_to_hub = async move {
            while let Some(msg) = hub_rx.next().await {
                match msg {
                    HubMsg::Render => {
                        let _ = JsFuture::from(js_sys::Promise::resolve(&JsValue::null())).await;
                        vdom.weak().render().await.expect("unable to rerender");
                    }
                    HubMsg::Drop => {
                        hub_rx.close();
                        break;
                    }
                }
            }
            drop(hub_rx);
            drop(vdom);
        };
        spawn_local(root_to_hub);
    }
}

pub enum HubMsg {
    Render,
    Drop,
}

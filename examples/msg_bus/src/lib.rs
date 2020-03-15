use afterglow::prelude::*;

pub struct Model {
    pub local: Container<Sibling>,
    pub remote: Container<Sibling>,
}

impl LifeCycle for Model {
    fn new(render_tx: Sender<()>) -> Self {
        let bus: BusService<BusMsg> = BusService::new();
        let mut child = Sibling::new(render_tx.clone());
        child.name = "local".to_string();
        child.bus.replace(bus.clone());
        let local = Container::new(child, Box::new(ChildView), render_tx.clone());
        let mut child = Sibling::new(render_tx.clone());
        child.name = "remote".to_string();
        child.bus.replace(bus.clone());
        let remote = Container::new(child, Box::new(ChildView), render_tx.clone());
        Model { local, remote }
    }
}

#[derive(Default)]
pub struct Sibling {
    pub name: String,
    pub value: usize,
    pub bus: Option<BusService<BusMsg>>,
}

impl LifeCycle for Sibling {
    fn new(render_tx: Sender<()>) -> Self {
        Sibling::default()
    }

    fn mounted(
        sender: MessageSender<Self>,
        render_tx: Sender<()>,
        handlers: &mut Vec<EventListener>,
    ) {
        ChildMsg::InitBus.dispatch(&sender);
    }
}

pub enum ChildMsg {
    InitBus,
    Notified(String),
    ValueUpdated,
}

impl Messenger for ChildMsg {
    type Target = Sibling;

    fn update(
        &self,
        target: &mut Self::Target,
        sender: MessageSender<Self::Target>,
        render_tx: Sender<()>,
    ) -> bool {
        match self {
            ChildMsg::InitBus => {
                target.bus.as_ref().map(|bus| {
                    bus.register(sender);
                });
            }
            ChildMsg::ValueUpdated => {
                target.value += 1;
                target.bus.as_ref().map(|bus| {
                    bus.publish(BusMsg::ValueUpdated(target.name.clone()));
                });
                return true;
            }
            ChildMsg::Notified(name) => {
                if target.name != *name {
                    target.value +=1;
                    return true
                }
            }
            _ => {}
        }
        false
    }
}

#[derive(Clone)]
pub enum BusMsg {
    ValueUpdated(String),
}

impl Into<Option<Message<Sibling>>> for BusMsg {
    fn into(self) -> Option<Message<Sibling>> {
        match self {
            BusMsg::ValueUpdated(name) => Some(Box::new(ChildMsg::Notified(name))),
        }
    }
}

pub struct ChildView;
impl Renderer for ChildView {
    type Target = Sibling;
    type Data = Sibling;

    fn view<'a>(
        &self,
        target: &Self::Target,
        ctx: &mut RenderContext<'a>,
        sender: MessageSender<Self::Data>,
    ) -> Node<'a> {
        let bump = ctx.bump;
        let button = if target.name == "local" {
            let btn  =dodrio!(bump, 
                <div class="button"
                onclick={ consume(move |_| { ChildMsg::ValueUpdated }, &sender)}
                >"add up"</div>);
            Some(btn)
        } else {
            None
        };
        let value = bf!(in bump, "value is: {}", target.value).into_bump_str();
        dodrio!(bump,
                <div class="box">
                { button }
                <p>{ text(value) }</p>
                </div>
        )
    }
}

pub struct View;
impl Renderer for View {
    type Target = Model;
    type Data = Model;

    fn view<'a>(&self, target: &Self::Target, ctx: &mut RenderContext<'a>, sender: MessageSender<Self::Data>) -> Node<'a> {
        let bump = ctx.bump;

        dodrio!(bump, 
            <div class="columns">
                <div class="column">{ target.local.render(ctx) }</div>
                <div class="column">{ target.remote.render(ctx) }</div>
            </div>
        )
    }
}

#[derive(Default)]
pub struct HeroView;
impl Renderer for HeroView {
    type Target = Model;
    type Data = Model;

    fn view<'a>(
        &self,
        target: &Self::Target,
        ctx: &mut RenderContext<'a>,
        sender: MessageSender<Self::Data>,
    ) -> Node<'a> {
        let bump = ctx.bump;

        dodrio!(bump,
            <div class="hero">
                <div class="hero-body">
                    <div class="container">{ View.view(target, ctx, sender) }</div>
                </div>
            </div>
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;
    wasm_bindgen_test_configure!(run_in_browser);

    fn preload_css() {
        let doc = web_sys::window().unwrap().document().unwrap();
        let link = doc.create_element("link").unwrap();
        link.set_attribute("rel", "stylesheet");
        link.set_attribute("type", "text/css");
        link.set_attribute(
            "href",
            "https://cdnjs.cloudflare.com/ajax/libs/bulma/0.7.5/css/bulma.css",
        );
        doc.head().map(|head| {
            head.append_child(&link.unchecked_into::<web_sys::Node>());
        });
    }

    #[wasm_bindgen_test]
    fn init() {
        preload_css();
        Entry::init_app::<Model, HeroView>(None);
    }
}
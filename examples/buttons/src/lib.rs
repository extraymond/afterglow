use afterglow::prelude::*;

pub struct Model {
    pub value: i32,
}

impl LifeCycle for Model {
    fn new(render_tx: Sender<((), oneshot::Sender<()>)>) -> Self {
        Model { value: 0 }
    }
}

pub enum Msg {
    Add,
    Sub,
}

impl Messenger for Msg {
    type Target = Model;

    fn update(
        &self,
        target: &mut Self::Target,
        sender: MessageSender<Self::Target>,
        render_tx: Sender<((), oneshot::Sender<()>)>,
    ) -> bool {
        match self {
            Msg::Add => {
                target.value += 1;
                true
            }
            Msg::Sub => {
                target.value -= 1;
                true
            }
        }
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
        let value = bf!(in bump, "{}", target.value).into_bump_str();
        dodrio!(bump,
            <div class="box" style="width: 400px">
                <p class="label">{ text(value) }</p>
                <div class="field is-grouped">
                    <div class="control">
                        <div class="button" onclick={consume(|_| { Msg::Add }, &sender)}>"click me"</div>
                        </div>
                    <div class="control">
                        <div class="button" onclick={consume(|_| { Msg::Sub }, &sender)}>"click me"</div>
                    </div>
                </div>
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
        sender: &MessageSender<Self::Data>,
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

impl Model {
    pub fn init() {
        Entry::init_app::<Self, HeroView>(None);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use wasm_bindgen_test::*;
    wasm_bindgen_test_configure!(run_in_browser);

    fn preload_css() -> Result<(), JsValue> {
        let doc = web_sys::window().unwrap().document().unwrap();
        let link = doc.create_element("link").unwrap();
        link.set_attribute("rel", "stylesheet")?;
        link.set_attribute("type", "text/css")?;
        link.set_attribute(
            "href",
            "https://cdnjs.cloudflare.com/ajax/libs/bulma/0.7.5/css/bulma.css",
        )?;
        doc.head().map(|head| {
            let _ = head.append_child(&link.unchecked_into::<web_sys::Node>());
        });

        Ok(())
    }

    #[wasm_bindgen_test]
    fn init() {
        preload_css();
        Model::init();
    }
}

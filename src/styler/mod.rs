use dodrio::{Node, RenderContext};
use typed_html::types::ClassList;

pub trait StateRender: PartialEq<bool> {
    fn active<'a>(&self, ctx: &mut RenderContext<'a>) -> Node<'a>;
    fn inactive<'a>(&self, ctx: &mut RenderContext<'a>) -> Node<'a>;
    fn state_render<'a>(&self, ctx: &mut RenderContext<'a>) -> Node<'a> {
        if *self == true {
            self.active(ctx)
        } else {
            self.inactive(ctx)
        }
    }

    fn active_class<'a>(&self, ctx: &mut RenderContext<'a>) -> ClassList;
    fn inactive_class<'a>(&self, ctx: &mut RenderContext<'a>) -> ClassList;
    fn state_class<'a>(&self, ctx: &mut RenderContext<'a>) -> ClassList {
        if *self == true {
            self.active_class(ctx)
        } else {
            self.inactive_class(ctx)
        }
    }

    fn active_style<'a>(&self, ctx: &mut RenderContext<'a>) -> &'a str;
    fn inactive_style<'a>(&self, ctx: &mut RenderContext<'a>) -> &'a str;
    fn state_style<'a>(&self, ctx: &mut RenderContext<'a>) -> &'a str {
        if *self == true {
            self.active_style(ctx)
        } else {
            self.inactive_style(ctx)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::*;

    use wasm_bindgen_test::*;

    struct Dummy {
        status: bool,
    }

    impl Default for Dummy {
        fn default() -> Self {
            Dummy { status: true }
        }
    }

    impl PartialEq<bool> for Dummy {
        fn eq(&self, rhs: &bool) -> bool {
            &self.status == rhs
        }
    }

    impl StateRender for Dummy {
        fn active_style<'a>(&self, ctx: &mut RenderContext<'a>) -> &'a str {
            let bump = ctx.bump;
            dodrio::bumpalo::format!(in bump, "{}", self.status).into_bump_str()
        }

        fn inactive_style<'a>(&self, ctx: &mut RenderContext<'a>) -> &'a str {
            let bump = ctx.bump;
            dodrio::bumpalo::format!(in bump, "{}", self.status).into_bump_str()
        }

        fn active_class<'a>(&self, ctx: &mut RenderContext<'a>) -> ClassList {
            let mut class = ClassList::new();
            class.add("button");
            class.add("is-primary");
            class
        }

        fn inactive_class<'a>(&self, ctx: &mut RenderContext<'a>) -> ClassList {
            let mut class = ClassList::new();
            class.add("button");
            class.add("is-danger");
            class
        }
        fn active<'a>(&self, ctx: &mut RenderContext<'a>) -> Node<'a> {
            let bump = ctx.bump;
            dodrio!(bump, <div style={ self.active_style(ctx) }></div>)
        }

        fn inactive<'a>(&self, ctx: &mut RenderContext<'a>) -> Node<'a> {
            let bump = ctx.bump;

            dodrio!(bump,
                <div class={ self.state_class(ctx) }>
                <video autoplay={
                    *self == true
                }></video>
                </div>)
        }
    }

    impl Component<(), ()> for Dummy {
        fn new(_: Sender<bool>) -> Self {
            Dummy::default()
        }

        fn update(&mut self, _: ()) -> bool {
            self.status = !self.status;
            true
        }
    }

    impl Render<(), ()> for Dummy {
        fn render<'a>(
            &self,
            ctx: &mut RenderContext<'a>,
            self_tx: Sender<()>,
            _: Sender<()>,
            _: Sender<bool>,
        ) -> Node<'a> {
            let bump = ctx.bump;
            dodrio!(bump,
                <div class="box">
                <div
                class={self.state_class(ctx)}
                style={self.state_style(ctx)}
                onclick={
                    move|_, _, _| {
                        let mut tx = self_tx.clone();
                        let fut = async move {
                            tx.send(()).await.unwrap();
                        };
                        spawn_local(fut);
                    }
                }
                >
                "hello world"

                </div>
                <link rel=typed_html::types::LinkType::StyleSheet href="https://cdnjs.cloudflare.com/ajax/libs/bulma/0.7.5/css/bulma.css"/>
                </div>
            )
        }
    }

    fn init_dummy() -> Result<(), JsValue> {
        let win = web_sys::window().ok_or(JsValue::UNDEFINED)?;
        let doc = win.document().ok_or(JsValue::UNDEFINED)?;
        let newbody: web_sys::HtmlElement = doc.create_element("body").unwrap().dyn_into()?;
        doc.set_body(Some(&newbody));
        let mut hub = MessageHub::new();
        let tag = "dummy";
        hub.bind_root_el(Dummy::default(), Some(tag));
        hub.mount_hub_rx();
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_dummy() {
        crate::tests::init_test();
        assert_eq!(true, init_dummy().is_ok());
    }

    #[wasm_bindgen_test]
    fn test_funny() {
        assert_eq!(1, 2);
    }
}

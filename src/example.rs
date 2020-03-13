use crate::prelude::*;

#[derive(Default)]
pub struct Model {
    status: bool,
    clicked: i32,
}

impl LifeCycle for Model {
    fn new(render_tx: Sender<()>) -> Self {
        Model::default()
    }
}

struct TableView;
impl Renderer for TableView {
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
            <table class="table">
                <thead>
                    <tr>
                        <th>"counts"</th>
                        <th>"status"</th>
                    </tr>
                </thead>
                <tbody>
                    <tr>
                        <td>{ text(bf!(in bump, "clicked: {} times", target.clicked).into_bump_str())}</td>
                        <td>{ text(bf!(in bump, "status: {}", target.status).into_bump_str())}</td>
                    </tr>
                </tbody>
            </table>
        )
    }
}

pub enum ClickMsg {
    Clicked,
}
impl Messenger for ClickMsg {
    type Target = Model;

    fn update(
        &self,
        target: &mut Self::Target,
        sender: MessageSender<Self::Target>,
        render_tx: Sender<()>,
    ) -> bool {
        match self {
            ClickMsg::Clicked => {
                target.clicked += 1;
                target.status = !target.status;
                true
            }
        }
    }
}

struct ButtonView;
impl Renderer for ButtonView {
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
            <div
            onclick={ consume(|e: web_sys::Event| { ClickMsg::Clicked }, &sender)}
            class="button">"clicked to increase stats"</div>
        )
    }
}

#[derive(Default)]
struct MainView;
impl Renderer for MainView {
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
            <div>
                <div class="hero is-light">
                    <div class="hero-body">
                        <div class="container">
                            <div class="card">
                                <div class="card-content">
                                { TableView.view(target, ctx, sender.clone())}
                                </div>
                                <div class="card-footer">
                                    <div class="card-footer-item">
                                    { ButtonView.view(target, ctx, sender)}
                                    </div>
                                </div>
                            </div>
                        </div>
                    </div>
                </div>
                <link rel="stylesheet"
                href="https://cdnjs.cloudflare.com/ajax/libs/bulma/0.7.5/css/bulma.css"/>
            </div>
        )
    }
}

pub fn init_example() {
    Entry::init_app::<Model, MainView>("app");
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    #[wasm_bindgen_test]
    fn test_example() {
        init_example();
    }
}

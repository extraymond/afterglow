use afterglow::prelude::*;

pub struct Model {
    pub rows: Vec<Item>,
}

pub struct Item {
    count: i32,
}

impl LifeCycle for Model {
    fn new(render_tx: Sender<()>) -> Self {
        let rows = (0..10)
            .map(|idx| Item { count: idx as i32 })
            .collect::<Vec<_>>();
        Model { rows }
    }
}

pub enum Msg {
    RowAdded,
    RowRemoved(usize),
    RowUpdated(usize),
}

impl Messenger for Msg {
    type Target = Model;

    fn update(
        &self,
        target: &mut Self::Target,
        sender: MessageSender<Self::Target>,
        render_tx: Sender<()>,
    ) -> bool {
        match self {
            Msg::RowAdded => {
                target.rows.push(Item { count: 0 });
                true
            }
            Msg::RowRemoved(idx) => {
                target.rows.remove(*idx);
                true
            }
            Msg::RowUpdated(idx) => target
                .rows
                .get_mut(*idx)
                .map(|item| {
                    item.count += 1;
                    true
                })
                .unwrap_or_default(),
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
        sender: MessageSender<Self::Data>,
    ) -> Node<'a> {
        let bump = ctx.bump;
        let row_viewer = RowView::default();
        let rows = target
            .rows
            .iter()
            .enumerate()
            .map(|(idx, item)| row_viewer.view(&(idx, item), ctx, sender.clone()));
        dodrio!(bump,
            <div class="box">
                <table class="table is-fullwidth">
                    <thead>
                        <tr>
                            <th>"idx"</th>
                            <th>"add count"</th>
                            <th>"remove row"</th>
                        </tr>
                    </thead>
                    <tbody>
                    { rows }
                    </tbody>
                </table>
            </div>
        )
    }
}

use std::marker::PhantomData;

#[derive(Default)]
pub struct RowView<'x> {
    lt: PhantomData<&'x ()>,
}

impl<'x> Renderer for RowView<'x> {
    type Target = (usize, &'x Item);
    type Data = Model;

    fn view<'a>(
        &self,
        target: &Self::Target,
        ctx: &mut RenderContext<'a>,
        sender: MessageSender<Self::Data>,
    ) -> Node<'a> {
        let bump = ctx.bump;
        let row_idx = target.0.clone();
        let counts =
            bf!(in bump, "no.{} clicked {} times", &row_idx, target.1.count).into_bump_str();

        dodrio!(bump,
            <tr>
                <td>
                { text(counts) }
                </td>
                <td>
                    <div
                    onclick={consume(move |_| {Msg::RowUpdated(row_idx)}, &sender) }
                    class="button">"add count"</div>
                </td>
                <td>
                    <div
                    onclick={consume(move|_| {Msg::RowRemoved(row_idx)}, &sender) }
                    class="button">"remove row"</div>
                </td>
            </tr>
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

use afterglow::prelude::*;

#[derive(Default)]
pub struct Grid {
    pub blocks: [[bool; 4]; 4],
}
impl LifeCycle for Grid {
    fn new(render_tx: Sender<((), oneshot::Sender<()>)>) -> Self {
        Grid::default()
    }
}

pub enum GridMsg {
    Clicked([usize; 2]),
}

impl Messenger for GridMsg {
    type Target = Grid;

    fn update(
        &self,
        target: &mut Self::Target,
        sender: &MessageSender<Self::Target>,
        render_tx: &Sender<((), oneshot::Sender<()>)>,
    ) -> bool {
        match self {
            GridMsg::Clicked([x, y]) => {
                target.blocks[*x][*y] = !target.blocks[*x][*y];
                return true;
            }
        }
        false
    }
}

pub struct GridView;
impl Renderer for GridView {
    type Target = (bool, usize, usize);
    type Data = Grid;

    fn view<'a>(
        &self,
        target: &Self::Target,
        ctx: &mut RenderContext<'a>,
        sender: &MessageSender<Self::Data>,
    ) -> Node<'a> {
        let bump = ctx.bump;
        let x = target.1;
        let y = target.2;
        let color = if target.0 {
            "background: purple; border-style: solid"
        } else {
            "background: white; border-style: solid"
        };

        dodrio!(bump, <div style={ color } onclick={ consume(move |_| { GridMsg::Clicked([x, y])}, &sender) }></div>)
    }
}

#[derive(Default)]
pub struct Board;
impl Renderer for Board {
    type Target = Grid;
    type Data = Grid;

    fn view<'a>(
        &self,
        target: &Self::Target,
        ctx: &mut RenderContext<'a>,
        sender: &MessageSender<Self::Data>,
    ) -> Node<'a> {
        let bump = ctx.bump;

        let css = r"
        height: 100vh;
        width: 100vw;
        display: grid;
        grid-template-rows: repeat(4, 1fr);
        grid-template-columns: repeat(4, 1fr);
        ";

        let mut views = vec![];
        for x in 0..4 {
            for y in 0..4 {
                let val = target.blocks[x][y];
                views.push(GridView.view(&(val, x, y), ctx, &sender));
            }
        }

        dodrio!(bump, 
            <div style={ css }>
            { views }
            </div>)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use wasm_bindgen_test::*;
    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_init() {
        Entry::init_app::<Grid, Board>(None);
    }
}

use crate::prelude::*;
use async_trait::*;

pub(crate) type Render<T, D> = Box<dyn Renderer<Target = T, Data = D>>;

pub trait Renderer {
    type Target;
    type Data;

    fn view<'a>(
        &self,
        target: &Self::Target,
        ctx: &mut RenderContext<'a>,
        sender: &MessageSender<Self::Data>,
    ) -> Node<'a>;
}

impl<'a, T> dodrio::Render<'a> for Container<T>
where
    T: LifeCycle,
{
    fn render(&self, cx: &mut RenderContext<'a>) -> Node<'a> {
        let bump = cx.bump;
        if let Some(data) = self.data.try_lock() {
            self.renderer.view(&*data, cx, &self.sender)
        } else {
            dodrio::builder::template(bump).finish()
        }
    }
}

#[async_trait]
pub trait AsyncRenderer {
    type Target;
    type Data;

    async fn view<'a>(
        &self,
        target: &Self::Target,
        ctx: &mut RenderContext<'a>,
        sender: &MessageSender<Self::Data>,
    ) -> Node<'a>;
}

#[cfg(test)]
mod tests {
    use super::*;

    pub enum Device {
        pc,
        mobile,
    }

    pub struct Data {
        state: i32,
    }

    impl Renderer for Device {
        type Target = Data;
        type Data = Data;

        fn view<'a>(
            &self,
            target: &Self::Target,
            ctx: &mut RenderContext<'a>,
            sender: &MessageSender<Self::Data>,
        ) -> Node<'a> {
            let bump = ctx.bump;
            let state = bf!(in bump, "{}", &target.state).into_bump_str();

            match self {
                Device::pc => dodrio::builder::div(bump)
                    .attr("class", state)
                    .child(text("I'm on pc"))
                    .finish(),
                Device::mobile => dodrio::builder::div(bump)
                    .attr("class", state)
                    .child(text("I'm on mobile"))
                    .finish(),
            }
        }
    }
}

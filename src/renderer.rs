use crate::container;
use crate::prelude::*;

pub trait Renderer {
    type Target;

    fn view<'a>(
        &self,
        target: &Self::Target,
        ctx: &mut RenderContext<'a>,
        sender: Sender<Box<dyn crate::messenger::Messenger<Target = Self::Target>>>,
    ) -> Node<'a>;
}

impl<'a, T> dodrio::Render<'a> for container::Container<T> {
    fn render(&self, cx: &mut RenderContext<'a>) -> Node<'a> {
        let bump = cx.bump;
        if let Some(data) = self.data.try_lock() {
            self.renderer.view(&*data, cx, self.sender.clone())
        } else {
            dodrio!(bump, <template></template>)
        }
    }
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

        fn view<'a>(
            &self,
            target: &Self::Target,
            ctx: &mut RenderContext<'a>,
            sender: Sender<Box<dyn crate::messenger::Messenger<Target = Self::Target>>>,
        ) -> Node<'a> {
            let bump = ctx.bump;
            let state = bf!(in bump, "{}", &target.state).into_bump_str();

            match self {
                Device::pc => dodrio!(bump, <div class={state}>"I'm on pc"</div>),
                Device::mobile => dodrio!(bump, <div class={state}>"I'm on mobile"</div>),
            }
        }
    }
}

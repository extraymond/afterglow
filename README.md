# Afterglow

Afterglow is a experimental rust frontend framework built upon [dodrio](https://github.com/fitzgen/dodrio) and [typed-html](https://github.com/bodil/typed-html/blob/master/typed-html/Cargo.toml)

## Features

1. Virtual-Dom: 
    
    Use vdom provided by dodrio to handle rerender.
2. JSX-like syntax:

    Using jsx-like macro provided by typed-html to create views.
3. Elm-inspired Container:

    Each data Container has its own lifecycle, will only trigger rerender if choose to.
3. Using trait objects to encourage freedom on UI exploration: 

    While having a strict data model and providing views tight to a strict set of views/mutations is great for maintaining a clean overview of component, often times building frontends will more likely requires a more adaptive workflow. Which means that for the same data model, there might be multiple use cases across your project, and each cases will require a exclusive sets of data mutations. Using trait objects on Renderer and Messenger allow the Container to have lesser constraint on the when and how it's gonna be rendered and mutated.
4. Relys on message bus to share events across heterogenous Containers:

    By registering the container's message sender to a centralized bus, containers can design their own response to that bus event. So parent-child, sibling communication can be achieved.

## Example

Considering you would like to reuse the same data as a visual element,

```rust
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
```

while maintaining a single container which handels data mutation and business logic, we can create multiple sets of views that has it's own UI logic encapsulated. 


```rust
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
```

And without having to define a giant enum to let the container handle all of the incoming events, we can have sets of Messenger that only contians variants that's related to each other. You can define new ones later on without changing the implementation of the Model

```rust

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

```

With view providers, aka Renderers, decoupled from the data model, we can compose a page with more freedom. User will be able to separate UI logic within it's own visual element.

```rust
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

```

So by definition, an app is a given data model, that has a default Renderer.

```rust
pub fn init_example() {
    Entry::init_app::<Model, MainView>("app");
}
```

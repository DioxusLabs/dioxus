use dioxus_core::debug_renderer::DebugRenderer;
use dioxus_core::{component::Properties, prelude::*};

fn main() -> Result<(), ()> {
    let p1 = SomeProps { name: "bob".into() };

    let _vdom = DebugRenderer::new_with_props(Example, p1);

    Ok(())
}

#[derive(Debug, PartialEq, Props)]
struct SomeProps {
    name: String,
}

static Example: FC<SomeProps> = |ctx, _props| {
    ctx.render(html! {
        <div>
            <h1> "hello world!" </h1>
        </div>
    })
};

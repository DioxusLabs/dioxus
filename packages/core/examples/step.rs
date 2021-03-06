use dioxus_core::prelude::*;

fn main() -> Result<(), ()> {
    let p1 = Props { name: "bob".into() };

    let mut vdom = VirtualDom::new_with_props(Example, p1);
    vdom.update_props(|p: &mut Props| {});

    Ok(())
}

#[derive(Debug, PartialEq)]
struct Props {
    name: String,
}

static Example: FC<Props> = |ctx, _props| {
    ctx.render(html! {
        <div>
            <h1> "hello world!" </h1>
            <h1> "hello world!" </h1>
            <h1> "hello world!" </h1>
            <h1> "hello world!" </h1>
        </div>
    })
};

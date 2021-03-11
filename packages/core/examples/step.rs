use dioxus_core::{component::Properties, prelude::*};

fn main() -> Result<(), ()> {
    let p1 = SomeProps { name: "bob".into() };

    let _vdom = VirtualDom::new_with_props(Example, p1);

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
            <h1> "hello world!" </h1>
            <h1> "hello world!" </h1>
            <h1> "hello world!" </h1>
        </div>
    })
};

// toodo: derive this
impl Properties for SomeProps {
    type Builder = SomePropsBuilder<((),)>;
    type StaticOutput = SomeProps;
    fn builder() -> Self::Builder {
        SomeProps::builder()
    }

    unsafe fn into_static(self) -> Self {
        self
    }
}

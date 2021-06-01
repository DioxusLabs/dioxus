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

static Example: FC<SomeProps> = |ctx| {
    ctx.render(html! {
        <div>
            <h1> "hello world!" </h1>
        </div>
    })
};

// #[test]
#[derive(PartialEq, Clone)]
struct MyStruct {
    a: String,
}

fn check_before_to_owned() {
    let new_str = MyStruct {
        a: "asd".to_string(),
    };

    let out = town(&new_str);
}

fn town<T: ToOwned + PartialEq>(t: &T) -> T::Owned {
    t.to_owned()
}

//! An example that shows how to:
//!     create a scope,
//!     render a component,
//!     change some data
//!     render it again
//!     consume the diffs and write that to a renderer

use dioxus_core::{prelude::*, scope::Scope};

fn main() -> Result<(), ()> {
    let p1 = Props { name: "bob".into() };

    let mut vdom = VirtualDom::new_with_props(Example, p1);
    // vdom.progress()?;

    Ok(())
}

struct Props {
    name: String,
}
impl Properties for Props {
    fn call(&self, ptr: *const ()) {}

    // fn new() -> Self {
    //     todo!()
    // }
}

static Example: FC<Props> = |ctx, props| {
    ctx.view(html! {
        <div>
            <h1> "hello world!" </h1>
            <h1> "hello world!" </h1>
            <h1> "hello world!" </h1>
            <h1> "hello world!" </h1>
        </div>
    })
};

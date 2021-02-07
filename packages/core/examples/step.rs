//! An example that shows how to:
//!     create a scope,
//!     render a component,
//!     change some data
//!     render it again
//!     consume the diffs and write that to a renderer

use dioxus_core::{
    prelude::*,
    virtual_dom::{Properties, Scope},
};

fn main() {
    let mut scope = Scope::new(Example);
    let ctx = scope.create_context::<Props>();
    let p1 = Props { name: "bob".into() };

    let p2 = Props { name: "bob".into() };
}

struct Props {
    name: String,
}
impl Properties for Props {
    fn new() -> Self {
        todo!()
    }
}

static Example: FC<Props> = |ctx| {
    ctx.view(html! {
        <div>
            <h1> </h1>
        </div>
    })
};

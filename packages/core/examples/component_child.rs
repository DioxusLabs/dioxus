use std::{ops::Deref, rc::Rc};

use dioxus::virtual_dom::Scope;
use dioxus_core::prelude::*;

type RcStr = Rc<str>;

fn main() {
    let r: RcStr = "asdasd".into();
    let r: RcStr = String::from("asdasd").into();

    let g = rsx! {
        div {
            Example {}
        }
    };
}

static Example: FC<()> = |ctx| {
    let nodes = ctx.children();

    //
    rsx! { in ctx,
        div {
            {nodes}
        }
    }
};

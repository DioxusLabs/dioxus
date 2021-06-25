//! POC: Planning the layout of a single element type
//!
//!
//! The ultimate goal with a dedicated namespace is three-fold:
//! - compile-time correct templates preventing misuse of elemnents
//! - deep integration of DSL with IDE
//!
//!
//!

struct NodeCtx {}

struct div<'a>(&NodeCtx);
impl<'a> div<'a> {
    fn new(cx: &NodeCtx) -> Self {
        div(cx)
    }
}

fn main() {}

fn factory(
    // this is your mom
    cx: &NodeCtx,
) {
    div::new(cx);
    rsx! {
        div {}
    }
}

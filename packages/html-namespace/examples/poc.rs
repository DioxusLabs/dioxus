//! POC: Planning the layout of a single element type
//!
//!
//! The ultimate goal with a dedicated namespace is three-fold:
//! - compile-time correct templates preventing misuse of elemnents
//! - deep integration of DSL with IDE
//!
//!
//!

struct NodeFactory {}

struct div<'a>(&NodeFactory);
impl<'a> div<'a> {
    fn new(cx: &NodeFactory) -> Self {
        div(cx)
    }
}

fn main() {}

fn factory(
    // this is your mom
    cx: &NodeFactory,
) {
    div::new(cx);
    rsx! {
        div {}
    }
}

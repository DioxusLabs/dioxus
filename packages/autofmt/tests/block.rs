use std::collections::VecDeque;

use dioxus_rsx::{BodyNode, CallBody};

const LINE_WIDTH: usize = 80;

pub struct Printer {
    buf: String,
    // queue: Vec<Item>,
}

pub enum Break {
    // Space flexes to line if need be
    Space,

    // Line always forces a new line
    // Comments are an example of this
    Line,
}

// enum Item {
//     BreakBegin,
//     BreakEnd,
//     Text(Cow<'static, str>),
// }

impl Printer {
    fn doit(&mut self, body: CallBody) {
        for node in body.roots {}
    }
    fn node(&mut self, node: BodyNode) {}
}

#[test]
fn it_works() {
    let src = r#"div {}"#;
    let contents: CallBody = syn::parse_str(src).unwrap();
}

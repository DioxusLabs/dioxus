fn main() {}

use dioxus_core as dioxus;
use dioxus_core::prelude::*;
mod dioxus_elements {
    use super::*;
    pub struct div;
    impl DioxusElement for div {
        const TAG_NAME: &'static str = "str";
        const NAME_SPACE: Option<&'static str> = None;
    }
}

static Example: FC<()> = |cx| {
    let list = (0..10).map(|f| {
        //
        LazyNodes::new(move |f: NodeFactory| todo!())
    });
    cx.render(LazyNodes::new(move |cx| {
        let bump = cx.bump();
        dioxus_core::builder::ElementBuilder::new(&cx, "h1")
            .children([
                cx.text(format_args!("hello")),
                cx.text(format_args!("hello")),
                cx.fragment_from_iter(list),
            ])
            .finish()
    }))
};

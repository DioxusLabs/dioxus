use super::{global_attributes::HtmlElement, NodeBuilder};
use dioxus_core::prelude::*;

macro_rules! element {
    ($( $(#[$attr:meta])* $suct:ident => $ef:ident;)*) => {
        $(
            pub struct $suct<'a>(NodeBuilder<'a>);

            // apply meta
            $(#[$attr])*
            pub fn $ef(_cx: &ScopeState) -> $suct {
                todo!(stringify!($ef))
            }

            impl<'a> HtmlElement<'a> for $suct<'a> {
                #[inline]
                fn inner_mut(&mut self) -> &mut NodeBuilder<'a> {
                    &mut self.0
                }
                #[inline]
                fn inner(self) -> NodeBuilder<'a> {
                    self.0
                }
            }
        )*
    };
}

element!(
    /// A div
    Div => div;

    // A base element
    Base => base;
);

#[test]
fn div_works() {
    fn template(cx: &ScopeState) {
        let r = div(cx)
            .class("asd")
            .content_editable(true)
            .id("ad")
            .tab_index(10)
            .class(format!("asd"))
            .build();
    }
}

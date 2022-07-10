use dioxus_core::exports::bumpalo::collections::Vec as BumpVec;
use dioxus_core::{prelude::*, Attribute};
use std::{any::Any, ops::Deref, ptr::addr_of};

use super::global_attributes::HtmlElement;
use super::NodeBuilder;

trait Buildable<'a, P> {
    type Builder;
}

struct Builder;
impl<'a, F, H> Buildable<'a, Builder> for F
where
    H: HtmlElement<'a>,
    F: Fn(&'a ScopeState) -> H,
{
    type Builder = H;
}

struct Template {}
impl<'a, F> Buildable<'a, Template> for F
where
    F: Fn(&'a ScopeState) -> Element<'a>,
{
    type Builder = Element<'a>;
}

impl<'a, F, P> Buildable<'a, P> for F
where
    F: Fn(Scope<P>) -> Element,
    P: Properties,
{
    type Builder = P::Builder;
}

fn take<'a, P, B>(g: B) -> B::Builder
where
    B: Buildable<'a, P>,
{
    todo!()
}

#[cfg(test)]
mod tests {
    use super::super::elements::*;
    use super::*;
    use crate::builder::global_attributes::HtmlElement;
    use crate::builder::types::WordBreak;

    fn my_component(cx: Scope) -> Element {
        todo!()
    }

    fn mytemplate(s: &ScopeState) -> Element {
        base(s)
            .class("asda")
            .class("asda")
            .class("asda")
            .word_break(WordBreak::BreakAll)
            .class("asda")
            .onclick(move |_| {
                //
            })
            .children([
                base(s).class("asda").build(),
                base(s).class("asda").build(),
                base(s).class("asda").build(),
                base(s).class("asda").build(),
                base(s).class("asda").build(),
                base(s).class("asda").build(),
                // buildit(my_component, my_component as _, "name").build(),
            ])
            .build()
    }

    fn disambiguate(cx: Scope) {
        // Used by builder
        let builder = base(&cx);

        let _ = take(base);
        let _ = take(my_component);
        let _ = take(mytemplate);
        // Used by the macro
        // let r = buildit(base, base as _, "base").class("asda").build();
        // let r = buildit(my_component, my_component as _, "name").build();
        // let r = buildit(mytemplate, my_component as _, "custom_template").build();
    }
}

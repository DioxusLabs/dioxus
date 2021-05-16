fn main() {}

use dioxus_core::prelude::*;

static Example: FC<()> = |ctx, props| {
    ctx.render(dioxus_core::prelude::LazyNodes::new(move |ctx| {
        let bump = ctx.bump();
        dioxus::builder::ElementBuilder::new(ctx, "h1")
            .children([{
                use bumpalo::core_alloc::fmt::Write;
                let mut s = bumpalo::collections::String::new_in(bump);
                write!(s, "hello");
                dioxus::builder::text2(s)
            }])
            .finish()
    }))
};

fn main() {}

use dioxus_core::prelude::*;

static Example: FC<()> = |ctx| {
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

struct Props {
    text: String,
}
static Example2: FC<Props> = |ctx| {
    ctx.render(dioxus_core::prelude::LazyNodes::new(move |__ctx| {
        let bump = __ctx.bump();
        dioxus::builder::ElementBuilder::new(__ctx, "h1")
            .children([{ dioxus::builder::text3(bump, format_args!("{}", ctx.text)) }])
            // .children([{ dioxus::builder::text3(bump, format_args!("hello")) }])
            .finish()
    }))
};

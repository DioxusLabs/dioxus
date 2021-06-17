fn main() {}

use dioxus_core::prelude::*;

static Example: FC<()> = |ctx| {
    ctx.render(dioxus_core::prelude::LazyNodes::new(move |ctx| {
        let bump = ctx.bump();
        dioxus_core::builder::ElementBuilder::new(ctx, "h1")
            .children([dioxus_core::builder::text3(bump, format_args!("hello"))])
            .finish()
    }))
};

struct Props {
    text: String,
}
static Example2: FC<Props> = |ctx| {
    ctx.render(dioxus_core::prelude::LazyNodes::new(move |__ctx| {
        let bump = __ctx.bump();
        dioxus_core::builder::ElementBuilder::new(__ctx, "h1")
            .children([dioxus_core::builder::text3(
                bump,
                format_args!("{}", ctx.props.text),
            )])
            .finish()
    }))
};

fn main() {}

use dioxus_core::prelude::*;

static Example: FC<()> = |cx| {
    cx.render(dioxus_core::prelude::LazyNodes::new(move |cx| {
        let bump = cx.bump();
        dioxus_core::builder::ElementBuilder::new(cx, "h1")
            .children([dioxus_core::builder::text3(bump, format_args!("hello"))])
            .finish()
    }))
};

struct Props {
    text: String,
}
static Example2: FC<Props> = |cx| {
    cx.render(dioxus_core::prelude::LazyNodes::new(move |__cx| {
        let bump = __cx.bump();
        dioxus_core::builder::ElementBuilder::new(__cx, "h1")
            .children([dioxus_core::builder::text3(
                bump,
                format_args!("{}", cx.props.text),
            )])
            .finish()
    }))
};

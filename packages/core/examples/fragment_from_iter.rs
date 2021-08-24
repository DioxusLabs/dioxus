use dioxus_core::prelude::*;

fn main() {}

fn app(cx: Context<()>) -> DomTree {
    let vak = use_suspense(
        cx,
        || async {},
        |c, _res| c.render(LazyNodes::new(move |f| f.text(format_args!("")))),
    );

    let d1 = cx.render(LazyNodes::new(move |f| {
        f.raw_element(
            "div",
            None,
            [],
            [],
            [
                f.fragment_from_iter(vak),
                f.text(format_args!("")),
                f.text(format_args!("")),
                f.text(format_args!("")),
                f.text(format_args!("")),
            ],
            None,
        )
    }));

    cx.render(LazyNodes::new(move |f| {
        f.raw_element(
            "div",
            None,
            [],
            [],
            [
                f.text(format_args!("")),
                f.text(format_args!("")),
                f.text(format_args!("")),
                f.text(format_args!("")),
                d1.unwrap(),
            ],
            None,
        )
    }))
}

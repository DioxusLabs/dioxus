fn main() {}

use dioxus_core::prelude::*;

fn App(cx: Context<()>) -> DomTree {
    //
    let vak = use_suspense(
        cx,
        || async {},
        |c, res| {
            //
            c.render(LazyNodes::new(move |f| f.text(format_args!(""))))
        },
    );

    let d1 = cx.render(LazyNodes::new(move |f| {
        f.raw_element(
            "div",
            None,
            &mut [],
            &[],
            f.bump().alloc([
                f.fragment_from_iter(vak),
                f.text(format_args!("")),
                f.text(format_args!("")),
                f.text(format_args!("")),
                f.text(format_args!("")),
            ]),
            None,
        )
    }));

    cx.render(LazyNodes::new(move |f| {
        f.raw_element(
            "div",
            None,
            &mut [],
            &[],
            f.bump().alloc([
                f.text(format_args!("")),
                f.text(format_args!("")),
                f.text(format_args!("")),
                f.text(format_args!("")),
                f.fragment_from_iter(d1),
            ]),
            None,
        )
    }))
}

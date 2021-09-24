use dioxus_core::prelude::*;

fn main() {
    let mut dom = VirtualDom::new(EXAMPLE);
    dom.rebuild();
    println!("{}", dom);
}

pub static EXAMPLE: FC<()> = |cx, _| {
    let list = (0..10).map(|_f| LazyNodes::new(move |_f| todo!()));

    cx.render(LazyNodes::new(move |cx| {
        cx.raw_element(
            "div",
            None,
            [],
            [],
            [
                cx.text(format_args!("hello")),
                cx.text(format_args!("hello")),
                cx.fragment_from_iter(list),
            ],
            None,
        )
    }))
};

use dioxus_core::prelude::*;

fn main() {}

pub static Example: FC<()> = |cx| {
    let list = (0..10).map(|f| LazyNodes::new(move |f| todo!()));

    cx.render(LazyNodes::new(move |cx| {
        let bump = cx.bump();
        cx.raw_element(
            "div",
            None,
            &mut [],
            &mut [],
            cx.bump().alloc([
                cx.text(format_args!("hello")),
                cx.text(format_args!("hello")),
                cx.fragment_from_iter(list),
            ]),
            None,
        )
    }))
};

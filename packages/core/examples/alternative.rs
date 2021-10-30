use dioxus_core::prelude::*;
use dioxus_core_macro::format_args_f;
use dioxus_core_macro::rsx;
use dioxus_html as dioxus_elements;

fn main() {
    let mut dom = VirtualDom::new(EXAMPLE);
    dom.rebuild();
    println!("{}", dom);
}

pub static EXAMPLE: FC<()> = |(cx, _)| {
    let list = (0..10).map(|_f| {
        rsx! {
            "{_f}"
        }
    });
    // let list = (0..10).map(|_f| Some(Box::new(move |_f| todo!())));

    cx.render(Some(Box::new(move |cx| {
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
    })))
};

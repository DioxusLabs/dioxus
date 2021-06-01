use dioxus_core::component::fc_to_builder;
use dioxus_core::prelude::*;

static BLAH: FC<()> = |ctx| {
    let g = "asd".to_string();
    ctx.render(rsx! {
        div {
            SomeComponent {
                some_field: g
            }
        }
    })
};

#[derive(PartialEq, Props)]
pub struct ExampleProps {
    some_field: String,
}

static SomeComponent: FC<ExampleProps> = |ctx| {
    let blah = rsx! {
        div {}
    };

    let data = match 1 {
        1 => ctx.render(rsx! (
            div {
                h1 {}
                h3 {}
            }
        )),
        1 => ctx.render(rsx!( div { "abc" } )),
        2 => ctx.render(rsx!( div { "abc" } )),
        3 => ctx.render(rsx!( div { "abc" } )),
        _ => todo!(),
    };

    let data = match 1 {
        1 => ctx.render(rsx! (
            div {
                h1 {}
                h3 {}
            }
        )),
        1 => ctx.render(rsx!(
            div { "abc" }
        )),
        2 => ctx.render(rsx!(
            div { "abc" }
        )),
        3 => ctx.render(rsx!(
            div { "abc" }
        )),
        _ => todo!(),
    };

    let i = (0..10).map(|v| {
        rsx! {
            div {
                "{v}"
            }
        }
    });

    ctx.render(rsx! {
        div {
            ""
         }
    })
};

fn main() {}

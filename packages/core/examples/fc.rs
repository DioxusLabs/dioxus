use dioxus_core::prelude::*;

use dioxus_core_macro::fc;

use std::marker::PhantomData;
// #[derive(PartialEq)]
// pub struct Example<'a> {
//     b: &'a str,
//     ___p: std::marker::PhantomData<&'a ()>,
// }

// impl<'a> FC for Example<'a> {
//     fn render(ctx: Context<'_>, props: &Example<'a>) -> DomTree {
//         let Example { b, .. } = props;
//         {
//             ctx.render(rsx! {
//                 div { "abcd {b}" }
//             })
//         }
//     }
// }

// always try to fill in with Default

// #[fc]
fn Example(ctx: Context, a: &str, b: &str, c: &str) -> DomTree {
    ctx.render(rsx! {
        div {
            SomeComponent {
                a: "123"
            }
        }
    })
}

// #[fc]
fn SomeComponent(ctx: Context, a: &str, b: &str) -> DomTree {
    todo!()
}

fn main() {}

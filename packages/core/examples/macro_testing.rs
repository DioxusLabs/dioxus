use baller::Baller;
use dioxus_core::prelude::*;

fn main() {
    let g = rsx! {
        div {
            crate::baller::Baller {}
            baller::Baller {
            }
            Taller {
                a: "asd"
            }
            baller::Baller {}
            baller::Baller {}
            Baller {}
            div {
                a: "asd",
                a: "asd",
                a: "asd",
                a: "asd",
                div {
                    "asdas",
                    "asdas",
                    "asdas",
                    "asdas",
                    div {

                    },
                    div {

                    },
                }
            }
        }
    };
}

mod baller {
    use super::*;
    pub struct BallerProps {}

    pub fn Baller(ctx: Context, props: &()) -> DomTree {
        todo!()
    }
}

#[derive(Debug, PartialEq, Props)]
pub struct TallerProps {
    a: &'static str,
}

pub fn Taller(ctx: Context, props: &TallerProps) -> DomTree {
    todo!()
}

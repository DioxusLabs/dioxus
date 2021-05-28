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
            Baller {
                // todo: manual props
                // {...BallerProps {}}
            }
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
                    div {},
                    div {
                        // classes: {[ ("baller", true), ("maller", false) ]}
                        // class: "asdasd"
                        // class: "{customname}",
                        // class: {[("baller", true), ("hover", false)]}
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
    let b = true;
    todo!()
}

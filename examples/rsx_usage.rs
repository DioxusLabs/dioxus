fn main() {}

mod components {
    use baller::Baller;
    use dioxus::prelude::*;

    fn example() {
        let g = rsx! {
            div {
                crate::components::baller::Baller {}
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
                            class: "asdasd"
                        },
                    }
                }
            }
        };
    }

    mod baller {
        use super::*;
        pub struct BallerProps {}

        pub fn Baller(ctx: Context<()>) -> VNode {
            todo!()
        }
    }

    #[derive(Debug, PartialEq, Props)]
    pub struct TallerProps {
        a: &'static str,
    }

    pub fn Taller(ctx: Context<TallerProps>) -> VNode {
        let b = true;
        todo!()
    }
}

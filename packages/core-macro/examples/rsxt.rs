use dioxus_core_macro::rsx;

pub mod dioxus {
    pub mod builder {
        pub struct Builder;

        struct AttrVal;

        impl Into<AttrVal> for &'static str {
            fn into(self) -> AttrVal {
                todo!()
            }
        }

        impl Into<AttrVal> for String {
            fn into(self) -> AttrVal {
                todo!()
            }
        }
        // impl<T> From<T> for AttrVal {
        //     fn from(_: T) -> Self {
        //         todo!()
        //     }
        // }

        impl Builder {
            // fn attr<T>(mut self, key: &str, value: impl Into<AttrVal>) -> Self {
            pub fn attr<T>(mut self, key: &str, value: T) -> Self {
                Self
            }

            pub fn on<T>(mut self, key: &str, value: T) -> Self {
                Self
            }

            pub fn finish(mut self) {
                // Self
            }
        }

        pub struct Bump;
        pub fn div(bump: &Bump) -> Builder {
            todo!()
        }
        pub fn h1(bump: &Bump) -> Builder {
            todo!()
        }
        pub fn h2(bump: &Bump) -> Builder {
            todo!()
        }
    }
}
use dioxus::builder::Bump;
pub fn main() {
    // render(rsx! {
    //     div { // we can actually support just a list of nodes too
    //         h1 {"Hello Dioxus"}
    //         p {"This is a beautful app you're building"}
    //         section {
    //             "custom section to the rescue",
    //             class: "abc123"
    //         }
    //         span {
    //             class: "abc123"
    //             "Try backwards too."
    //             "Anything goes!"
    //             "As long as it's within the rules"
    //             {0..10.map(|f| rsx!{
    //                 div {
    //                     h3 {"totally okay to drop in iterators and expressions"}
    //                     p {"however, debug information is lost"}
    //                 }
    //             })}
    //         }
    //         span {
    //             "Feel free"
    //             class: "abc123"
    //             "To mix to your heart's content"
    //         }
    //         span { class: "some-very-long-and-tedious-class-name-is-now-separated"
    //             "Very ergonomic"
    //         }
    //         span { "Innovative design 🛠"
    //             class: "some-very-long-and-tedious-class-name-is-now-separated"
    //         }
    //     }
    // });

    let g = String::from("asd");

    // let lazy = rsx! {
    //     div {
    //         a: "asd",
    //         a: "asd",
    //         a: "asd",
    //         a: "asd",
    //         a: "asd",
    //         // a: {rsx!{ h1 {"hello world"} }}, // include
    //         a: {&g},
    //         b: {1 + 2},
    //         onclick: {move |e: ()| {
    //             println!("hello world!")
    //         }},
    //         div {
    //             a: "asd"
    //             div {
    //                 div {
    //                     div {

    //                     }
    //                 }
    //             }
    //         }
    //         h1 {

    //         }
    //         h2 {
    //             "child"
    //         }
    //         "Childnode"
    //     }
    // };

    // render(lazy);
}

fn render(f: impl Fn(&dioxus::builder::Bump)) {}

use std::fmt::Arguments;

use dioxus_core_macro::{format_args_f, rsx};

fn blah() {
    let cx = ScopeState {};

    let g = rsx! {
        div {
            id: "asd",
            div {}
            div {}
            div {}
            div {}
            div {}
            div {}
            div {}
            div {}
            div {}
        }
    };
}

struct ScopeState {}

mod dioxus_elements {
    pub mod builder {

        use super::super::*;

        pub(crate) fn div(cx: &ScopeState) -> Builder {
            Builder {}
        }
    }
}

struct Builder {}

impl Builder {
    fn build(self) {}

    fn id(self, id: Arguments) -> Self {
        self
    }

    fn child<F>(mut self, f: F) -> Self {
        self
    }
}

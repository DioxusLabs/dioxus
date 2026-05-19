use dioxus::prelude::*;
use dioxus_renderer_oracle::Sequence;

#[test]
fn bool_test() {
    Sequence::new().render(rsx! { div { hidden: false } }).run();
}

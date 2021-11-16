#![allow(unused, non_upper_case_globals)]

use dioxus::{prelude::*, DomEdit, Mutations};
use dioxus_core as dioxus;
use dioxus_core_macro::*;
use dioxus_html as dioxus_elements;

use DomEdit::*;

mod test_logging;

#[test]
fn shared_state_test() {
    struct MySharedState(&'static str);

    static App: FC<()> = |cx, props| {
        cx.provide_state(MySharedState("world!"));
        cx.render(rsx!(Child {}))
    };

    static Child: FC<()> = |cx, props| {
        let shared = cx.consume_state::<MySharedState>()?;
        cx.render(rsx!("Hello, {shared.0}"))
    };

    let mut dom = VirtualDom::new(App);
    let Mutations { edits, .. } = dom.rebuild();

    assert_eq!(
        edits,
        [
            CreateTextNode {
                root: 1,
                text: "Hello, world!"
            },
            AppendChildren { many: 1 },
        ]
    );
}

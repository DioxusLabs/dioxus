use dioxus::{nodes::VSuspended, prelude::*, DomEdit, TestDom};
use dioxus_core as dioxus;
use dioxus_core_macro::*;
use dioxus_html as dioxus_elements;

use DomEdit::*;

mod test_logging;

fn new_dom() -> TestDom {
    const IS_LOGGING_ENABLED: bool = false;
    test_logging::set_up_logging(IS_LOGGING_ENABLED);
    TestDom::new()
}

#[test]
fn shared_state_test() {
    struct MySharedState(&'static str);

    static App: FC<()> = |cx, props| {
        cx.provide_state(MySharedState("world!"));
        rsx!(cx, Child {})
    };

    static Child: FC<()> = |cx, props| {
        let shared = cx.consume_state::<MySharedState>()?;
        rsx!(cx, "Hello, {shared.0}")
    };

    let mut dom = VirtualDom::new(App);
    let Mutations { edits, .. } = dom.rebuild();

    assert_eq!(
        edits,
        [
            CreateTextNode {
                root: 0,
                text: "Hello, world!"
            },
            AppendChildren { many: 1 },
        ]
    );
}

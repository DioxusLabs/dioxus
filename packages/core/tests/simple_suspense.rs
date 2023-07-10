//! Verify that tasks get polled by the virtualdom properly, and that we escape wait_for_work safely

use dioxus::core::{ElementId, Mutation};
use dioxus::prelude::*;
use std::time::Duration;

#[tokio::test]
async fn it_works() {
    // Make the dom
    let mut dom = VirtualDom::new(app);

    // Progress only its state, working through any suspense
    let muts = dom
        .render_with_deadline(tokio::time::sleep(Duration::from_millis(1000)))
        .await;

    // The rendered output should only be the last contents
    assert_eq!(
        muts.edits,
        vec![Mutation::CreateTextNode { id: ElementId(1), value: "5" }]
    );
}

fn app(cx: Scope) -> Element {
    let count = use_state(cx, || 0);

    if **count < 5 {
        let count = count.clone();
        cx.spawn(async move {
            tokio::time::sleep(Duration::from_millis(10)).await;
            count.set(*count + 1);
        });
        return cx.suspend();
    };

    render!("{count}")
}

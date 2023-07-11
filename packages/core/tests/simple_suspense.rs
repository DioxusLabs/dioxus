//! Verify that tasks get polled by the virtualdom properly, and that we escape wait_for_work safely

use dioxus::core::{ElementId, Mutation};
use dioxus::prelude::*;
use std::time::Duration;

#[tokio::test]
async fn simple_suspense() {
    // Make the dom
    let mut dom = VirtualDom::new(app);

    // Wait for suspense to properly progress
    // Once the dom has been resolved, then we'll diff it against itself
    dom.wait_for_suspsnese().await;

    // Now render immediately, computing the diff for this scope
    // Will go until there is no more diff to compute :)
    let muts = dom.compute_diff(ScopeId(0));

    // The rendered output should only be the last contents
    assert_eq!(
        muts.edits,
        vec![Mutation::CreateTextNode { id: ElementId(1), value: "5" }]
    );
}

fn app(cx: Scope) -> Element {
    let count = use_state(cx, || 0);

    // The scope will suspend until the count is 5 here
    // We then start working on the component below
    //
    // We're taking advantage of the scheduler directly here
    if **count < 5 {
        // Schedule a task that will increment the count
        cx.spawn({
            let count = count.clone();
            async move {
                tokio::time::sleep(Duration::from_millis(10)).await;
                count.with_mut(|f| *f += 1);
            }
        });

        // Suspend the scope
        return cx.suspend();
    };

    println!("finally {:?}!", cx.is_suspended());

    cx.render(rsx!(Child { count: count.clone() }))
}

#[inline_props]
fn Child(cx: Scope, count: UseState<i32>) -> Element {
    println!("rendering child...");
    let update_any = cx.schedule_update();

    if *count.get() < 6 {
        cx.spawn({
            let count = count.clone();
            async move {
                tokio::time::sleep(Duration::from_millis(10)).await;
                count.with_mut(|f| *f += 1);
                update_any();
            }
        });

        return cx.suspend();
    }

    println!("All suspense complete!");

    render! { "{count}" }
}

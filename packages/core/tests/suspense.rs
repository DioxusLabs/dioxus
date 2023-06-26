use dioxus::core::ElementId;
use dioxus::core::{Mutation::*, SuspenseContext};
use dioxus::prelude::*;
use std::future::IntoFuture;
use std::rc::Rc;
use std::time::Duration;
use tokio::time::sleep;

#[test]
fn it_works() {
    // wait just a moment, not enough time for the boundary to resolve

    tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .unwrap()
        .block_on(async {
            let mut dom = VirtualDom::new(app);

            {
                let mutations = dom.rebuild().santize();

                // We should at least get the top-level template in before pausing for the children
                // note: we dont test template edits anymore
                // assert_eq!(
                //     mutations.templates,
                //     [
                //         CreateElement { name: "div" },
                //         CreateStaticText { value: "Waiting for child..." },
                //         CreateStaticPlaceholder,
                //         AppendChildren { m: 2 },
                //         SaveTemplate { name: "template", m: 1 }
                //     ]
                // );

                // And we should load it in and assign the placeholder properly
                assert_eq!(
                    mutations.edits,
                    [
                        LoadTemplate { name: "template", index: 0, id: ElementId(1) },
                        // hmmmmmmmmm.... with suspense how do we guarantee that IDs increase linearly?
                        // can we even?
                        AssignId { path: &[1], id: ElementId(3) },
                        AppendChildren { m: 1, id: ElementId(0) },
                    ]
                );
            }

            dom.wait_for_work().await;
        });
}

fn app(cx: Scope) -> Element {
    cx.render(rsx!(
        div {
            "Waiting for child..."
            suspense_boundary {}
        }
    ))
}

fn suspense_boundary(cx: Scope) -> Element {
    cx.use_hook(|| {
        cx.provide_context(Rc::new(SuspenseContext::new(cx.scope_id())));
    });

    // Ensure the right types are found
    cx.has_context::<Rc<SuspenseContext>>().unwrap();

    cx.render(rsx!(suspense_child {}))
}

fn suspense_child(cx: Scope) -> Element {
    use_future!(cx, || tokio::time::sleep(Duration::from_millis(10))).await;

    cx.render(rsx!(suspense_text {}))
}

fn suspense_text(cx: Scope) -> Element {
    let username = use_future!(cx, || async {
        sleep(Duration::from_secs(1)).await;
        "async child 1"
    });

    let age = use_future!(cx, || async {
        sleep(Duration::from_secs(2)).await;
        1234
    });

    cx.render(rsx!( div { "Hello! {username}, you are {age}, {_user} {_age}" } ))
}

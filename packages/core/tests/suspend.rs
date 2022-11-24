use dioxus::core::ElementId;
use dioxus::core::{Mutation::*, SuspenseBoundary};
use dioxus::prelude::*;
use dioxus_core::SuspenseContext;
use std::{rc::Rc, time::Duration};

#[tokio::test]
async fn it_works() {
    let mut dom = VirtualDom::new(app);

    let mutations = dom.rebuild().santize();

    // We should at least get the top-level template in
    assert_eq!(
        mutations.template_mutations,
        [
            CreateElement { name: "div" },
            CreateStaticText { value: "Waiting for child..." },
            CreatePlaceholder { id: ElementId(0) },
            AppendChildren { m: 2 },
            SaveTemplate { name: "template", m: 1 }
        ]
    );

    // And we should load it in and assign the placeholder properly
    assert_eq!(
        mutations.edits,
        [
            LoadTemplate { name: "template", index: 0, id: ElementId(1) },
            // hmmmmmmmmm.... with suspense how do we guarantee that IDs increase linearly?
            // can we even?
            AssignId { path: &[1], id: ElementId(3) },
            AppendChildren { m: 1 },
        ]
    );

    // wait just a moment, not enough time for the boundary to resolve

    dom.wait_for_work().await;
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
    cx.use_hook(|| cx.provide_context(Rc::new(SuspenseBoundary::new(cx.scope_id()))));

    // Ensure the right types are found
    cx.has_context::<SuspenseContext>().unwrap();

    cx.render(rsx!(async_child {}))
}

async fn async_child(cx: Scope<'_>) -> Element {
    use_future!(cx, || tokio::time::sleep(Duration::from_millis(10))).await;
    cx.render(rsx!(async_text {}))
}

async fn async_text(cx: Scope<'_>) -> Element {
    cx.render(rsx!("async_text"))
}

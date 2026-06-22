use dioxus::prelude::*;
use dioxus_core::{
    ScopeId, SuspenseBoundaryProps, WriteMutations, current_scope_id, generation,
};
use dioxus_renderer_oracle::{RendererOracle, SnapshotNode};
use pretty_assertions::assert_eq;
use std::cell::Cell;
use std::future::poll_fn;
use std::task::Poll;

fn div_snapshot(children: Vec<SnapshotNode>) -> SnapshotNode {
    SnapshotNode::Element {
        tag: "div".into(),
        namespace: None,
        attrs: vec![],
        listeners: vec![],
        children,
    }
}

thread_local! {
    static SUSPENDED_CHILD_SCOPE: Cell<Option<ScopeId>> = const { Cell::new(None) };
}

/// Regression for the streaming-resume panic when a suspense boundary's fallback renders to zero
/// DOM nodes. With markerless placeholders an empty fallback has no element to `replace_with`, so
/// [`SuspenseBoundaryProps::resolve_suspense`] must splice the streamed nodes in at the boundary's
/// position (here, before the trailing "after" text) instead of panicking on a missing placeholder.
#[test]
fn resolve_suspense_with_empty_fallback() {
    fn empty_fallback_app() -> Element {
        rsx! {
            div {
                "before"
                SuspenseBoundary {
                    fallback: |_| rsx! {},
                    pending_child {}
                }
                "after"
            }
        }
    }

    fn pending_child() -> Element {
        SUSPENDED_CHILD_SCOPE.with(|s| s.set(Some(current_scope_id())));
        // Suspend on a task that never resolves so the boundary stays in its (empty) fallback.
        let task = spawn(std::future::pending::<()>());
        suspend(task)?;
        rsx! { "resolved-child" }
    }

    let mut dom = VirtualDom::new(empty_fallback_app);
    let mut oracle = RendererOracle::new();
    oracle.rebuild(&mut dom);

    // The empty fallback contributes no DOM, so the div holds only its static text.
    oracle.assert_snapshot_eq(&[div_snapshot(vec![
        SnapshotNode::Text("before".into()),
        SnapshotNode::Text("after".into()),
    ])]);

    let child_scope = SUSPENDED_CHILD_SCOPE
        .with(|s| s.get())
        .expect("pending child rendered");
    let boundary_scope = dom
        .runtime()
        .parent_scope(child_scope)
        .expect("suspended child has a parent boundary scope");

    // Simulate the server stream arriving: push one resolved text node as the replacement. With no
    // fallback element to replace, this must splice in at the boundary's slot (before "after").
    SuspenseBoundaryProps::resolve_suspense(boundary_scope, &mut dom, &mut oracle, |to| {
        to.create_text("resolved");
        1
    });

    oracle.assert_stack_clean();
    oracle.assert_snapshot_eq(&[div_snapshot(vec![
        SnapshotNode::Text("before".into()),
        SnapshotNode::Text("resolved".into()),
        SnapshotNode::Text("after".into()),
    ])]);
}

async fn poll_three_times() {
    // Poll each task 3 times
    let mut count = 0;
    poll_fn(|cx| {
        println!("polling... {}", count);
        if count < 3 {
            count += 1;
            cx.waker().wake_by_ref();
            Poll::Pending
        } else {
            Poll::Ready(())
        }
    })
    .await;
}

#[test]
fn suspense_resolves_ssr() {
    // wait just a moment, not enough time for the boundary to resolve
    tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap()
        .block_on(async {
            let mut dom = VirtualDom::new(app);
            dom.rebuild_in_place();
            dom.wait_for_suspense().await;
            dom.render_immediate(&mut dioxus_core::NoOpMutations);
            let out = dioxus_ssr::render(&dom);

            assert_eq!(out, "<div>Waiting for... child</div>");
        });
}

fn app() -> Element {
    rsx!(
        div {
            "Waiting for... "
            SuspenseBoundary {
                fallback: |_| rsx! { "fallback" },
                suspended_child {}
            }
        }
    )
}

fn suspended_child() -> Element {
    let mut val = use_signal(|| 0);

    // Tasks that are not suspended should never be polled
    spawn(async move {
        panic!("Non-suspended task was polled");
    });

    // Memos should still work like normal
    let memo = use_memo(move || val * 2);
    assert_eq!(memo, val * 2);

    if val() < 3 {
        let task = spawn(async move {
            poll_three_times().await;
            println!("waiting... {}", val);
            val += 1;
        });
        suspend(task)?;
    }

    rsx!("child")
}

/// When switching from a suspense fallback to the real child, the state of that component must be kept
#[test]
fn suspense_keeps_state() {
    tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .unwrap()
        .block_on(async {
            let mut dom = VirtualDom::new(app);
            dom.rebuild(&mut dioxus_core::NoOpMutations);
            dom.render_suspense_immediate().await;

            let out = dioxus_ssr::render(&dom);

            assert_eq!(out, "fallback");

            dom.wait_for_suspense().await;
            let out = dioxus_ssr::render(&dom);

            assert_eq!(out, "<div>child with future resolved</div>");
        });

    fn app() -> Element {
        rsx! {
            SuspenseBoundary {
                fallback: |_| rsx! { "fallback" },
                Child {}
            }
        }
    }

    #[component]
    fn Child() -> Element {
        let mut future_resolved = use_signal(|| false);

        let task = use_hook(|| {
            spawn(async move {
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                future_resolved.set(true);
            })
        });
        if !future_resolved() {
            suspend(task)?;
        }

        println!("future resolved: {future_resolved:?}");

        if future_resolved() {
            rsx! {
                div { "child with future resolved" }
            }
        } else {
            rsx! {
                div { "this should never be rendered" }
            }
        }
    }
}

/// spawn doesn't run in suspense
#[test]
fn suspense_does_not_poll_spawn() {
    tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .unwrap()
        .block_on(async {
            let mut dom = VirtualDom::new(app);
            dom.rebuild(&mut dioxus_core::NoOpMutations);

            dom.wait_for_suspense().await;
            let out = dioxus_ssr::render(&dom);

            assert_eq!(out, "<div>child with future resolved</div>");
        });

    fn app() -> Element {
        rsx! {
            SuspenseBoundary {
                fallback: |_| rsx! { "fallback" },
                Child {}
            }
        }
    }

    #[component]
    fn Child() -> Element {
        let mut future_resolved = use_signal(|| false);

        // futures that are spawned, but not suspended should never be polled
        use_hook(|| {
            spawn(async move {
                panic!("Non-suspended task was polled");
            });
        });

        let task = use_hook(|| {
            spawn(async move {
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                future_resolved.set(true);
            })
        });
        if !future_resolved() {
            suspend(task)?;
        }

        rsx! {
            div { "child with future resolved" }
        }
    }
}

/// suspended nodes are not mounted, so they should not run effects
#[test]
fn suspended_nodes_dont_trigger_effects() {
    tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .unwrap()
        .block_on(async {
            let mut dom = VirtualDom::new(app);
            dom.rebuild(&mut dioxus_core::NoOpMutations);

            let work = async move {
                loop {
                    dom.wait_for_work().await;
                    dom.render_immediate(&mut dioxus_core::NoOpMutations);
                }
            };
            tokio::select! {
                _ = work => {},
                _ = tokio::time::sleep(std::time::Duration::from_millis(100)) => {}
            }
        });

    fn app() -> Element {
        rsx! {
            SuspenseBoundary {
                fallback: |_| rsx! { "fallback" },
                Child {}
            }
        }
    }

    #[component]
    fn Child() -> Element {
        let mut future_resolved = use_signal(|| false);

        use_effect(|| panic!("effects should not run during suspense"));

        let task = use_hook(|| {
            spawn(async move {
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                future_resolved.set(true);
            })
        });
        if !future_resolved() {
            suspend(task)?;
        }

        rsx! {
            div { "child with future resolved" }
        }
    }
}

/// Make sure we keep any state of components when we switch from a resolved future to a suspended future
#[test]
fn resolved_to_suspended() {
    static SUSPENDED: GlobalSignal<bool> = Signal::global(|| false);

    tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .unwrap()
        .block_on(async {
            let mut dom = VirtualDom::new(app);
            dom.rebuild(&mut dioxus_core::NoOpMutations);

            let out = dioxus_ssr::render(&dom);

            assert_eq!(out, "rendered 1 times");

            dom.in_scope(ScopeId::APP, || *SUSPENDED.write() = true);

            dom.render_suspense_immediate().await;
            let out = dioxus_ssr::render(&dom);

            assert_eq!(out, "fallback");

            dom.wait_for_suspense().await;
            let out = dioxus_ssr::render(&dom);

            assert_eq!(out, "rendered 3 times");
        });

    fn app() -> Element {
        rsx! {
            SuspenseBoundary {
                fallback: |_| rsx! { "fallback" },
                Child {}
            }
        }
    }

    #[component]
    fn Child() -> Element {
        let mut render_count = use_signal(|| 0);
        render_count += 1;

        let mut task = use_hook(|| CopyValue::new(None));

        tracing::info!("render_count: {}", render_count.peek());

        if SUSPENDED() {
            if task().is_none() {
                task.set(Some(spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    tracing::info!("task finished");
                    *SUSPENDED.write() = false;
                })));
            }
            suspend(task().unwrap())?;
        }

        rsx! {
            "rendered {render_count.peek()} times"
        }
    }
}

/// Make sure suspense tells the renderer that a suspense boundary was resolved
#[test]
fn suspense_tracks_resolved() {
    tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .unwrap()
        .block_on(async {
            let mut dom = VirtualDom::new(app);
            dom.rebuild(&mut dioxus_core::NoOpMutations);

            dom.render_suspense_immediate().await;
            dom.wait_for_suspense_work().await;
            let resolved = dom.render_suspense_immediate().await;
            assert_eq!(resolved.len(), 1);
        });

    fn app() -> Element {
        rsx! {
            SuspenseBoundary {
                fallback: |_| rsx! { "fallback" },
                Child {}
            }
        }
    }

    #[component]
    fn Child() -> Element {
        let mut resolved = use_signal(|| false);
        let task = use_hook(|| {
            spawn(async move {
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                tracing::info!("task finished");
                resolved.set(true);
            })
        });

        if resolved() {
            println!("suspense is resolved");
        } else {
            println!("suspense is not resolved");
            suspend(task)?;
        }

        rsx! {
            "child"
        }
    }
}

// Regression test for https://github.com/DioxusLabs/dioxus/issues/2783
#[test]
fn toggle_suspense() {
    fn app() -> Element {
        rsx! {
            SuspenseBoundary {
                fallback: |_| rsx! { "fallback" },
                if generation() % 2 == 0 {
                    Page {}
                } else {
                    Home {}
                }
            }
        }
    }

    #[component]
    pub fn Home() -> Element {
        let _calculation = use_resource(|| async move {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            1 + 1
        })
        .suspend()?;
        rsx! {
            "hello world"
        }
    }

    #[component]
    pub fn Page() -> Element {
        rsx! {
            "goodbye world"
        }
    }

    fn expected_page() -> Element {
        rsx! {
            "goodbye world"
        }
    }

    fn expected_fallback() -> Element {
        rsx! {
            "fallback"
        }
    }

    fn expected_home() -> Element {
        rsx! {
            "hello world"
        }
    }

    tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .unwrap()
        .block_on(async {
            let mut dom = VirtualDom::new(app);
            let mut oracle = RendererOracle::new();
            oracle.rebuild(&mut dom);
            oracle.assert_matches(expected_page);

            dom.mark_dirty(ScopeId::APP);
            oracle.render(&mut dom);
            oracle.assert_matches(expected_fallback);

            dom.wait_for_work().await;
            oracle.render(&mut dom);
            oracle.assert_matches(expected_home);
        });
}

#[test]
fn nested_suspense_resolves_client() {
    fn app() -> Element {
        rsx! {
            SuspenseBoundary {
                fallback: move |_| rsx! {},
                LoadTitle {}
            }
            MessageWithLoader { id: 0 }
        }
    }

    #[component]
    fn MessageWithLoader(id: usize) -> Element {
        rsx! {
            SuspenseBoundary {
                fallback: move |_| rsx! {
                    "Loading {id}..."
                },
                Message { id }
            }
        }
    }

    #[component]
    fn LoadTitle() -> Element {
        let title = use_resource(move || async_content(0)).suspend()?();

        rsx! {
            document::Title { "{title.title}" }
        }
    }

    #[component]
    fn Message(id: usize) -> Element {
        let message = use_resource(move || async_content(id)).suspend()?();

        rsx! {
            h2 {
                id: "title-{id}",
                "{message.title}"
            }
            p {
                id: "body-{id}",
                "{message.body}"
            }
            div {
                id: "children-{id}",
                padding: "10px",
                for child in message.children {
                    MessageWithLoader { id: child }
                }
            }
        }
    }

    #[derive(Clone)]
    pub struct Content {
        title: String,
        body: String,
        children: Vec<usize>,
    }

    async fn async_content(id: usize) -> Content {
        let content_tree = [
            Content {
                title: "The robot says hello world".to_string(),
                body: "The robot becomes sentient and says hello world".to_string(),
                children: vec![1, 2],
            },
            Content {
                title: "The world says hello back".to_string(),
                body: "In a stunning turn of events, the world collectively unites and says hello back"
                    .to_string(),
                children: vec![],
            },
            Content {
                title: "Goodbye Robot".to_string(),
                body: "The robot says goodbye".to_string(),
                children: vec![3],
            },
            Content {
                title: "Goodbye Robot again".to_string(),
                body: "The robot says goodbye again".to_string(),
                children: vec![],
            },
        ];
        poll_three_times().await;
        content_tree[id].clone()
    }

    fn expected_loading_root() -> Element {
        rsx! {
            "Loading 0..."
        }
    }

    fn expected_root_message_loading_children() -> Element {
        rsx! {
            h2 {
                id: "title-0",
                "The robot says hello world"
            }
            p {
                id: "body-0",
                "The robot becomes sentient and says hello world"
            }
            div {
                id: "children-0",
                padding: "10px",
                "Loading 1..."
                "Loading 2..."
            }
        }
    }

    fn expected_nested_messages_loading_grandchild() -> Element {
        rsx! {
            h2 {
                id: "title-0",
                "The robot says hello world"
            }
            p {
                id: "body-0",
                "The robot becomes sentient and says hello world"
            }
            div {
                id: "children-0",
                padding: "10px",
                h2 {
                    id: "title-1",
                    "The world says hello back"
                }
                p {
                    id: "body-1",
                    "In a stunning turn of events, the world collectively unites and says hello back"
                }
                div {
                    id: "children-1",
                    padding: "10px",
                }
                h2 {
                    id: "title-2",
                    "Goodbye Robot"
                }
                p {
                    id: "body-2",
                    "The robot says goodbye"
                }
                div {
                    id: "children-2",
                    padding: "10px",
                    "Loading 3..."
                }
            }
        }
    }

    fn expected_resolved_tree() -> Element {
        rsx! {
            h2 {
                id: "title-0",
                "The robot says hello world"
            }
            p {
                id: "body-0",
                "The robot becomes sentient and says hello world"
            }
            div {
                id: "children-0",
                padding: "10px",
                h2 {
                    id: "title-1",
                    "The world says hello back"
                }
                p {
                    id: "body-1",
                    "In a stunning turn of events, the world collectively unites and says hello back"
                }
                div {
                    id: "children-1",
                    padding: "10px",
                }
                h2 {
                    id: "title-2",
                    "Goodbye Robot"
                }
                p {
                    id: "body-2",
                    "The robot says goodbye"
                }
                div {
                    id: "children-2",
                    padding: "10px",
                    h2 {
                        id: "title-3",
                        "Goodbye Robot again"
                    }
                    p {
                        id: "body-3",
                        "The robot says goodbye again"
                    }
                    div {
                        id: "children-3",
                        padding: "10px",
                    }
                }
            }
        }
    }

    tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap()
        .block_on(async {
            let mut dom = VirtualDom::new(app);
            let mut oracle = RendererOracle::new();
            oracle.rebuild(&mut dom);
            oracle.assert_matches(expected_loading_root);

            dom.wait_for_work().await;
            oracle.render(&mut dom);
            oracle.assert_matches(expected_root_message_loading_children);

            dom.wait_for_work().await;
            oracle.render(&mut dom);
            oracle.assert_matches(expected_nested_messages_loading_grandchild);

            dom.wait_for_work().await;
            oracle.render(&mut dom);
            oracle.assert_matches(expected_resolved_tree);
        });
}

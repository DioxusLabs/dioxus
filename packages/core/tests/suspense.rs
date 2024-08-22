use dioxus::prelude::*;
use dioxus_core::{AttributeValue, ElementId, Mutation};
use pretty_assertions::assert_eq;
use std::future::poll_fn;
use std::task::Poll;

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
    fn RerendersFrequently() -> Element {
        let mut count = use_signal(|| 0);

        use_future(move || async move {
            for _ in 0..100 {
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                count.set(count() + 1);
            }
        });

        rsx! {
            div { "rerenders frequently" }
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

            dom.in_runtime(|| ScopeId::APP.in_runtime(|| *SUSPENDED.write() = true));

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
            assert_eq!(
                dom.render_suspense_immediate().await,
                vec![ScopeId(ScopeId::APP.0 + 1)]
            );
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
    use dioxus::prelude::*;

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

    tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .unwrap()
        .block_on(async {
            let mut dom = VirtualDom::new(app);
            let mutations = dom.rebuild_to_vec();

            // First create goodbye world
            println!("{:#?}", mutations);
            assert_eq!(
                mutations.edits,
                [
                    Mutation::LoadTemplate { index: 0, id: ElementId(1) },
                    Mutation::AppendChildren { id: ElementId(0), m: 1 }
                ]
            );

            dom.mark_dirty(ScopeId::APP);
            let mutations = dom.render_immediate_to_vec();

            // Then replace that with nothing
            println!("{:#?}", mutations);
            assert_eq!(
                mutations.edits,
                [
                    Mutation::CreatePlaceholder { id: ElementId(2) },
                    Mutation::ReplaceWith { id: ElementId(1), m: 1 },
                ]
            );

            dom.wait_for_work().await;
            let mutations = dom.render_immediate_to_vec();

            // Then replace it with a placeholder
            println!("{:#?}", mutations);
            assert_eq!(
                mutations.edits,
                [
                    Mutation::LoadTemplate { index: 0, id: ElementId(1) },
                    Mutation::ReplaceWith { id: ElementId(2), m: 1 },
                ]
            );

            dom.wait_for_work().await;
            let mutations = dom.render_immediate_to_vec();

            // Then replace it with the resolved node
            println!("{:#?}", mutations);
            assert_eq!(
                mutations.edits,
                [
                    Mutation::CreatePlaceholder { id: ElementId(2,) },
                    Mutation::ReplaceWith { id: ElementId(1,), m: 1 },
                    Mutation::LoadTemplate { index: 0, id: ElementId(1) },
                    Mutation::ReplaceWith { id: ElementId(2), m: 1 },
                ]
            );
        });
}

#[test]
fn nested_suspense_resolves_client() {
    use Mutation::*;

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

    // wait just a moment, not enough time for the boundary to resolve
    tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap()
        .block_on(async {
            let mut dom = VirtualDom::new(app);
            let mutations = dom.rebuild_to_vec();
            // Initial loading message and loading title
            assert_eq!(
                mutations.edits,
                vec![
                    CreatePlaceholder { id: ElementId(1,) },
                    CreateTextNode { value: "Loading 0...".to_string(), id: ElementId(2,) },
                    AppendChildren { id: ElementId(0,), m: 2 },
                ]
            );

            dom.wait_for_work().await;
            // DOM STATE:
            // placeholder // ID: 1
            // "Loading 0..." // ID: 2
            let mutations = dom.render_immediate_to_vec();
            // Fill in the contents of the initial message and start loading the nested suspense
            // The title also finishes loading
            assert_eq!(
                mutations.edits,
                vec![
                    // Creating and swapping these placeholders doesn't do anything
                    // It is just extra work that we are forced to do because mutations are not
                    // reversible. We start rendering the children and then realize it is suspended.
                    // Then we need to replace what we just rendered with the suspense placeholder
                    CreatePlaceholder { id: ElementId(3,) },
                    ReplaceWith { id: ElementId(1,), m: 1 },

                    // Replace the pending placeholder with the title placeholder
                    CreatePlaceholder { id: ElementId(1,) },
                    ReplaceWith { id: ElementId(3,), m: 1 },

                    // Replace loading... with a placeholder for us to fill in later
                    CreatePlaceholder { id: ElementId(3,) },
                    ReplaceWith { id: ElementId(2,), m: 1 },

                    // Load the title
                    LoadTemplate {  index: 0, id: ElementId(2,) },
                    CreateTextNode { value: "The robot says hello world".to_string(), id: ElementId(4,) },
                    ReplacePlaceholder { path: &[0,], m: 1 },
                    SetAttribute {
                        name: "id",
                        ns: None,
                        value: AttributeValue::Text("title-0".to_string()),
                        id: ElementId(2,),
                    },

                    // Then load the body
                    LoadTemplate {  index: 1, id: ElementId(5,) },
                    CreateTextNode { value: "The robot becomes sentient and says hello world".to_string(), id: ElementId(6,) },
                    ReplacePlaceholder { path: &[0,], m: 1 },
                    SetAttribute {
                        name: "id",
                        ns: None,
                        value: AttributeValue::Text("body-0".to_string()),
                        id: ElementId(5,),
                    },

                    // Then load the suspended children
                    LoadTemplate {  index: 2, id: ElementId(7,) },
                    CreateTextNode { value: "Loading 1...".to_string(), id: ElementId(8,) },
                    CreateTextNode { value: "Loading 2...".to_string(), id: ElementId(9,) },
                    ReplacePlaceholder { path: &[0,], m: 2 },
                    SetAttribute {
                        name: "id",
                        ns: None,
                        value: AttributeValue::Text("children-0".to_string()),
                        id: ElementId(7,),
                    },

                    // Finally replace the loading placeholder in the body with the resolved children
                    ReplaceWith { id: ElementId(3,), m: 3 },
                ]
            );

            dom.wait_for_work().await;
            // DOM STATE:
            // placeholder // ID: 1
            // h2 // ID: 2
            // p // ID: 5
            // div // ID: 7
            //   "Loading 1..." // ID: 8
            //   "Loading 2..." // ID: 9
            let mutations = dom.render_immediate_to_vec();
            assert_eq!(
                mutations.edits,
                vec![
                    // Replace the first loading placeholder with a placeholder for us to fill in later
                    CreatePlaceholder {
                        id: ElementId(
                            3,
                        ),
                    },
                    ReplaceWith {
                        id: ElementId(
                            8,
                        ),
                        m: 1,
                    },

                    // Load the nested suspense
                    LoadTemplate {

                        index: 0,
                        id: ElementId(
                            8,
                        ),
                    },
                    CreateTextNode { value: "The world says hello back".to_string(), id: ElementId(10,) },
                    ReplacePlaceholder {
                        path: &[
                            0,
                        ],
                        m: 1,
                    },
                    SetAttribute {
                        name: "id",
                        ns: None,
                        value: AttributeValue::Text("title-1".to_string()),
                        id: ElementId(
                            8,
                        ),
                    },
                    LoadTemplate {

                        index: 1,
                        id: ElementId(
                            11,
                        ),
                    },
                    CreateTextNode { value: "In a stunning turn of events, the world collectively unites and says hello back".to_string(), id: ElementId(12,) },
                    ReplacePlaceholder {
                        path: &[
                            0,
                        ],
                        m: 1,
                    },
                    SetAttribute {
                        name: "id",
                        ns: None,
                        value: AttributeValue::Text("body-1".to_string()),
                        id: ElementId(
                            11,
                        ),
                    },
                    LoadTemplate {
                        index: 2,
                        id: ElementId(
                            13,
                        ),
                    },
                    CreatePlaceholder { id: ElementId(14,) },
                    ReplacePlaceholder {
                        path: &[
                            0,
                        ],
                        m: 1,
                    },
                    SetAttribute {
                        name: "id",
                        ns: None,
                        value: AttributeValue::Text("children-1".to_string()),
                        id: ElementId(
                            13,
                        ),
                    },
                    ReplaceWith {
                        id: ElementId(
                            3,
                        ),
                        m: 3,
                    },

                    // Replace the second loading placeholder with a placeholder for us to fill in later
                    CreatePlaceholder {
                        id: ElementId(
                            3,
                        ),
                    },
                    ReplaceWith {
                        id: ElementId(
                            9,
                        ),
                        m: 1,
                    },
                    LoadTemplate {
                        index: 0,
                        id: ElementId(
                            9,
                        ),
                    },
                    CreateTextNode { value: "Goodbye Robot".to_string(), id: ElementId(15,) },
                    ReplacePlaceholder {
                        path: &[
                            0,
                        ],
                        m: 1,
                    },
                    SetAttribute {
                        name: "id",
                        ns: None,
                        value: AttributeValue::Text("title-2".to_string()),
                        id: ElementId(
                            9,
                        ),
                    },
                    LoadTemplate {
                        index: 1,
                        id: ElementId(
                            16,
                        ),
                    },
                    CreateTextNode { value: "The robot says goodbye".to_string(), id: ElementId(17,) },
                    ReplacePlaceholder {
                        path: &[
                            0,
                        ],
                        m: 1,
                    },
                    SetAttribute {
                        name: "id",
                        ns: None,
                        value: AttributeValue::Text("body-2".to_string()),
                        id: ElementId(
                            16,
                        ),
                    },
                    LoadTemplate {

                        index: 2,
                        id: ElementId(
                            18,
                        ),
                    },
                    // Create a placeholder for the resolved children
                    CreateTextNode { value: "Loading 3...".to_string(), id: ElementId(19,) },
                    ReplacePlaceholder { path: &[0,], m: 1 },
                    SetAttribute {
                        name: "id",
                        ns: None,
                        value: AttributeValue::Text("children-2".to_string()),
                        id: ElementId(
                            18,
                        ),
                    },

                    // Replace the loading placeholder with the resolved children
                    ReplaceWith {
                        id: ElementId(
                            3,
                        ),
                        m: 3,
                    },
                ]
            );

            dom.wait_for_work().await;
            let mutations = dom.render_immediate_to_vec();
            assert_eq!(
                mutations.edits,
                vec![
                    CreatePlaceholder {
                        id: ElementId(
                            3,
                        ),
                    },
                    ReplaceWith {
                        id: ElementId(
                            19,
                        ),
                        m: 1,
                    },
                    LoadTemplate {

                        index: 0,
                        id: ElementId(
                            19,
                        ),
                    },
                    CreateTextNode { value: "Goodbye Robot again".to_string(), id: ElementId(20,) },
                    ReplacePlaceholder {
                        path: &[
                            0,
                        ],
                        m: 1,
                    },
                    SetAttribute {
                        name: "id",
                        ns: None,
                        value: AttributeValue::Text("title-3".to_string()),
                        id: ElementId(
                            19,
                        ),
                    },
                    LoadTemplate {
                        index: 1,
                        id: ElementId(
                            21,
                        ),
                    },
                    CreateTextNode { value: "The robot says goodbye again".to_string(), id: ElementId(22,) },
                    ReplacePlaceholder {
                        path: &[
                            0,
                        ],
                        m: 1,
                    },
                    SetAttribute {
                        name: "id",
                        ns: None,
                        value: AttributeValue::Text("body-3".to_string()),
                        id: ElementId(
                            21,
                        ),
                    },
                    LoadTemplate {
                        index: 2,
                        id: ElementId(
                            23,
                        ),
                    },
                    CreatePlaceholder { id: ElementId(24,) },
                    ReplacePlaceholder {
                        path: &[
                            0
                        ],
                        m: 1,
                    },
                    SetAttribute {
                        name: "id",
                        ns: None,
                        value: AttributeValue::Text("children-3".to_string()),
                        id: ElementId(
                            23,
                        ),
                    },
                    ReplaceWith {
                        id: ElementId(
                            3,
                        ),
                        m: 3,
                    },
                ]
            )
        });
}

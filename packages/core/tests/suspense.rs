use dioxus::prelude::*;
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
fn suspense_resolves() {
    // wait just a moment, not enough time for the boundary to resolve
    tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap()
        .block_on(async {
            let mut dom = VirtualDom::new(app);
            dom.rebuild(&mut dioxus_core::NoOpMutations);
            dom.wait_for_suspense().await;
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
    tracing_subscriber::fmt::SubscriberBuilder::default()
        .with_max_level(tracing::Level::INFO)
        .init();

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

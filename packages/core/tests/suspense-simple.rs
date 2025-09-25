use anyhow::bail;
use dioxus::prelude::*;
use dioxus_core::{generation, AttributeValue, ElementId, Mutation};
use pretty_assertions::assert_eq;
use std::future::poll_fn;
use std::task::Poll;

#[tokio::test]
async fn suspense_holds_dom() {
    tracing_subscriber::fmt()
        .with_env_filter("info,dioxus_core=trace,dioxus=trace")
        .without_time()
        .init();

    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();

    // make sure the suspense is registered - the dom is suspended
    assert!(dom.suspended_tasks_remaining());

    // Wait for the first tier of suspense to resolve
    dom.wait_for_suspense().await;

    // Assert no more suspense - not always the case but true for this test
    assert!(!dom.suspended_tasks_remaining());

    // Render out the DOM now that it's no longer stuck and then print it
    dom.render_immediate(&mut dioxus_core::NoOpMutations);

    println!("{}", dioxus_ssr::render(&dom));
}

fn app() -> Element {
    let mut ready = use_signal(|| false);

    let suspense_err = use_hook(|| {
        suspend(spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            ready.set(true);
            println!("ready!");
        }))
    });

    debug!("suspense: {:?}", suspense_err);
    debug!("ready: {}", ready());

    if !ready() {
        return suspense_err;
    }

    rsx! {
        div { "Hello!" }
    }
}

#[tokio::test]
async fn error_while_suspense() {}

fn app2() -> Element {
    let mut ready = use_signal(|| false);

    use_hook(|| {
        suspend(spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            ready.set(true);
        }))
    })?;

    Err(anyhow::anyhow!("oh no").into())
}

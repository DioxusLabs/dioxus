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
    dom.wait_for_suspense().await;
    dom.render_immediate(&mut dioxus_core::NoOpMutations);

    let out = dioxus_ssr::render(&dom);
    println!("{}", out);
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

    println!("suspense: {:?}", suspense_err);
    println!("ready: {}", ready());

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

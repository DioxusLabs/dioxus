#![allow(non_snake_case)]

use anyhow::bail;
use dioxus::prelude::*;
use dioxus_core::{generation, AttributeValue, ElementId, Mutation, Mutations};
use pretty_assertions::assert_eq;
use std::future::poll_fn;
use std::task::Poll;

#[tokio::test]
async fn suspense_holds_dom() {
    init_logger();

    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();

    // make sure the suspense is registered - the dom is suspended
    assert!(dom.suspended_tasks_remaining());

    // Make sure the only thing in the dom is the initial placeholder
    assert_eq!("<!--placeholder0-->", dioxus_ssr::pre_render(&dom));

    // Wait for the first tier of suspense to resolve
    dom.wait_for_suspense(&mut dioxus_core::NoOpMutations).await;

    // Assert no more suspense - not always the case but true for this test
    assert!(!dom.suspended_tasks_remaining());

    // Render out the DOM now that it's no longer stuck and then print it
    dom.render_immediate(&mut dioxus_core::NoOpMutations);

    assert_eq!("<div>Hello!</div>", dioxus_ssr::render(&dom));

    fn app() -> Element {
        use_short_delay(10)?;
        rsx! { div { "Hello!" } }
    }
}

#[tokio::test]
async fn suspense_with_levels() {
    init_logger();

    fn app() -> Element {
        rsx! {
            h1 { "parent!" }
            SuspenseBoundary {
                fallback: |_| rsx! { "loading..." },
                Child {}
            }
            h2 { "after suspense" }
        }
    }

    fn Child() -> Element {
        rsx! {
            div { "Child parent" }
            SuspendedChild {}
        }
    }

    fn SuspendedChild() -> Element {
        use_short_delay(10)?;

        rsx! {
            div { "Child is ready!" }
        }
    }

    let mut dom = VirtualDom::new(app);
    let mutations = dom.rebuild_to_vec();
    debug!("mutations: {:#?}", mutations);
    use Mutation::*;
    assert_eq!(
        mutations.edits,
        vec![
            // Load the h1 parent element
            LoadTemplate { index: 0, id: ElementId(1) },
            // Create the `div { "Child parent" }` element
            LoadTemplate { index: 0, id: ElementId(2) },
            // Load a placeholder for the suspended component
            CreatePlaceholder { id: ElementId(3) },
            // Pop these off the stack, putting them into a fragment
            SaveNodes { n: 2 },
            // Create the loading fallback for the suspense
            LoadTemplate { index: 0, id: ElementId(4) },
            // Create the h2 after suspense element
            LoadTemplate { index: 2, id: ElementId(5) },
            // Append the nodes on the stack
            AppendChildren { id: ElementId(0), m: 3 },
        ]
    );

    debug!("Initial DOM:\n{}", dioxus_ssr::render(&dom));

    // make sure the suspense is registered - the dom is suspended
    assert!(dom.suspended_tasks_remaining());

    // Wait for the first tier of suspense to resolve
    let mut mutations = Mutations::default();
    dom.wait_for_suspense(&mut mutations).await;
    assert_eq!(
        mutations.edits,
        vec![
            // Load the resolved suspended child
            LoadTemplate { index: 0, id: ElementId(6) },
            // Replace the placeholder with the loaded content
            ReplaceWith { id: ElementId(3), m: 1 },
            // Load in the roots of the suspended content
            PushRoot { id: ElementId(2) },
            PushRoot { id: ElementId(6) },
            // And then swap the loading UI out
            ReplaceWith { id: ElementId(4), m: 2 }
        ]
    );

    // Assert no more suspense - not always the case but true for this test
    assert!(!dom.suspended_tasks_remaining());

    // Render out the DOM now that it's no longer stuck and then print it
    dom.render_immediate(&mut dioxus_core::NoOpMutations);

    println!("{}", dioxus_ssr::render(&dom));
}

// #[tokio::test]

fn init_logger() {
    tracing_subscriber::fmt()
        .with_env_filter("debug,dioxus_core=trace,dioxus=trace")
        .without_time()
        .init();
}

/// runs a short delay inside a suspense boundary, returning the suspense error if it is still pending
fn use_short_delay(delay_ms: u64) -> Result<(), RenderError> {
    let mut ready = use_signal(|| false);

    let suspense_err = use_hook(|| {
        suspend(spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
            ready.set(true);
        }))
    });

    if !ready() {
        debug!("Suspending because of suspense.");
        return suspense_err.map(|_| ());
    }

    info!("suspense: {:?}", suspense_err);
    info!("ready: {}", ready());

    Ok(())
}

#![allow(non_snake_case)]

use std::usize;

use dioxus::prelude::*;
use dioxus_core::{
    generation, AttributeValue, ElementId, Mutation, Mutations, NoOpMutations,
    SuspenseBoundaryProps,
};
use pretty_assertions::assert_eq;

/// If we suspend the first time, it should prevent the dom from writing anything useful out until it resolves.
#[tokio::test]
async fn suspense_holds_dom() {
    init_logger();

    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();

    // make sure the suspense is registered - the dom is suspended
    assert!(dom.suspended_tasks_remaining());

    // Also assert the virtualdom is right about being suspended
    assert!(dom.root_is_suspended());

    // Make sure the only thing in the dom is the initial placeholder
    assert_eq!("<!--placeholder0-->", dioxus_ssr::pre_render(&dom));

    // Wait for all the suspense to resolve
    dom.wait_for_suspense(&mut dioxus_core::NoOpMutations).await;

    // Assert no more suspense - not always the case but true for this test
    assert!(!dom.suspended_tasks_remaining());

    // Render out the DOM now that it's no longer stuck and then print it
    dom.render_immediate(&mut dioxus_core::NoOpMutations);

    assert_eq!("<div>Hello!</div>", dioxus_ssr::render(&dom));

    fn app() -> Element {
        use_delay(10)?;
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
        use_delay(10)?;

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

#[tokio::test]
async fn multiple_boundaries() {
    init_logger();

    fn app() -> Element {
        rsx! {
            h1 { "parent!" }
            SuspenseBoundary {
                fallback: |_| rsx! { "loading 1..." },
                DelayChild { delay_ms: 2 }
            }
            SuspenseBoundary {
                fallback: |_| rsx! { "loading 2..." },
                DelayChild { delay_ms: 5 }
            }
            SuspenseBoundary {
                fallback: |_| rsx! { "loading 3..." },
                DelayChild { delay_ms: 10 }
            }
            h3 { "after suspense" }
        }
    }

    #[component]
    fn DelayChild(delay_ms: u64) -> Element {
        use_delay(delay_ms)?;
        rsx! {
            div { "Child {delay_ms} is ready!" }
        }
    }

    let mut dom = VirtualDom::new(app);
    let mutations = dom.rebuild_to_vec();
    #[rustfmt::skip] {
        assert_eq!(
            mutations.edits,
            vec![
                // Load the h1 parent element
                Mutation::LoadTemplate { index: 0, id: ElementId(1) },

                // Create a placeholder for the first suspense boundary
                Mutation::CreatePlaceholder { id: ElementId(2) },
                // Save this placeholder
                Mutation::SaveNodes { n: 1 },
                // Create loading UI for the first suspense boundary
                Mutation::LoadTemplate { index: 0, id: ElementId(3) },

                // Create a placeholder for the second suspense boundary
                Mutation::CreatePlaceholder { id: ElementId(4) },
                // Save this placeholder
                Mutation::SaveNodes { n: 1 },
                // Create loading UI for the second suspense boundary
                Mutation::LoadTemplate { index: 0, id: ElementId(5) },

                // Create a placeholder for the third suspense boundary
                Mutation::CreatePlaceholder { id: ElementId(6) },
                // Save this placeholder
                Mutation::SaveNodes { n: 1 },
                // Create loading UI for the third suspense boundary
                Mutation::LoadTemplate { index: 0, id: ElementId(7) },

                // Create the h3 after suspense element
                Mutation::LoadTemplate { index: 4, id: ElementId(8) },

                // Append the nodes on the stack
                Mutation::AppendChildren { id: ElementId(0), m: 5 },
            ]
        )
    };
}

/// Test that we can wait for the first tier of suspense to resolve while the inner suspense is still pending
///
/// This is useful for SSR or native renderers that want to prevent showing the window or committing the
/// response until there's something to show.
#[tokio::test]
async fn suspense_wait_for_first_commit() {
    init_logger();

    fn app() -> Element {
        use_delay(10)?;

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
        use_delay(100)?;

        rsx! {
            div { "Child is ready!" }
        }
    }

    // Make sure it renders correctly after waiting for all suspense to resolve
    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();
    dom.wait_for_suspense(&mut NoOpMutations).await;
    assert_eq!(
        "<h1>parent!</h1><div>Child parent</div><div>Child is ready!</div><h2>after suspense</h2>",
        dioxus_ssr::render(&dom)
    );

    // Wait individually
    let mut dom = VirtualDom::new(app);
    let mutations = dom.rebuild_to_vec();
    assert_eq!(
        mutations.edits,
        vec![
            // Creating the placeholder node and nothing else.
            Mutation::CreatePlaceholder { id: ElementId(1) },
            Mutation::SaveNodes { n: 1 },
            // And then the loading UI for the suspense boundary, which happens to be just a placeholder too..
            Mutation::CreatePlaceholder { id: ElementId(2) },
            // Add to document
            Mutation::AppendChildren { id: ElementId(0), m: 1 }
        ]
    );

    // make sure the suspense is registered - the dom is suspended
    assert!(dom.suspended_tasks_remaining());

    // Also assert the virtualdom is right about being suspended
    assert!(dom.root_is_suspended());

    // Make sure the only thing in the dom is the initial placeholder
    assert_eq!("<!--placeholder0-->", dioxus_ssr::pre_render(&dom));

    // Wait for all the suspense to resolve
    let mut mutations = Mutations::default();
    dom.wait_for_root_suspense(&mut mutations).await;

    #[rustfmt::skip]
    {assert_eq!(
        mutations.edits,
        vec![
            // Create the "parent!" h1"
            Mutation::LoadTemplate { index: 0, id: ElementId(3) },

            // Create the `div { "Child parent" }` element
            Mutation::LoadTemplate { index: 0, id: ElementId(4) },

            // Create a placeholder for the first suspense boundary
            Mutation::CreatePlaceholder { id: ElementId(5) },
            // Save this placeholder
            Mutation::SaveNodes { n: 2 },

            // Create loading UI for the first suspense boundary
            Mutation::LoadTemplate { index: 0, id: ElementId(6) },

            // Create the h2 after suspense element
            Mutation::LoadTemplate { index: 2, id: ElementId(7) },

            // Replace the first suspense placeholder with the resolved tier 1 suspense content
            Mutation::ReplaceWith { id: ElementId(1), m: 3 },

            // And then replace the root placeholder with the suspense resolved content
            Mutation::PushRoot { id: ElementId(3) }, /* parent! h1 */
            Mutation::PushRoot { id: ElementId(4) }, /* child parent div */
            Mutation::PushRoot { id: ElementId(usize::MAX - 1) }, /* suspense fragment */
            // Mutation::PushRoot { id: ElementId(6) }, /* suspense fragment */
            Mutation::PushRoot { id: ElementId(7) }, /* h2 after suspense */
            Mutation::ReplaceWith { id: ElementId(2), m: 4 },
        ]
    )};

    assert_eq!(
        "<h1>parent!</h1>loading<h2>after suspense</h2>",
        dioxus_ssr::render(&dom)
    );

    // Assert no more suspense - not always the case but true for this test
    assert!(dom.suspended_tasks_remaining());

    // Render out the DOM now that it's no longer stuck and then print it
    dom.render_immediate(&mut dioxus_core::NoOpMutations);

    assert_eq!("<div>Hello!</div>", dioxus_ssr::render(&dom));
}

/// Test that a suspense boundary can go from resolved to suspended again if its internal state changes
#[tokio::test]
async fn suspense_moves_from_okay_to_suspended() {}

fn init_logger() {
    _ = tracing_subscriber::fmt()
        .with_env_filter("debug,dioxus_core=trace,dioxus=trace")
        .without_time()
        .try_init();
}

/// runs a short delay inside a suspense boundary, returning the suspense error if it is still pending
fn use_delay(delay_ms: u64) -> Result<(), RenderError> {
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

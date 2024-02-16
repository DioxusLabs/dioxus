use std::rc::Rc;
use dioxus::dioxus_core::Mutation::*;
use dioxus::prelude::*;
use dioxus_core::ElementId;
use tracing_fluent_assertions::{AssertionsLayer, AssertionRegistry};
use tracing_subscriber::{layer::SubscriberExt, Registry};
use dioxus::html::SerializedHtmlEventConverter;


#[test]
fn miri_rollover() {
    // setup tracing
    let assertion_registry = AssertionRegistry::default();
    let base_subscriber = Registry::default();
    let std_out_log = tracing_subscriber::fmt::layer().pretty();
    let subscriber = base_subscriber
        .with(std_out_log)
        .with(AssertionsLayer::new(&assertion_registry));
    tracing::subscriber::set_global_default(subscriber).unwrap();

    let new_virtual_dom = assertion_registry.build()
        .with_name("VirtualDom::new")
        .was_created()
        .was_entered_exactly(1)
        .was_closed()
        .finalize();

    let edited_virtual_dom = assertion_registry.build()
        .with_name("VirtualDom::rebuild")
        .was_created()
        .was_entered_exactly(1)
        .was_closed()
        .finalize();

    set_event_converter(Box::new(SerializedHtmlEventConverter));
    let mut dom = VirtualDom::new(app);

    dom.rebuild(&mut dioxus_core::NoOpMutations);

    new_virtual_dom.assert();
    edited_virtual_dom.assert();

    for _ in 0..3 {
        dom.handle_event(
            "click",
            Rc::new(PlatformEventData::new(Box::<SerializedMouseData>::default())),
            ElementId(2),
            true,
        );
        dom.process_events();
        _ = dom.render_immediate_to_vec();
    }
}

fn app() -> Element {
    let mut idx = use_signal(|| 0);
    let onhover = |_| println!("go!");

    rsx! {
        div {
            button {
                onclick: move |_| {
                    idx += 1;
                    println!("Clicked");
                },
                "+"
            }
            button { onclick: move |_| idx -= 1, "-" }
            ul {
                {(0..idx()).map(|i| rsx! {
                    ChildExample { i: i, onhover: onhover }
                })}
            }
        }
    }
}

#[component]
fn ChildExample(i: i32, onhover: EventHandler<MouseEvent>) -> Element {
    rsx! { li { onmouseover: move |e| onhover.call(e), "{i}" } }
}






#[test]
fn test_original_diff() {
    // Create the assertion registry and install the assertion layer,
    // then install that subscriber as the global default.
    let assertion_registry = AssertionRegistry::default();
    let base_subscriber = Registry::default();
    // create a std out logger just to help with testing
    let std_out_log = tracing_subscriber::fmt::layer().pretty();
    let subscriber = base_subscriber
        .with(std_out_log)
        .with(AssertionsLayer::new(&assertion_registry));
    tracing::subscriber::set_global_default(subscriber).unwrap();

    let new_virtual_dom = assertion_registry.build()
        .with_name("VirtualDom::new")
        .was_created()
        .was_entered_exactly(1)
        .was_exited()
        .was_closed()
        .finalize();

    let edited_virtual_dom = assertion_registry.build()
        .with_name("VirtualDom::rebuild")
        .was_created()
        .was_entered_exactly(1)
        .was_exited()
        .was_closed()
        .finalize();

    let mut dom = VirtualDom::new(|| {
        rsx! {
            div { div { "Hello, world!" } }
        }
    });


    let edits = dom.rebuild_to_vec().santize();

    new_virtual_dom.assert();
    edited_virtual_dom.assert();

    assert_eq!(
        edits.edits,
        [
            // add to root
            LoadTemplate { name: "template", index: 0, id: ElementId(1) },
            AppendChildren { m: 1, id: ElementId(0) }
        ]
    )
}
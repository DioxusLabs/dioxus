use dioxus::html::SerializedHtmlEventConverter;
use dioxus::prelude::*;
use dioxus_core::ElementId;
use std::rc::Rc;
use tracing_fluent_assertions::{AssertionRegistry, AssertionsLayer};
use tracing_subscriber::{layer::SubscriberExt, Registry};

#[test]
fn basic_tracing() {
    // setup tracing
    let assertion_registry = AssertionRegistry::default();
    let base_subscriber = Registry::default();
    // log to standard out for testing
    let std_out_log = tracing_subscriber::fmt::layer().pretty();
    let subscriber = base_subscriber
        .with(std_out_log)
        .with(AssertionsLayer::new(&assertion_registry));
    tracing::subscriber::set_global_default(subscriber).unwrap();

    let new_virtual_dom = assertion_registry
        .build()
        .with_name("VirtualDom::new")
        .was_created()
        .was_entered_exactly(1)
        .was_closed()
        .finalize();

    let edited_virtual_dom = assertion_registry
        .build()
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

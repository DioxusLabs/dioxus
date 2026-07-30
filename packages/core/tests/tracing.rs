use dioxus::html::SerializedHtmlEventConverter;
use dioxus::prelude::*;
use dioxus_core::Event;
use dioxus_renderer_oracle::RendererOracle;
use std::{any::Any, rc::Rc};
use tracing_fluent_assertions::{AssertionRegistry, AssertionsLayer};
use tracing_subscriber::{Registry, layer::SubscriberExt};

// This test asserts on tracing events emitted by `VirtualDom::new` and
// `VirtualDom::rebuild`; it requires those calls to happen *exactly once*.
#[test]
fn basic_tracing() {
    let assertion_registry = AssertionRegistry::default();
    let base_subscriber = Registry::default();
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
    let mut oracle = RendererOracle::new();
    oracle.rebuild(&mut dom);

    new_virtual_dom.assert();
    edited_virtual_dom.assert();

    for _ in 0..3 {
        let event = Event::new(
            Rc::new(PlatformEventData::new(Box::<SerializedMouseData>::default())) as Rc<dyn Any>,
            true,
        );
        let target = oracle.element_id_by_attr("id", "increment");
        dom.runtime().handle_event("click", event, target);
        dom.process_events();
        oracle.render(&mut dom);
    }
}

fn app() -> Element {
    let mut idx = use_signal(|| 0);
    let onhover = |_| println!("go!");

    rsx! {
        div {
            button {
                id: "increment",
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

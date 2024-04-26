use dioxus::prelude::*;
use dioxus_core::ElementId;
use std::rc::Rc;

#[test]
fn values_memoize_in_place() {
    thread_local! {
        static DROP_COUNT: std::cell::RefCell<usize> = const { std::cell::RefCell::new(0) };
    }

    struct CountsDrop;

    impl Drop for CountsDrop {
        fn drop(&mut self) {
            DROP_COUNT.with(|c| *c.borrow_mut() += 1);
        }
    }

    fn app() -> Element {
        let mut count = use_signal(|| 0);
        let x = CountsDrop;

        if generation() < 15 {
            count += 1;
        }

        rsx! {
            TakesEventHandler {
                click: move |num| {
                    // Force the closure to own the drop counter
                    let _ = &x;
                    println!("num is {num}");
                },
                children: count() / 2
            }
            TakesSignal { sig: count(), children: count() / 2 }
        }
    }

    set_event_converter(Box::new(dioxus::html::SerializedHtmlEventConverter));
    let mut dom = VirtualDom::new(app);

    let mutations = dom.rebuild_to_vec();
    println!("{:#?}", mutations);
    dom.mark_dirty(ScopeId::ROOT);
    for _ in 0..20 {
        dom.handle_event(
            "click",
            Rc::new(PlatformEventData::new(Box::<SerializedMouseData>::default())),
            ElementId(1),
            true,
        );
        dom.render_immediate(&mut dioxus_core::NoOpMutations);
    }
    dom.render_immediate(&mut dioxus_core::NoOpMutations);
    // As we rerun the app, the drop count should be 15 one for each render of the app component
    let drop_count = DROP_COUNT.with(|c| *c.borrow());
    assert_eq!(drop_count, 15);
}

// We move over event handlers in place. Make sure we do that in a way that doesn't destroy the original event handler
#[test]
fn cloning_event_handler_components_work() {
    fn app() -> Element {
        let rsx_with_event_handler_component = rsx! {
            TakesEventHandler {
                click: move |evt| {
                    println!("Clicked {evt:?}!");
                }
            }
        };

        rsx! {
            {rsx_with_event_handler_component.clone()}
            {rsx_with_event_handler_component.clone()}
            {rsx_with_event_handler_component.clone()}
            {rsx_with_event_handler_component}
        }
    }

    set_event_converter(Box::new(dioxus::html::SerializedHtmlEventConverter));
    let mut dom = VirtualDom::new(app);

    let mutations = dom.rebuild_to_vec();
    println!("{:#?}", mutations);
    dom.mark_dirty(ScopeId::ROOT);
    for _ in 0..20 {
        dom.handle_event(
            "click",
            Rc::new(PlatformEventData::new(Box::<SerializedMouseData>::default())),
            ElementId(1),
            true,
        );
        dom.render_immediate(&mut dioxus_core::NoOpMutations);
    }
    dom.render_immediate(&mut dioxus_core::NoOpMutations);
}

#[component]
fn TakesEventHandler(click: EventHandler<usize>, children: usize) -> Element {
    let first_render_click = use_hook(move || click);
    if generation() > 0 {
        // Make sure the event handler is memoized in place and never gets dropped
        first_render_click(children);
    }

    rsx! {
        button {
            onclick: move |_| click(children),
            "{children}"
        }
    }
}

#[component]
fn TakesSignal(sig: ReadOnlySignal<usize>, children: usize) -> Element {
    let first_render_sig = use_hook(move || sig);
    if generation() > 0 {
        // Make sure the signal is memoized in place and never gets dropped
        println!("{first_render_sig}");
    }

    rsx! {
        button { "{children}" }
    }
}

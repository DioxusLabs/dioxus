use dioxus::prelude::*;
use dioxus_core::ElementId;
use std::rc::Rc;

#[tokio::test]
async fn values_memoize_in_place() {
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

        use_hook(|| {
            spawn(async move {
                for _ in 0..15 {
                    tokio::time::sleep(std::time::Duration::from_millis(1)).await;
                    count += 1;
                }
            });
        });

        rsx! {
            TakesEventHandler {
                click: move |num| {
                    // Force the closure to own the drop counter
                    let _ = &x;
                    println!("num is {num}");
                },
                number: count() / 2
            }
            TakesSignal { sig: count(), number: count() / 2 }
        }
    }

    set_event_converter(Box::new(dioxus::html::SerializedHtmlEventConverter));
    let mut dom = VirtualDom::new(app);

    let mutations = dom.rebuild_to_vec();
    println!("{:#?}", mutations);
    dom.mark_dirty(ScopeId::APP);
    for _ in 0..40 {
        dom.handle_event(
            "click",
            Rc::new(PlatformEventData::new(Box::<SerializedMouseData>::default())),
            ElementId(1),
            true,
        );
        tokio::select! {
            _ = tokio::time::sleep(std::time::Duration::from_millis(20)) => {},
            _ = dom.wait_for_work() => {}
        }
        dom.render_immediate(&mut dioxus_core::NoOpMutations);
    }
    dom.render_immediate(&mut dioxus_core::NoOpMutations);
    // As we rerun the app, the drop count should be 15 one for each render of the app component
    let drop_count = DROP_COUNT.with(|c| *c.borrow());
    assert_eq!(drop_count, 16);
}

// We move over event handlers in place. Make sure we do that in a way that doesn't destroy the original event handler
#[test]
fn cloning_event_handler_components_work() {
    fn app() -> Element {
        let rsx_with_event_handler_component = rsx! {
            TakesEventHandler {
                click: move |evt| {
                    println!("Clicked {evt:?}!");
                },
                number: 0
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
    dom.mark_dirty(ScopeId::APP);
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
fn TakesEventHandler(click: EventHandler<usize>, number: usize) -> Element {
    let first_render_click = use_hook(move || click);
    if generation() > 0 {
        // Make sure the event handler is memoized in place and never gets dropped
        first_render_click(number);
    }

    rsx! {
        button {
            onclick: move |_| click(number),
            "{number}"
        }
    }
}

#[component]
fn TakesSignal(sig: ReadOnlySignal<usize>, number: usize) -> Element {
    let first_render_sig = use_hook(move || sig);
    if generation() > 0 {
        // Make sure the signal is memoized in place and never gets dropped
        println!("{first_render_sig}");
    }

    rsx! {
        button { "{number}" }
    }
}

// Regression test for https://github.com/DioxusLabs/dioxus/issues/2582
#[test]
fn spreads_memorize_in_place() {
    #[derive(Props, Clone, PartialEq)]
    struct CompProps {
        #[props(extends = GlobalAttributes)]
        attributes: Vec<Attribute>,
    }

    let mut props = CompProps::builder().build();
    assert!(!props.memoize(&CompProps::builder().all("123").build()));
    assert_eq!(
        props.attributes,
        vec![Attribute::new("all", "123", Some("style"), false)]
    );

    assert!(!props.memoize(&CompProps::builder().width("123").build()));
    assert_eq!(
        props.attributes,
        vec![Attribute::new("width", "123", Some("style"), false)]
    );

    assert!(!props.memoize(&CompProps::builder().build()));
    assert_eq!(props.attributes, vec![]);

    assert!(props.memoize(&CompProps::builder().build()));
    assert_eq!(props.attributes, vec![]);
}

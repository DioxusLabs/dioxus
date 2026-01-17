use dioxus::{
    core::{generation, needs_update},
    prelude::*,
};
use dioxus_core::ElementId;
use std::{any::Any, rc::Rc};

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
        let event = Event::new(
            Rc::new(PlatformEventData::new(Box::<SerializedMouseData>::default())) as Rc<dyn Any>,
            true,
        );
        dom.runtime().handle_event("click", event, ElementId(1));
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
        let event = Event::new(
            Rc::new(PlatformEventData::new(Box::<SerializedMouseData>::default())) as Rc<dyn Any>,
            true,
        );
        dom.runtime().handle_event("click", event, ElementId(1));
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
fn TakesSignal(sig: ReadSignal<usize>, number: usize) -> Element {
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
    use dioxus_core::Properties;

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

// Regression test for https://github.com/DioxusLabs/dioxus/issues/2331
#[test]
fn cloning_read_signal_components_work() {
    fn app() -> Element {
        if generation() < 5 {
            println!("Generating new props");
            needs_update();
        }

        let read_signal_rsx = rsx! {
            TakesReadSignalNonClone { sig: NonCloneable(generation() as i32) }
            TakesReadSignalNum { sig: generation() as i32 }
        };

        rsx! {
            {read_signal_rsx.clone()}
            {read_signal_rsx}
        }
    }

    struct NonCloneable<T>(T);

    #[component]
    fn TakesReadSignalNum(sig: ReadSignal<i32>) -> Element {
        rsx! {}
    }

    #[component]
    fn TakesReadSignalNonClone(sig: ReadSignal<NonCloneable<i32>>) -> Element {
        rsx! {}
    }

    set_event_converter(Box::new(dioxus::html::SerializedHtmlEventConverter));
    let mut dom = VirtualDom::new(app);

    let mutations = dom.rebuild_to_vec();
    println!("{:#?}", mutations);
    dom.mark_dirty(ScopeId::APP);
    for _ in 0..20 {
        let event = Event::new(
            Rc::new(PlatformEventData::new(Box::<SerializedMouseData>::default())) as Rc<dyn Any>,
            true,
        );
        dom.runtime().handle_event("click", event, ElementId(1));
        dom.render_immediate(&mut dioxus_core::NoOpMutations);
    }
    dom.render_immediate(&mut dioxus_core::NoOpMutations);
}

// Regression test for https://github.com/DioxusLabs/dioxus/issues/5222
#[tokio::test]
async fn optional_event_handler_diff() {
    use dioxus_core::Properties;

    #[derive(Props, Clone, PartialEq)]
    struct CompProps {
        callback: Option<Callback>,
    }

    let dom = VirtualDom::new(|| rsx! {});

    dom.in_scope(ScopeId::APP, || {
        // Diffing from None to Some should be different and copy the callback
        let mut props = CompProps::builder().callback(None).build();
        assert!(!props.memoize(&CompProps::builder().callback(|_| {}).build()));
        assert!(props.inner.callback.is_some());

        // Diffing from Some to None should be different and remove the callback
        let mut props = CompProps::builder().callback(|_| {}).build();
        assert!(!props.memoize(&CompProps::builder().callback(None).build()));
        assert!(props.inner.callback.is_none());
    });
}

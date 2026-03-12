use dioxus::prelude::*;
use dioxus_core::Mutation::*;
use dioxus_core::*;
use std::any::Any;

#[derive(Store, std::fmt::Debug, Clone, Default)]
struct X {
    inner1: i32,
    inner2: i32,
}

fn dummy_event() -> Event<dyn Any> {
    use dioxus_core::*;
    use std::rc::Rc;
    Event::new(
        Rc::new(PlatformEventData::new(Box::<SerializedMouseData>::default())) as Rc<dyn Any>,
        true,
    )
}

#[test]
fn children_see_parent_write() {
    set_event_converter(Box::new(dioxus::html::SerializedHtmlEventConverter));

    fn app() -> Element {
        let mut x = use_store::<X>(X::default);
        let inner1 = x.inner1();
        rsx! {
            div {
                onclick: move |_| x.set(X { inner1: 1, inner2: 0}),
            }
            "x = {inner1}"
        }
    }

    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();

    dom.runtime()
        .handle_event("click", dummy_event(), ElementId(1));

    use dioxus_core::Mutation::*;
    let edits = dom.render_immediate_to_vec();
    assert_eq!(
        edits.edits,
        [SetText {
            value: "x = 1".into(),
            id: ElementId(2)
        }]
    );
}

// https://github.com/DioxusLabs/dioxus/issues/5363
// This also tests that an unwritten sibling does NOT see the change,
// and that the writes notify effects
#[tokio::test]
async fn parents_see_child_write() {
    set_event_converter(Box::new(dioxus::html::SerializedHtmlEventConverter));

    #[component]
    fn child_reader(label: &'static str, value: ReadSignal<i32>) -> Element {
        rsx! {
            "{label} = {value()}"
        }
    }
    use std::sync::atomic::*;
    static EFFECT_COUNT: AtomicUsize = AtomicUsize::new(0);

    fn app() -> Element {
        let x = use_store::<X>(|| X {
            inner1: 0,
            inner2: 0,
        });

        use_effect(move || {
            let _ = x.read();
            println!("i am effect");
            EFFECT_COUNT.fetch_add(1, Ordering::SeqCst);
            dioxus_core::needs_update();
        });

        rsx! {
            div {
                onclick: move |_| x.inner1().set(1),
            }

            child_reader { label: "inner1", value: x.inner1() }
            child_reader { label: "inner2", value: x.inner2() }
            "x = {x:?}"
        }
    }

    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();
    tokio::time::timeout(std::time::Duration::from_millis(50), dom.wait_for_work())
        .await
        .unwrap();

    assert_eq!(EFFECT_COUNT.load(Ordering::SeqCst), 1);

    dom.runtime()
        .handle_event("click", dummy_event(), ElementId(1));

    let edits = dom.render_immediate_to_vec();
    tokio::time::timeout(std::time::Duration::from_millis(50), dom.wait_for_work())
        .await
        .unwrap();

    assert_eq!(EFFECT_COUNT.load(Ordering::SeqCst), 2);
    assert_eq!(
        edits.edits,
        [
            SetText {
                value: "x = X { inner1: 1, inner2: 0 }".into(),
                id: ElementId(4)
            },
            SetText {
                value: "inner1 = 1".into(),
                id: ElementId(2)
            }
        ]
    );
}

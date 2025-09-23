#![allow(unused, non_upper_case_globals, non_snake_case)]

use std::sync::mpsc::{sync_channel, SyncSender};
use dioxus::prelude::*;
use dioxus_core::{generation, ElementId, NoOpMutations};
use dioxus_signals::*;

#[test]
fn create_signals_global() {
    let mut dom = VirtualDom::new(|| {
        rsx! {
            for _ in 0..10 {
                Child {}
            }
        }
    });

    fn Child() -> Element {
        let signal = create_without_cx();

        rsx! {
            "{signal}"
        }
    }

    dom.rebuild_in_place();

    fn create_without_cx() -> Signal<String> {
        Signal::new("hello world".to_string())
    }
}

#[test]
fn deref_signal() {
    #[derive(Clone, Props)]
    struct ChildProps {
        tx: SyncSender<bool>,
    }

    impl PartialEq for ChildProps {
        fn eq(&self, _: &Self) -> bool {
            false
        }

        fn ne(&self, _: &Self) -> bool {
            true
        }
    }

    fn Child(props: ChildProps) -> Element {
        let signal = Signal::new("hello world".to_string());

        // You can call signals like functions to get a Ref of their value.
        let result = &*signal();
        let _ = props.tx.send(result.eq("hello world"));

        rsx! {
            "arbitrary text"
        }
    }

    let (tx, rx) = sync_channel::<bool>(1);
    let props = ChildProps { tx };
    let mut dom = VirtualDom::new_with_props(Child, props);

    dom.rebuild_in_place();

    let result = rx.recv();
    assert!(matches!(result, Ok(true)));
}

#[test]
fn drop_signals() {
    use std::sync::atomic::AtomicUsize;
    use std::sync::atomic::Ordering;

    static SIGNAL_DROP_COUNT: AtomicUsize = AtomicUsize::new(0);

    let mut dom = VirtualDom::new(|| {
        let generation = generation();

        let count = if generation % 2 == 0 { 10 } else { 0 };
        rsx! {
            for _ in 0..count {
                Child {}
            }
        }
    });

    fn Child() -> Element {
        struct TracksDrops;

        impl Drop for TracksDrops {
            fn drop(&mut self) {
                SIGNAL_DROP_COUNT.fetch_add(1, Ordering::Relaxed);
            }
        }

        use_signal(|| TracksDrops);

        rsx! {
            ""
        }
    }

    dom.rebuild_in_place();
    dom.mark_dirty(ScopeId::APP);
    dom.render_immediate(&mut NoOpMutations);

    assert_eq!(SIGNAL_DROP_COUNT.load(Ordering::Relaxed), 10);
}

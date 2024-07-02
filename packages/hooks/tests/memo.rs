#[tokio::test]
async fn memo_updates() {
    use std::cell::RefCell;

    use dioxus::prelude::*;

    thread_local! {
        static VEC_SIGNAL: RefCell<Option<Signal<Vec<usize>, SyncStorage>>> = const { RefCell::new(None) };
    }

    fn app() -> Element {
        let mut vec = use_signal_sync(|| vec![0, 1, 2]);

        // Signals should update if they are changed from another thread
        use_hook(|| {
            VEC_SIGNAL.with(|cell| {
                *cell.borrow_mut() = Some(vec);
            });
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_millis(100));
                vec.push(5);
            });
        });

        let len = vec.len();
        let len_memo = use_memo(move || vec.len());

        // Make sure memos that update in the middle of a component work
        if generation() < 2 {
            vec.push(len);
        }

        // The memo should always be up to date
        assert_eq!(vec.len(), len_memo());

        rsx! {
            for i in 0..len {
                Child { index: i, vec }
            }
        }
    }

    #[component]
    fn Child(index: usize, vec: Signal<Vec<usize>, SyncStorage>) -> Element {
        // This memo should not rerun after the element is removed
        let item = use_memo(move || vec.read()[index]);

        rsx! {
            div { "Item: {item}" }
        }
    }

    let race = async move {
        let mut dom = VirtualDom::new(app);

        dom.rebuild_in_place();
        let mut signal = VEC_SIGNAL.with(|cell| (*cell.borrow()).unwrap());
        // Wait for the signal to update
        for _ in 0..2 {
            dom.wait_for_work().await;
            dom.render_immediate(&mut dioxus::dioxus_core::NoOpMutations);
        }
        assert_eq!(signal(), vec![0, 1, 2, 3, 4, 5]);
        // Remove each element from the vec
        for _ in 0..6 {
            signal.pop();
            dom.wait_for_work().await;
            dom.render_immediate(&mut dioxus::dioxus_core::NoOpMutations);
            println!("Signal: {signal:?}");
        }
    };

    tokio::select! {
        _ = race => {},
        _ = tokio::time::sleep(std::time::Duration::from_millis(1000)) => panic!("timed out")
    };
}

#[tokio::test]
async fn use_memo_only_triggers_one_update() {
    use dioxus::prelude::*;
    use std::cell::RefCell;

    thread_local! {
        static VEC_SIGNAL: RefCell<Vec<usize>> = const { RefCell::new(Vec::new()) };
    }

    fn app() -> Element {
        let mut count = use_signal(|| 0);

        let memorized = use_memo(move || dbg!(count() * 2));

        use_memo(move || {
            println!("reading doubled");
            let doubled = memorized();
            VEC_SIGNAL.with_borrow_mut(|v| v.push(doubled))
        });

        // Writing to count many times in a row should not cause the memo to update other subscribers multiple times
        use_hook(move || {
            for _ in 0..10 {
                count += 1;
                // Reading the memo each time will trigger the memo to rerun immediately, but the VEC_SIGNAL should still only rerun once
                println!("doubled {memorized}");
            }
        });

        rsx! {}
    }

    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();

    tokio::select! {
        _ = dom.wait_for_work() => {},
        _ = tokio::time::sleep(std::time::Duration::from_millis(100)) => {}
    };

    dom.render_immediate(&mut dioxus::dioxus_core::NoOpMutations);

    assert_eq!(VEC_SIGNAL.with(|v| v.borrow().clone()), vec![0, 20]);
}

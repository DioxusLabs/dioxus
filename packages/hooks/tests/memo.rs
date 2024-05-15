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
}

#[tokio::test]
async fn use_memo_with_use_effect() {
    use dioxus::prelude::*;
    use futures_util::future::select;
    use std::{cell::RefCell, collections::VecDeque, pin::pin, rc::Rc};
    use tokio::sync::mpsc;

    let (action_sender, mut action_receiver) = mpsc::unbounded_channel::<Action>();

    #[derive(Debug, Copy, Clone, PartialEq, Eq)]
    enum Action {
        Write(Option<usize>, Option<usize>),
        Read(usize, usize),
    }

    #[derive(Clone)]
    struct ActionSender(Rc<RefCell<mpsc::UnboundedSender<Action>>>);

    #[component]
    fn App() -> Element {
        let action_sender = use_context::<ActionSender>();
        let mut a = use_signal(|| 0);
        let mut b = use_signal(|| 0);
        let mut counter = use_signal(|| 0);

        use_effect({
            let action_sender = action_sender.clone();
            move || {
                let count = counter();
                let action = match count {
                    0 => Action::Write(Some(0), None),
                    1 => Action::Write(None, Some(0)),
                    2 => Action::Write(Some(0), Some(0)),
                    3 => Action::Write(Some(1), None),
                    4 => Action::Write(None, Some(1)),
                    5 => Action::Write(Some(1), Some(1)),
                    6 => Action::Write(Some(2), Some(2)),
                    7 => Action::Write(Some(0), Some(0)),
                    _ => return,
                };
                action_sender.0.borrow_mut().send(action).unwrap();
                if let Action::Write(Some(n), _) = action {
                    *a.write() = n;
                }
                if let Action::Write(_, Some(n)) = action {
                    *b.write() = n;
                }
                *counter.write() = count + 1;
            }
        });

        let tuple = use_memo(move || (a(), b()));

        use_effect(move || {
            let (a, b) = tuple();
            action_sender
                .0
                .borrow_mut()
                .send(Action::Read(a, b))
                .unwrap();
        });

        None
    }

    let mut dom =
        VirtualDom::new(App).with_root_context(ActionSender(Rc::new(RefCell::new(action_sender))));
    dom.rebuild_in_place();

    let update_dom = async {
        loop {
            dom.wait_for_work().await;
            dom.render_immediate(&mut dioxus::dioxus_core::NoOpMutations);
        }
    };

    let mut reads = VecDeque::new();
    let mut writes = VecDeque::new();
    let recv_actions = async {
        while let Some(action) = action_receiver.recv().await {
            println!("{action:?}");
            match action {
                read @ Action::Read(_, _) => reads.push_back(read),
                write @ Action::Write(_, _) => writes.push_back(write),
            }
            if reads.len() == 5 && writes.len() == 8 {
                break;
            }
        }
    };

    select(pin!(update_dom), pin!(recv_actions)).await;

    assert_eq!(reads.pop_front(), Some(Action::Read(0, 0)));
    assert_eq!(writes.pop_front(), Some(Action::Write(Some(0), None)));
    assert_eq!(writes.pop_front(), Some(Action::Write(None, Some(0))));
    assert_eq!(writes.pop_front(), Some(Action::Write(Some(0), Some(0))));
    assert_eq!(writes.pop_front(), Some(Action::Write(Some(1), None)));
    assert_eq!(reads.pop_front(), Some(Action::Read(1, 0)));
    assert_eq!(writes.pop_front(), Some(Action::Write(None, Some(1))));
    assert_eq!(reads.pop_front(), Some(Action::Read(1, 1)));
    assert_eq!(writes.pop_front(), Some(Action::Write(Some(1), Some(1))));
    assert_eq!(writes.pop_front(), Some(Action::Write(Some(2), Some(2))));
    assert_eq!(reads.pop_front(), Some(Action::Read(2, 2)));
    assert_eq!(writes.pop_front(), Some(Action::Write(Some(0), Some(0))));
    assert_eq!(reads.pop_front(), Some(Action::Read(0, 0)));
}

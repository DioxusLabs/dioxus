#![allow(non_snake_case)]
use dioxus::prelude::dioxus_core::NoOpMutations;
use dioxus::prelude::*;
use sysinfo::{ProcessRefreshKind, RefreshKind, System};

// Regression test for https://github.com/DioxusLabs/dioxus/issues/3421
#[tokio::test]
async fn test_for_memory_leaks() {
    fn app() -> Element {
        let mut count = use_signal(|| 0);

        use_hook(|| {
            spawn(async move {
                loop {
                    tokio::time::sleep(std::time::Duration::from_nanos(1)).await;
                    let val = *count.peek_unchecked();
                    if val == 70 {
                        count.set(0);
                    } else {
                        count.set(val + 1);
                    }
                }
            })
        });

        rsx! {
            for el in 0..*count.read() {
                div {
                    key: "{el}",
                    div {
                        onclick: move |_| { println!("click"); },
                    }
                    AcceptsEventHandlerAndReadOnlySignal {
                        event_handler: move |_| { println!("click"); },
                        signal: el,
                    }
                }
            }
        }
    }

    // Event handlers and ReadOnlySignals have extra logic on component boundaries that has caused memory leaks
    // in the past
    #[component]
    fn AcceptsEventHandlerAndReadOnlySignal(
        event_handler: EventHandler<MouseEvent>,
        signal: ReadOnlySignal<i32>,
    ) -> Element {
        rsx! {
            div {
                onclick: event_handler,
                "{signal}"
            }
        }
    }

    // create the vdom, the real_dom, and the binding layer between them
    let mut vdom = VirtualDom::new(app);

    vdom.rebuild(&mut NoOpMutations);

    let pid = sysinfo::get_current_pid().expect("failed to get PID");

    let refresh =
        RefreshKind::nothing().with_processes(ProcessRefreshKind::nothing().with_memory());
    let mut system = System::new_with_specifics(refresh);

    let mut get_memory_usage = || {
        system.refresh_specifics(refresh);
        let this_process = system.process(pid).expect("failed to get process");
        this_process.memory()
    };

    let initial_memory_usage = get_memory_usage();

    // we need to run the vdom in a async runtime
    for i in 0..=10000 {
        // wait for the vdom to update
        vdom.wait_for_work().await;

        // get the mutations from the vdom
        vdom.render_immediate(&mut NoOpMutations);

        if i % 1000 == 0 {
            let new_memory_usage = get_memory_usage();
            println!("iteration: {} memory usage: {}", i, new_memory_usage);

            // Memory usage might increase as arenas fill up, but it shouldn't double from the initial render
            assert!(new_memory_usage < initial_memory_usage * 2);
        }
    }
}

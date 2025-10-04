use dioxus::prelude::*;

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    let mut store = use_hook(|| Store::new_maybe_sync(0u32));

    rsx! {
        button { onclick: move |_| store += 1, "Increase" }
        "{store}"
        Child { store }
    }
}

#[component]
fn Child(store: WriteStore<u32, SyncStorage>) -> Element {
    // A `WriteStore<T, SyncStorage>` can be sent to another thread!
    // Make sure to clean up your threads when the component unmounts.
    use_hook(|| {
        std::thread::spawn(move || loop {
            std::thread::sleep(std::time::Duration::from_secs(1));
            store += 1;

            if store() >= 10 {
                break;
            }
        });
    });

    rsx! {}
}

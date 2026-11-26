use dioxus_core::prelude::*;

fn main() {
    let mut dom = VirtualDom::new(app);

    loop {
        dom.render_immediate(&mut dioxus_core::dioxus_vdom::NoOpMutations);
        std::thread::sleep(std::time::Duration::from_millis(16));
    }
}

fn app() -> Element {
    let mut signal = use_signal_sync(|| 0);

    use_hook(|| {
        std::thread::spawn(move || {
            loop {
                std::thread::sleep(std::time::Duration::from_secs(1));
                signal += 1;
            }
        });
    });

    rsx! {
        button { onclick: move |_| signal += 1, "Increase" }
        "{signal}"
    }
}

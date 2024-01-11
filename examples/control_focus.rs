use std::rc::Rc;

use dioxus::prelude::*;

fn main() {
    dioxus_desktop::launch(app);
}

fn app(cx: Scope) -> Element {
    let elements: &UseRef<Vec<Rc<MountedData>>> = use_ref(cx, Vec::new);
    let running = use_state(cx, || true);

    use_future!(cx, |(elements, running)| async move {
        let mut focused = 0;
        if *running.current() {
            loop {
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                if let Some(element) = elements.with(|f| f.get(focused).cloned()) {
                    _ = element.set_focus(true).await;
                } else {
                    focused = 0;
                }
                focused += 1;
            }
        }
    });

    cx.render(rsx!(
        div {
            h1 { "Input Roulette" }
            for i in 0..100 {
                input {
                    value: "{i}",
                    onmounted: move |cx| {
                        elements.write().push(cx.inner().clone());
                    },
                    oninput: move |_| {
                        running.set(false);
                    }
                }
            }
        }
    ))
}

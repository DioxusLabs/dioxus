// https://jakelazaroff.com/words/were-react-hooks-a-mistake/
use dioxus::prelude::*;

fn main() {
    dioxus_web::launch(app);
}

fn app(cx: Scope) -> Element {
    let count = use_ref(cx, || 0);
    let started = use_state(cx, || false);

    let start = move || {
        if !*started.get() {
            let count = count.clone(); // clone reference rather than value
            let alert = move || gloo_dialogs::alert(&format!("Your score was {}!", count.read()));
            gloo_timers::callback::Timeout::new(5_000, alert).forget();
        }
        started.set(true); // this cannot be done inside condition or infinite loop
    };

    cx.render(rsx! {
        button {
            onclick: move |_event| {
                start();
                *count.write() += 1;
            },
            if **started {
                "Current score: {count.read()}"
            } else {
                "Start"
            }
        }
    })
}

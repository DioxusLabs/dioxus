// https://jakelazaroff.com/words/were-react-hooks-a-mistake/
use dioxus::prelude::*;

fn main() {
    dioxus_web::launch::launch(app, vec![], Default::default());
}

fn app() -> Element {
    let mut count = use_signal(|| 0);
    let mut started = use_signal(|| false);

    let mut start = move || {
        if !started() {
            let alert = move || gloo_dialogs::alert(&format!("Your score was {count}!",));
            gloo_timers::callback::Timeout::new(5_000, alert).forget();
        }
        started.set(true); // this cannot be done inside condition or infinite loop
    };

    rsx! {
        button {
            onclick: move |_event| {
                start();
                count += 1;
            },

            if started() {
                "Current score: {count}"
            } else {
                "Start"
            }
        }
    }
}

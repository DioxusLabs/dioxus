use dioxus::prelude::*;

fn main() {
    let app = dioxus_webview::new::<()>(|ctx| {
        let (count, set_count) = use_state(ctx, || 0);
        html! {
             <div>
                 <h1> "Dioxus Desktop Demo" </h1>
                 <p> "Count is {count}"</p>
                 <button onclick=|_| set_count(count + 1) >
                     "Click to increment"
                 </button>
             </div>
        }
    });

    app.launch(());
}

fn use_state<T, G>(ctx: &mut Context<G>, init: impl Fn() -> T) -> (T, impl Fn(T)) {
    let g = init();
    (g, |_| {})
}

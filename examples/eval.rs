use dioxus::prelude::*;

fn main() {
    dioxus_desktop::launch(app);
}

fn app(cx: Scope) -> Element {
    let eval_provider = use_eval(cx);
    let mut eval = eval_provider(
        r#"
        dioxus.send("Hi from JS!");
        let msg = await dioxus.recv();
        console.log(msg);
    "#,
    );

    eval.run().unwrap();
    eval.send("Hi from Rust!".into()).unwrap();

    let receiver = eval.receiver();

    let future = use_future(cx, (), |_| async move { receiver.recv().await.unwrap() });

    match future.value() {
        Some(v) => {
            eval.done();
            cx.render(rsx!(
                p { "{v}" }
            ))
        }
        _ => cx.render(rsx!(
            p { "hello" }
        )),
    }
}

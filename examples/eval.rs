use dioxus::prelude::*;

fn main() {
    dioxus_desktop::launch(app);
}

fn app(cx: Scope) -> Element {
    let eval_provider = use_eval(cx);

    let future = use_future(cx, (), |_| {
        to_owned![eval_provider];
        async move {
            let eval = eval_provider(
                r#"
                dioxus.send("Hi from JS!");
                let msg = await dioxus.recv();
                console.log(msg);
                return "hello world";
            "#,
            )
            .unwrap();

            eval.send("Hi from Rust!".into()).unwrap();
            let res = eval.recv().await.unwrap();
            println!("{:?}", eval.await);
            res
        }
    });

    match future.value() {
        Some(v) => cx.render(rsx!(
            p { "{v}" }
        )),
        _ => cx.render(rsx!(
            p { "hello" }
        )),
    }
}

use dioxus::prelude::*;

fn main() {
    dioxus_desktop::launch(app);
}

fn app() -> Element {
    let future = use_future(|_| async move {
        let eval = eval(
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
    });

    match future.value() {
        Some(v) => rsx!(
            p { "{v}" }
        ),
        _ => rsx!(
            p { "hello" }
        ),
    }
}

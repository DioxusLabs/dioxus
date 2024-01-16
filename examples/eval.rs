use dioxus::prelude::*;

fn main() {
    launch_desktop(app);
}

fn app() -> Element {
    let future = use_future(|| async move {
        let mut eval = eval(
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

    match future.value().read().as_ref() {
        Some(v) => rsx!( p { "{v}" } ),
        _ => rsx!( p { "waiting.." } ),
    }
}

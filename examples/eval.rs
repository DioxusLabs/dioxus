//! This example shows how to use the `eval` function to run JavaScript code in the webview.
//!
//! Eval will only work with renderers that support javascript - so currently only the web and desktop/mobile renderers
//! that use a webview. Native renderers will throw "unsupported" errors when calling `eval`.

use dioxus::prelude::*;

fn main() {
    launch(app);
}

fn app() -> Element {
    // Create a future that will resolve once the javascript has been succesffully executed.
    let future = use_resource(move || async move {
        // Wait a little bit just to give the appearance of a loading screen
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

        // The `eval` is available in the prelude - and simply takes a block of JS.
        // Dioxus' eval is interesting since it allows sending messages to and from the JS code using the `await dioxus.recv()`
        // builtin function. This allows you to create a two-way communication channel between Rust and JS.
        let mut eval = eval(
            r#"
                dioxus.send("Hi from JS!");
                let msg = await dioxus.recv();
                console.log(msg);
                return "hi from JS!";
            "#,
        );

        // Send a message to the JS code.
        eval.send("Hi from Rust!".into()).unwrap();

        // Our line on the JS side will log the message and then return "hello world".
        let res = eval.recv().await.unwrap();

        // This will print "Hi from JS!" and "Hi from Rust!".
        println!("{:?}", eval.await);

        res
    });

    match future.value().as_ref() {
        Some(v) => rsx!( p { "{v}" } ),
        _ => rsx!( p { "waiting.." } ),
    }
}

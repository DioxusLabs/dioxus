//! This example shows how to use the `eval` function to run JavaScript code in the webview.
//!
//! Eval will only work with renderers that support javascript - so currently only the web and desktop/mobile renderers
//! that use a webview. Native renderers will throw "unsupported" errors when calling `eval`.

use dioxus::prelude::*;

fn main() {
    launch(app);
}

fn app() -> Element {
    // Create a future that will resolve once the javascript has been successfully executed.
    let future = use_resource(move || async move {
        // Wait a little bit just to give the appearance of a loading screen
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

        // The `eval` is available in the prelude - and simply takes a block of JS.
        // Dioxus' eval is interesting since it allows sending messages to and from the JS code using the `await dioxus.recv()`
        // builtin function. This allows you to create a two-way communication channel between Rust and JS.
        let mut eval = document::eval(
            r#"
            return "hi from JS!";
            "#,
        );

        // This will print "Hi from JS!" and "Hi from Rust!".
        let res = eval.await;

        println!("hello from js! {:?}", res);

        res
    });

    todo!()
    // future.read_unchecked().as_ref().map(|f| match f {
    //     Some(Ok(v)) => rsx!( p { "{v:?}" } ),
    //     Some(Err(e)) => rsx!( p { "{v:?}" } ),
    //     None => rsx!( p { "waiting.." } ),
    // })
}

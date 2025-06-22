use dioxus::prelude::*;

// Generate the greeting function at compile time
// use_js!("examples/assets/example.js"::greeting);

// Or generate multiple functions:
use_js!("examples/assets/example.js"::{greeting, add});

// Or generate all exported functions:
// use_js!("examples/assets/example.js"::*);

fn main() {
    launch(App);
}

#[component]
fn App() -> Element {
    let future = use_resource(|| async move {
        let from = "dave";
        let to = "john";

        // Now we can call the generated function directly!
        let greeting_result = greeting(from, to)
            .await
            .map_err(Box::<dyn std::error::Error>::from)?;
        let greeting: String =
            serde_json::from_value(greeting_result).map_err(Box::<dyn std::error::Error>::from)?;
        Ok::<String, Box<dyn std::error::Error>>(greeting)
    });

    rsx!(
        div {
            h1 { "Dioxus `use_js!` macro example!" }
            {
                match &*future.read() {
                    Some(Ok(greeting)) => rsx! {
                        p { "Greeting from JavaScript: {greeting}" }
                    },
                    Some(Err(e)) => rsx! {
                        p { "Error: {e}" }
                    },
                    None => rsx! {
                        p { "Running js..." }
                    },
                }
            }
        }
    )
}

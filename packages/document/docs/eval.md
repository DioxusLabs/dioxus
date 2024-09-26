# Communicating with JavaScript

You can use the `eval` function to execute JavaScript code in your application with the desktop, mobile, web or liveview renderers. Eval takes a block of JavaScript code (that may be asynchronous) and returns a `UseEval` object that you can use to send data to the JavaScript code and receive data from it.

<div class="warning">

## Safety

Please be careful when executing JavaScript code with `eval`. You should only execute code that you trust. **This applies especially to web targets, where the JavaScript context has access to most, if not all of your application data.** Running untrusted code can lead to a [cross-site scripting](https://developer.mozilla.org/en-US/docs/Glossary/Cross-site_scripting) (XSS) vulnerability.

</div>

```rust
use dioxus::prelude::*;

fn App() -> Element {
    rsx! {
        button {
            onclick: move |_| async move {
                // Eval is a global function you can use anywhere inside Dioxus. It will execute the given JavaScript code.
                let result = eval(r#"console.log("Hello World");
                return "Hello World";"#);

                // You can use the `await` keyword to wait for the result of the JavaScript code.
                println!("{:?}", result.await);
            },
            "Log Hello World"
        }
    }
}
```

## Sending data to JavaScript

When you execute JavaScript code with `eval`, you can pass data to it by formatting the value into the JavaScript code or sending values to the `UseEval` channel.

```rust
use dioxus::prelude::*;

fn app() -> Element {
    rsx! {
        button {
            onclick: move |_| {
                // You can pass initial data to the eval function by formatting it into the JavaScript code.
                const LOOP_COUNT: usize = 10;
                let eval = eval(&format!(r#"for(let i = 0; i < {LOOP_COUNT}; i++) {{
                    // You can receive values asynchronously with the the `await dioxus.recv()` method.
                    let value = await dioxus.recv();
                    console.log("Received", value);
                }}"#));

                // You can send values from rust to the JavaScript code with the `send` method on the object returned by `eval`.
                for i in 0..LOOP_COUNT {
                    eval.send(i.into()).unwrap();
                }
            },
            "Log Count"
        }
    }
}
```

## Sending data from JavaScript

The `UseEval` struct also contains methods for receiving values you send from JavaScript. You can use the `dioxus.send()` method to send values to the JavaScript code and the `UseEval::recv()` method to receive values from the JavaScript code.

```rust
use dioxus::prelude::*;

fn app() -> Element {
    rsx! {
        button {
            onclick: move |_| async move {
                // You can send values from rust to the JavaScript code by using the `send` method on the object returned by `eval`.
                let mut eval = eval(r#"for(let i = 0; i < 10; i++) {
                    // You can send values asynchronously with the `dioxus.send()` method.
                    dioxus.send(i);
                }"#);

                // You can receive values from the JavaScript code with the `recv` method on the object returned by `eval`.
                for _ in 0..10 {
                    let value = eval.recv().await.unwrap();
                    println!("Received {}", value);
                }
            },
            "Log Count"
        }
    }
}
```

## Interacting with the DOM with Eval

You can also use the `eval` function to execute JavaScript code that reads or modifies the DOM. If you want to interact with the mounted DOM, you need to use `eval` inside the [`dioxus_hooks::use_effect`] hook which runs after the component has been mounted.

```rust
use dioxus::prelude::*;

const SCRIPT: &str = r#"
    let element = document.getElementById("my-element");
    element.innerHTML = "Hello World";
    return element.getAttribute("data-count");
"#;

fn app() -> Element {
    // ❌ You shouldn't run eval in the body of a component. This will run before the component has been mounted
    // eval(SCRIPT);

    // ✅ You should run eval inside an effect or event. This will run after the component has been mounted
    use_effect(move || {
        spawn(async {
            let count = eval(SCRIPT).await;
            println!("Count is {:?}", count);
        });
    });


    rsx! {
        div {
            id: "my-element",
            "data-count": "123",
        }
    }
}
```

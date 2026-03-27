//! Web Worker example
//!
//! This example shows how to offload heavy computation to a Web Worker so that
//! the main UI thread stays responsive.
//!
//! ## How it works
//!
//! 1. A JavaScript worker (`assets/worker.js`) runs in a separate browser thread.
//! 2. When the user clicks "Compute", we send the input number to the worker via
//!    `Worker::post_message`.
//! 3. The worker computes the nth Fibonacci number and replies with `postMessage`.
//! 4. A one-shot channel bridges the callback-based worker API into an `async`
//!    future that Dioxus can `spawn`.
//! 5. When the result arrives, a signal is updated and the component re-renders.
//!
//! ## Running
//!
//! ```
//! dx serve --platform web
//! ```

use dioxus::prelude::*;
use futures_channel::oneshot;
use wasm_bindgen::{closure::Closure, JsCast, JsValue};
use web_sys::{MessageEvent, Worker};

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    // `result` holds the text displayed below the button.
    let mut result = use_signal(|| String::from("—"));

    // `pending` prevents the button from being clicked while a computation is running.
    let mut pending = use_signal(|| false);

    // `n` is the Fibonacci index entered by the user.
    let mut n = use_signal(|| 10_u32);

    // `compute` is called when the user clicks the button.
    let compute = move |_| {
        // Guard against double-clicks.
        if pending() {
            return;
        }
        pending.set(true);
        result.set(String::from("Computing…"));

        // Open a one-shot channel so the worker callback can send the result
        // into an async context that Dioxus understands.
        let (tx, rx) = oneshot::channel::<f64>();

        // Spawn the worker. The path is relative to the web root; Dioxus copies
        // everything in `assets/` to the dist root automatically.
        let worker = Worker::new("/worker.js").expect("failed to create worker");

        // `Closure::once` wraps a Rust closure as a JS callback that fires once.
        // When the worker posts its result back, we forward the value into `tx`.
        let onmessage = Closure::once(Box::new(move |event: MessageEvent| {
            // The worker sends `{ input: n, result: fib(n) }`.
            let result_val = js_sys::Reflect::get(&event.data(), &JsValue::from_str("result"))
                .ok()
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);

            // Ignore send errors (the receiver is always waiting here).
            let _ = tx.send(result_val);
        }) as Box<dyn FnOnce(MessageEvent)>);

        worker.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));

        // `forget` keeps the closure alive until it fires; the worker drops it
        // automatically after posting the first message.
        onmessage.forget();

        // Send the input to the worker.
        worker
            .post_message(&JsValue::from_f64(n() as f64))
            .expect("failed to post message to worker");

        // `spawn` runs an async block on Dioxus's executor.
        // We wait for the one-shot channel and then update the UI signal.
        spawn(async move {
            match rx.await {
                Ok(value) => result.set(format!("fib({}) = {}", n(), value as u64)),
                Err(_) => result.set(String::from("Worker error")),
            }
            pending.set(false);
        });
    };

    rsx! {
        div { style: "font-family: sans-serif; max-width: 480px; margin: 3rem auto; padding: 0 1rem;",
            h1 { "Web Worker Example" }
            p {
                "Compute the nth "
                a { href: "https://en.wikipedia.org/wiki/Fibonacci_sequence", target: "_blank",
                    "Fibonacci number"
                }
                " in a Web Worker without blocking the UI thread."
            }

            div { style: "display: flex; gap: 0.5rem; align-items: center; margin-top: 1.5rem;",
                label { r#for: "n-input", "n = " }
                input {
                    id: "n-input",
                    r#type: "number",
                    min: "0",
                    max: "40",
                    value: "{n}",
                    oninput: move |e| {
                        if let Ok(val) = e.value().parse::<u32>() {
                            n.set(val.min(40));
                        }
                    },
                    style: "width: 5rem; padding: 0.25rem 0.5rem; font-size: 1rem;",
                }

                button {
                    onclick: compute,
                    disabled: pending(),
                    style: "padding: 0.4rem 1rem; font-size: 1rem; cursor: pointer;",
                    if pending() { "Computing…" } else { "Compute" }
                }
            }

            p { style: "margin-top: 1.5rem; font-size: 1.25rem;",
                strong { "Result: " }
                "{result}"
            }

            details { style: "margin-top: 2rem; color: #555;",
                summary { "How does this work?" }
                p {
                    "Clicking "Compute" sends the number to a JavaScript Web Worker
                    running in a separate thread ("
                    code { "assets/worker.js" }
                    "). The worker calculates the result and posts it back.
                    A Rust "
                    code { "oneshot" }
                    " channel bridges the callback into an "
                    code { "async" }
                    " block that updates the Dioxus signal when done."
                }
            }
        }
    }
}

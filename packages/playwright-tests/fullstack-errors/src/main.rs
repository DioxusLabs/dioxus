// This test is used by playwright configured in the root of the repo
// Tests:
// - Errors that originate in the initial render

#![allow(non_snake_case)]
use dioxus::prelude::*;

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    rsx! {
        Errors {}
    }
}

#[server]
async fn server_error() -> ServerFnResult<String> {
    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
    Err(ServerFnError::new("the server threw an error!"))
}

#[component]
fn Errors() -> Element {
    rsx! {
        // This is a tricky case for suspense https://github.com/DioxusLabs/dioxus/issues/2570
        // Root suspense boundary is already resolved when the inner suspense boundary throws an error.
        // We need to throw the error from the inner suspense boundary on the server to the hydrated
        // suspense boundary on the client
        ErrorBoundary {
            handle_error: |_| rsx! {
                ErrorFallbackButton {}
            },
            SuspenseBoundary {
                fallback: |_: SuspenseContext| rsx! {
                    div {
                        "Loading..."
                    }
                },
                ThrowsError {}
            }
        }
    }
}

#[component]
pub fn ErrorFallbackButton() -> Element {
    let mut count = use_signal(|| 0);

    rsx! {
        // Make sure the error fallback is interactive after hydration
        button {
            id: "error-fallback-button",
            onclick: move |_| count += 1,
            "Error fallback button clicked {count} times"
        }
    }
}

#[component]
pub fn ThrowsError() -> Element {
    use_server_future(server_error)?.unwrap()?;
    rsx! {
        "success"
    }
}

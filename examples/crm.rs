//! Tiny CRM - A simple CRM app using the Router component and global signals
//!
//! This shows how to use the `Router` component to manage different views in your app. It also shows how to use global
//! signals to manage state across the entire app.
//!
//! We could simply pass the state as a prop to each component, but this is a good example of how to use global state
//! in a way that works across pages.
//!
//! We implement a number of important details here too, like focusing inputs, handling form submits, navigating the router,
//! platform-specific configuration, and importing 3rd party CSS libaries.

use dioxus::prelude::*;

fn main() {
    LaunchBuilder::new()
        .with_cfg(desktop!({
            use dioxus::desktop::{LogicalSize, WindowBuilder};
            dioxus::desktop::Config::default()
                .with_window(WindowBuilder::new().with_inner_size(LogicalSize::new(800, 600)))
        }))
        .launch(|| {
            rsx! {
                link {
                    rel: "stylesheet",
                    href: "https://unpkg.com/purecss@2.0.6/build/pure-min.css",
                    integrity: "sha384-Uu6IeWbM+gzNVXJcM9XV3SohHtmWE+3VGi496jvgX1jyvDTXfdK+rfZc8C1Aehk5",
                    crossorigin: "anonymous"
                }
                style { {include_str!("./assets/crm.css")} }
                h1 { "Dioxus CRM Example" }
                Router::<Route> {}
            }
        });
}

/// We only have one list of clients for the whole app, so we can use a global signal.
static CLIENTS: GlobalSignal<Vec<Client>> = Signal::global(Vec::new);

struct Client {
    first_name: String,
    last_name: String,
    description: String,
}

/// The pages of the app, each with a route
#[derive(Routable, Clone)]
enum Route {
    #[route("/")]
    List,

    #[route("/new")]
    New,

    #[route("/settings")]
    Settings,
}

#[component]
fn List() -> Element {
    rsx! {
        h2 { "List of Clients" }
        Link { to: Route::New, class: "pure-button pure-button-primary", "Add Client" }
        Link { to: Route::Settings, class: "pure-button", "Settings" }
        for client in CLIENTS.read().iter() {
            div { class: "client", style: "margin-bottom: 50px",
                p { "Name: {client.first_name} {client.last_name}" }
                p { "Description: {client.description}" }
            }
        }
    }
}

#[component]
fn New() -> Element {
    let mut first_name = use_signal(String::new);
    let mut last_name = use_signal(String::new);
    let mut description = use_signal(String::new);

    let submit_client = move |_| {
        // Write the client
        CLIENTS.write().push(Client {
            first_name: first_name(),
            last_name: last_name(),
            description: description(),
        });

        // And then navigate back to the client list
        router().push(Route::List);
    };

    rsx! {
        h2 { "Add new Client" }
        form { class: "pure-form pure-form-aligned", onsubmit: submit_client,
            fieldset {
                div { class: "pure-control-group",
                    label { r#for: "first_name", "First Name" }
                    input {
                        id: "first_name",
                        r#type: "text",
                        placeholder: "First Name…",
                        required: true,
                        value: "{first_name}",
                        oninput: move |e| first_name.set(e.value()),

                        // when the form mounts, focus the first name input
                        onmounted: move |e| async move {
                            _ = e.set_focus(true).await;
                        },
                    }
                }

                div { class: "pure-control-group",
                    label { r#for: "last_name", "Last Name" }
                    input {
                        id: "last_name",
                        r#type: "text",
                        placeholder: "Last Name…",
                        required: true,
                        value: "{last_name}",
                        oninput: move |e| last_name.set(e.value())
                    }
                }

                div { class: "pure-control-group",
                    label { r#for: "description", "Description" }
                    textarea {
                        id: "description",
                        placeholder: "Description…",
                        value: "{description}",
                        oninput: move |e| description.set(e.value())
                    }
                }

                div { class: "pure-controls",
                    button { r#type: "submit", class: "pure-button pure-button-primary", "Save" }
                    Link { to: Route::List, class: "pure-button pure-button-primary red", "Cancel" }
                }
            }
        }
    }
}

#[component]
fn Settings() -> Element {
    rsx! {
        h2 { "Settings" }
        button {
            class: "pure-button pure-button-primary red",
            onclick: move |_| {
                CLIENTS.write().clear();
                dioxus::router::router().push(Route::List);
            },
            "Remove all Clients"
        }
        Link { to: Route::List, class: "pure-button", "Go back" }
    }
}

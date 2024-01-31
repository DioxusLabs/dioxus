//! Tiny CRM: A port of the Yew CRM example to Dioxus.
use dioxus::prelude::*;
use dioxus::router::prelude::*;

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
                style { {r#" .red { background-color: rgb(202, 60, 60) !important; } "#} }
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

#[derive(Routable, Clone)]
enum Route {
    #[route("/")]
    ClientList,

    #[route("/new")]
    ClientAdd,

    #[route("/settings")]
    Settings,
}

#[component]
fn ClientList() -> Element {
    rsx! {
        h2 { "List of Clients" }
        Link { to: Route::ClientAdd, class: "pure-button pure-button-primary", "Add Client" }
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
fn ClientAdd() -> Element {
    let mut first_name = use_signal(String::new);
    let mut last_name = use_signal(String::new);
    let mut description = use_signal(String::new);

    let submit_client = move |_: FormEvent| {
        // Write the client
        CLIENTS.write().push(Client {
            first_name: first_name(),
            last_name: last_name(),
            description: description(),
        });

        // And then navigate back to the client list
        dioxus::router::router().push(Route::ClientList);
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
                        required: "",
                        value: "{first_name}",
                        oninput: move |e| first_name.set(e.value()),

                        // when the form mounts, focus the first name input
                        onmounted: move |e| async move {
                            _ = e.set_focus(true).await;
                        },
                    }
                }

                div { class: "pure-control-group",
                    label { "for": "last_name", "Last Name" }
                    input {
                        id: "last_name",
                        r#type: "text",
                        placeholder: "Last Name…",
                        required: "",
                        value: "{last_name}",
                        oninput: move |e| last_name.set(e.value())
                    }
                }

                div { class: "pure-control-group",
                    label { "for": "description", "Description" }
                    textarea {
                        id: "description",
                        placeholder: "Description…",
                        value: "{description}",
                        oninput: move |e| description.set(e.value())
                    }
                }

                div { class: "pure-controls",
                    button { r#type: "submit", class: "pure-button pure-button-primary", "Save" }
                    Link { to: Route::ClientList, class: "pure-button pure-button-primary red", "Cancel" }
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
                dioxus::router::router().push(Route::ClientList);
            },
            "Remove all Clients"
        }
        Link { to: Route::ClientList, class: "pure-button", "Go back" }
    }
}

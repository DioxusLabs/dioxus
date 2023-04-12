//! Tiny CRM: A port of the Yew CRM example to Dioxus.
#![allow(non_snake_case)]

use std::sync::{Arc, Mutex};

use dioxus::prelude::*;
use dioxus_router::prelude::*;

fn main() {
    dioxus_desktop::launch(App);
}

#[derive(Clone, Debug, Default)]
pub struct Client {
    pub first_name: String,
    pub last_name: String,
    pub description: String,
}

type ClientContext = Arc<Mutex<Vec<Client>>>;

fn App(cx: Scope) -> Element {
    use_router(cx, &RouterConfiguration::default, &|| {
        Segment::content(comp(ClientList))
            .fixed(
                "new",
                Route::content(comp(ClientAdd)).name::<ClientAddName>(),
            )
            .fixed(
                "settings",
                Route::content(comp(Settings)).name::<SettingsName>(),
            )
    });

    use_context_provider::<ClientContext>(cx, Default::default);

    render! {
        link {
            rel: "stylesheet",
            href: "https://unpkg.com/purecss@2.0.6/build/pure-min.css",
            integrity: "sha384-Uu6IeWbM+gzNVXJcM9XV3SohHtmWE+3VGi496jvgX1jyvDTXfdK+rfZc8C1Aehk5",
            crossorigin: "anonymous",
        }

        style { "
            .red {{
                background-color: rgb(202, 60, 60) !important;
            }}
        " }

        h1 { "Dioxus CRM Example" }

        Outlet { }
    }
}

fn ClientList(cx: Scope) -> Element {
    let clients = use_context::<ClientContext>(cx).unwrap();

    cx.render(rsx! {
        h2 { "List of Clients" }

        Link {
            target: named::<ClientAddName>(),
            class: "pure-button pure-button-primary",
            "Add Client"
        }
        Link {
            target: named::<SettingsName>(),
            class: "pure-button",
            "Settings"
        }

        clients.lock().unwrap().iter().map(|client| rsx! {
            div {
                class: "client",
                style: "margin-bottom: 50px",

                p { "Name: {client.first_name} {client.last_name}" }
                p { "Description: {client.description}" }
            }
        })
    })
}

struct ClientAddName;
fn ClientAdd(cx: Scope) -> Element {
    let clients = use_context::<ClientContext>(cx).unwrap();
    let first_name = use_state(cx, String::new);
    let last_name = use_state(cx, String::new);
    let description = use_state(cx, String::new);

    let navigator = use_navigate(cx).unwrap();

    cx.render(rsx! {
        h2 { "Add new Client" }

        form {
            class: "pure-form pure-form-aligned",
            onsubmit: move |_| {
                let mut clients = clients.lock().unwrap();

                clients.push(Client {
                    first_name: first_name.to_string(),
                    last_name: last_name.to_string(),
                    description: description.to_string(),
                });

                navigator.push(named::<RootIndex>());
            },

            fieldset {
                div {
                    class: "pure-control-group",
                    label {
                        "for": "first_name",
                        "First Name"
                    }
                    input {
                        id: "first_name",
                        "type": "text",
                        placeholder: "First Name…",
                        required: "",
                        value: "{first_name}",
                        oninput: move |e| first_name.set(e.value.clone())
                    }
                }

                div {
                    class: "pure-control-group",
                    label {
                        "for": "last_name",
                        "Last Name"
                    }
                    input {
                        id: "last_name",
                        "type": "text",
                        placeholder: "Last Name…",
                        required: "",
                        value: "{last_name}",
                        oninput: move |e| last_name.set(e.value.clone())
                    }
                }

                div {
                    class: "pure-control-group",
                    label {
                        "for": "description",
                        "Description"
                    }
                    textarea {
                        id: "description",
                        placeholder: "Description…",
                        value: "{description}",
                        oninput: move |e| description.set(e.value.clone())
                    }
                }

                div {
                    class: "pure-controls",
                    button {
                        "type": "submit",
                        class: "pure-button pure-button-primary",
                        "Save"
                    }
                    Link {
                        target: named::<RootIndex>(),
                        class: "pure-button pure-button-primary red",
                        "Cancel"
                    }
                }
            }


        }
    })
}

struct SettingsName;
fn Settings(cx: Scope) -> Element {
    let clients = use_context::<ClientContext>(cx).unwrap();

    cx.render(rsx! {
        h2 { "Settings" }

        button {
            class: "pure-button pure-button-primary red",
            onclick: move |_| {
                let mut clients = clients.lock().unwrap();
                clients.clear();
            },
            "Remove all Clients"
        }

        Link {
            target: named::<RootIndex>(),
            class: "pure-button",
            "Go back"
        }
    })
}

//! Tiny CRM: A port of the Yew CRM example to Dioxus.
use dioxus::prelude::*;
use dioxus_router::prelude::*;

fn main() {
    dioxus_desktop::launch(App);
}

#[derive(Routable, Clone)]
#[rustfmt::skip]
enum Route {
    #[route("/")]
    ClientList {},
    #[route("/new")]
    ClientAdd {},
    #[route("/settings")]
    Settings {},
}

#[derive(Clone, Debug, Default)]
pub struct Client {
    pub first_name: String,
    pub last_name: String,
    pub description: String,
}

type ClientContext = Vec<Client>;

#[component]
fn App(cx: Scope) -> Element {
    use_shared_state_provider::<ClientContext>(cx, Default::default);

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

        Router::<Route> {}
    }
}

#[component]
fn ClientList(cx: Scope) -> Element {
    let clients = use_shared_state::<ClientContext>(cx).unwrap();

    cx.render(rsx! {
        h2 { "List of Clients" }

        Link {
            to: Route::ClientAdd {},
            class: "pure-button pure-button-primary",
            "Add Client"
        }
        Link {
            to: Route::Settings {},
            class: "pure-button",
            "Settings"
        }

        clients.read().iter().map(|client| rsx! {
            div {
                class: "client",
                style: "margin-bottom: 50px",

                p { "Name: {client.first_name} {client.last_name}" }
                p { "Description: {client.description}" }
            }
        })
    })
}

#[component]
fn ClientAdd(cx: Scope) -> Element {
    let clients = use_shared_state::<ClientContext>(cx).unwrap();
    let first_name = use_state(cx, String::new);
    let last_name = use_state(cx, String::new);
    let description = use_state(cx, String::new);

    let navigator = use_navigator(cx);

    cx.render(rsx! {
        h2 { "Add new Client" }

        form {
            class: "pure-form pure-form-aligned",
            onsubmit: move |_| {
                let mut clients = clients.write();

                clients.push(Client {
                    first_name: first_name.to_string(),
                    last_name: last_name.to_string(),
                    description: description.to_string(),
                });

                navigator.push(Route::ClientList {});
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
                        to: Route::ClientList {},
                        class: "pure-button pure-button-primary red",
                        "Cancel"
                    }
                }
            }


        }
    })
}

#[component]
fn Settings(cx: Scope) -> Element {
    let clients = use_shared_state::<ClientContext>(cx).unwrap();

    cx.render(rsx! {
        h2 { "Settings" }

        button {
            class: "pure-button pure-button-primary red",
            onclick: move |_| {
                let mut clients = clients.write();
                clients.clear();
            },
            "Remove all Clients"
        }

        Link {
            to: Route::ClientList {},
            class: "pure-button",
            "Go back"
        }
    })
}

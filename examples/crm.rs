/*
Tiny CRM: A port of the Yew CRM example to Dioxus.
*/
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

#[allow(non_snake_case)]
fn App(cx: Scope) -> Element {
    let routes = use_segment(&cx, || {
        Segment::new()
            .index(ClientList as Component)
            .fixed("new", Route::new(ClientAdd as Component).name(ClientAdd))
            .fixed("settings", Route::new(Settings as Component).name(Settings))
    });

    use_context_provider::<Vec<Client>>(&cx, Vec::new);

    cx.render(rsx! {
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

        h1 {"Dioxus CRM Example"}

        Router {
            routes: routes.clone(),
            Outlet { }
        }
    })
}

#[allow(non_snake_case)]
fn ClientList(cx: Scope) -> Element {
    let clients = use_context::<Vec<Client>>(&cx).unwrap();

    cx.render(rsx! {
        h2 { "List of Clients" }

        Link {
            target: (ClientAdd, []),
            class: "pure-button pure-button-primary",
            "Add Client"
        }
        Link {
            target: (Settings, []),
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

#[allow(non_snake_case)]
fn ClientAdd(cx: Scope) -> Element {
    let clients = use_context::<Vec<Client>>(&cx).unwrap();
    let first_name = use_state(&cx, String::new);
    let last_name = use_state(&cx, String::new);
    let description = use_state(&cx, String::new);

    let navigator = use_navigate(&cx).unwrap();

    cx.render(rsx! {
        h2 { "Add new Client" }

        form {
            class: "pure-form pure-form-aligned",
            onsubmit: move |_| {
                clients.write().push(Client {
                    first_name: first_name.to_string(),
                    last_name: last_name.to_string(),
                    description: description.to_string(),
                });

                navigator.push((RootIndex, []));
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
                        target: (RootIndex, []),
                        class: "pure-button pure-button-primary red",
                        "Cancel"
                    }
                }
            }


        }
    })
}

#[allow(non_snake_case)]
fn Settings(cx: Scope) -> Element {
    let clients = use_context::<Vec<Client>>(&cx).unwrap();

    cx.render(rsx! {
        h2 { "Settings" }

        button {
            class: "pure-button pure-button-primary red",
            onclick: move |_| clients.write().clear(),
            "Remove all Clients"
        }

        Link {
            target: (RootIndex, []),
            class: "pure-button",
            "Go back"
        }
    })
}

use dioxus_core as dioxus;
use dioxus_core::prelude::*;
use dioxus_core_macro::*;
use dioxus_hooks::*;

use dioxus_html as dioxus_elements;

fn main() {
    dioxus_desktop::set_up_logging(true);
    dioxus_desktop::launch(App, |c| c).unwrap();
}

enum Scene {
    ClientsList,
    NewClientForm,
    Settings,
}

#[derive(Clone, Debug, Default)]
pub struct Client {
    pub first_name: String,
    pub last_name: String,
    pub description: String,
}

static App: FC<()> = |cx, _| {
    let scene = use_state(cx, || Scene::ClientsList);
    let clients = use_ref(cx, || vec![] as Vec<Client>);

    let firstname = use_state(cx, || String::new());
    let lastname = use_state(cx, || String::new());
    let description = use_state(cx, || String::new());

    let scene = match *scene {
        Scene::ClientsList => {
            rsx!(cx, div { class: "crm"
                h2 { "List of clients" margin_bottom: "10px" }
                div { class: "clients" margin_left: "10px"
                    {clients.read().iter().map(|client| rsx!(
                        div { class: "client" style: "margin-bottom: 50px"
                            p { "First Name: {client.first_name}" }
                            p { "Last Name: {client.last_name}" }
                            p {"Description: {client.description}"}
                        })
                    )}
                }
                button { class: "pure-button pure-button-primary" onclick: move |_| scene.set(Scene::NewClientForm), "Add New" }
                button { class: "pure-button" onclick: move |_| scene.set(Scene::Settings), "Settings" }
            })
        }
        Scene::NewClientForm => {
            let add_new = move |_| {
                clients.write().push(Client {
                    description: (*description).clone(),
                    first_name: (*firstname).clone(),
                    last_name: (*lastname).clone(),
                });
                description.set(String::new());
                firstname.set(String::new());
                lastname.set(String::new());
            };
            rsx!(cx, div { class: "crm"
                h2 {"Add new client" margin_bottom: "10px" }
                form { class: "pure-form"
                    input { class: "new-client firstname" placeholder: "First name" value: "{firstname}"
                        oninput: move |evt| firstname.set(evt.value.clone())
                    }
                    input { class: "new-client lastname" placeholder: "Last name" value: "{lastname}"
                        oninput: move |evt| lastname.set(evt.value.clone())
                    }
                    textarea { class: "new-client description" placeholder: "Description" value: "{description}"
                        oninput: move |evt| description.set(evt.value.clone())
                    }
                }
                button { class: "pure-button pure-button-primary", onclick: {add_new}, "Add New" }
                button { class: "pure-button", onclick: move |_| scene.set(Scene::ClientsList), "Go Back" }
            })
        }
        Scene::Settings => {
            rsx!(cx, div {
                h2 { "Settings" margin_bottom: "10px" }
                button {
                    background: "rgb(202, 60, 60)"
                    class: "pure-button pure-button-primary"
                    onclick: move |_| {
                        clients.write().clear();
                        scene.set(Scene::ClientsList);
                    },
                    "Remove all clients"
                }
                button {
                    class: "pure-button pure-button-primary"
                    onclick: move |_| scene.set(Scene::ClientsList),
                    "Go Back"
                }
            })
        }
    };

    rsx!(cx, body {
        link {
            rel: "stylesheet"
            href: "https://unpkg.com/purecss@2.0.6/build/pure-min.css"
            integrity: "sha384-Uu6IeWbM+gzNVXJcM9XV3SohHtmWE+3VGi496jvgX1jyvDTXfdK+rfZc8C1Aehk5"
            crossorigin: "anonymous"
        }
        margin_left: "35%"
        h1 {"Dioxus CRM Example"}
        {scene}
    })
};

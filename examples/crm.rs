/*
Tiny CRM: A port of the Yew CRM example to Dioxus.
*/
use dioxus::prelude::*;

fn main() {
    dioxus::web::launch(App, |c| c);
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

    match *scene {
        Scene::ClientsList => {
            rsx!(cx, div { class: "crm"
                h1 { "List of clients" }
                div { class: "clients" {clients.read().iter().map(|client| rsx!(
                    div { class: "client" style: "margin-bottom: 50px"
                        p { "First Name: {client.first_name}" }
                        p { "Last Name: {client.last_name}" }
                        p {"Description: {client.description}"}
                    }))}
                }
                button { onclick: move |_| scene.set(Scene::NewClientForm), "Add New" }
                button { onclick: move |_| scene.set(Scene::Settings), "Settings" }
            })
        }
        Scene::NewClientForm => {
            rsx!(cx, div { class: "crm"
                h1 {"Add new client"}
                div { class: "names"
                    input { class: "new-client firstname" placeholder: "First name"
                        onchange: move |e| firstname.set(e.value())
                    }
                    input { class: "new-client lastname" placeholder: "Last name"
                        onchange: move |e| lastname.set(e.value())
                    }
                    textarea { class: "new-client description" placeholder: "Description"
                        onchange: move |e| description.set(e.value())
                    }
                }
                button { disabled: "false", onclick: move |_| clients.write().push(Client {
                    description: (*description).clone(),
                    first_name: (*firstname).clone(),
                    last_name: (*lastname).clone(),

                }), "Add New" }
                button { onclick: move |_| scene.set(Scene::ClientsList), "Go Back" }
            })
        }
        Scene::Settings => {
            rsx!(cx, div {
                h1 {"Settings"}
                button { onclick: move |_| clients.write().clear() "Remove all clients"  }
                button { onclick: move |_| scene.set(Scene::ClientsList), "Go Back"  }
            })
        }
    }
};

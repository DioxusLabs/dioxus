use dioxus::prelude::*;

fn main() {
    launch(app);
}

fn app() -> Element {
    let val = use_server_future(fetch_users).suspend()?;

    rsx! {
        h1 { "Users" }

    }
}

#[component]
fn ClientComponent(name: Signal<i32>, id: i64) -> Element {
    rsx! {
        div { "Name: {name}, ID: {id}" }
        button {
            onclick: move |_| async move {
                // Optimistically change the name on the client
                name.set("new name".to_string());

                // Change the name on the server
                change_name(id, "new name".to_string()).await;

                // And then re-fetch the user list
                revalidate(user_list);
            },
            "Change name"
        }
    }
}

#[derive(Table)]
struct Users {
    name: String,
    age: i32,
}

#[server]
async fn fetch_users() -> Result<Element> {
    let users = get_users().await?;

    Ok(rsx! {
        for user in users {
            ClientComponent {
                name: user.name,
                id: user.id,
            }
        }
    })
}

#[server]
async fn change_name(id: i64, new_name: String) -> Result<()> {
    // Send a request to the server to change the name
}

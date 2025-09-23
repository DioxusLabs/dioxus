//! Implementing a login form

use dioxus::fullstack::Form;
use dioxus::prelude::*;

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    rsx! {
        h1 { "Login" }
        form {
            onsubmit: move |evt: FormEvent| async move {
                let res = evt.prevent_default();
            },
            input { r#type: "text", id: "username", name: "username" }
            label { "Username" }
            input { r#type: "password", id: "password", name: "password" }
            label { "Password" }
            button { "Login" }
        }
    }
}

#[derive(serde::Deserialize)]
pub struct LoginForm {}

#[post("/api/login")]
async fn login(form: Form<LoginForm>) -> Result<()> {
    todo!()
}

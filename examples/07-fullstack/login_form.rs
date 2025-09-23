//! Implementing a login form

use dioxus::prelude::*;

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    let onsubmit = move |evt: FormEvent| async move {
        // Intercept the form submission
        let res = evt.prevent_default();

        // login(evt.data()).await;

        // match resp {
        //     // Parse data from here, such as storing a response token
        //     Ok(_data) => println!("Login successful!"),

        //     //Handle any errors from the fetch here
        //     Err(_err) => {
        //         println!("Login failed - you need a login server running on localhost:8080.")
        //     }
        // }

        todo!()
    };

    rsx! {
        h1 { "Login" }
        form { onsubmit,
            input { r#type: "text", id: "username", name: "username" }
            label { "Username" }
            br {}
            input { r#type: "password", id: "password", name: "password" }
            label { "Password" }
            br {}
            button { "Login" }
        }
    }
}

use dioxus::fullstack::Form;

#[derive(serde::Deserialize)]
pub struct LoginForm {}

#[post("/api/login")]
async fn login(form: Form<LoginForm>) -> Result<()> {
    todo!()
}

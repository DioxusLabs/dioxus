//! Handling form submission and deserializing into a struct.
//!
//! `evt.parsed_values::<T>()` takes the form's named inputs and deserializes them straight
//! into your own type via serde. This skips the chore of pulling each field out of the
//! values map by hand — just define a struct whose fields match the `name` attributes,
//! derive `Deserialize`, and you're done.

use dioxus::prelude::*;
use serde::Deserialize;

fn main() {
    dioxus::launch(app);
}

// Field names must match the `name` attribute on each input
#[derive(Deserialize, Debug, Clone)]
struct Signup {
    username: String,
    email: String,
    role: String,
    // Checkboxes are present in the form data only when checked, so model optional fields
    // with Option<String>. Use `#[serde(default)]` so the field doesn't fail to deserialize
    // when the input is missing.
    #[serde(default)]
    newsletter: Option<String>,
}

fn app() -> Element {
    let mut submitted = use_signal(|| None::<Signup>);

    rsx! {
        h1 { "Sign up" }

        form {
            onsubmit: move |evt| {
                // prevent_default stops the browser from navigating the page on submit
                evt.prevent_default();

                // Deserialize directly into our struct
                match evt.parsed_values::<Signup>() {
                    Ok(signup) => submitted.set(Some(signup)),
                    Err(err) => eprintln!("failed to parse form: {err}"),
                }
            },

            div {
                label { r#for: "username", "Username: " }
                input { r#type: "text", id: "username", name: "username", required: true }
            }

            div {
                label { r#for: "email", "Email: " }
                input { r#type: "email", id: "email", name: "email", required: true }
            }

            div {
                label { r#for: "role", "Role: " }
                select { id: "role", name: "role",
                    option { value: "developer", "Developer" }
                    option { value: "designer", "Designer" }
                    option { value: "manager", "Manager" }
                }
            }

            div {
                label { r#for: "newsletter",
                    input { r#type: "checkbox", id: "newsletter", name: "newsletter", value: "yes" }
                    " Subscribe to newsletter"
                }
            }

            button { r#type: "submit", "Submit" }
        }

        if let Some(signup) = submitted() {
            h2 { "Welcome, {signup.username}!" }
            ul {
                li { "Email: {signup.email}" }
                li { "Role: {signup.role}" }
                li { "Newsletter: {signup.newsletter.is_some()}" }
            }
            details {
                summary { "Raw struct" }
                pre { "{signup:#?}" }
            }
        }
    }
}

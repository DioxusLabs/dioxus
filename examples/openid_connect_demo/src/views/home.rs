use dioxus::prelude::*;

/// A home component that renders a welcome message.
///
/// This component demonstrates how we use the USER global state to show a
/// different message if the user is logged in or not.
#[component]
pub fn Home(cx: Scope) -> Element {
    render! {
        div { border: "1px solid black", padding: "1rem",
            h1 {
                "Welcome home, "
                match crate::USER().logged_in() {
                    true => "logged in user".to_string(),
                    false => "stranger".to_string(),
                }
            }
        }
    }
}

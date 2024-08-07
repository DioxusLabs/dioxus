use dioxus::prelude::*;
use lookbook::{Json, LookBook};
use lookbook_macros::preview;

/// To-Do Task.
#[preview]
pub fn TaskPreview(
    /// Label of the task.
    #[lookbook(default = "Ice skating")]
    label: String,

    /// Content of the task.
    #[lookbook(default = "Central Park")]
    content: String,

    /// List of tags.
    #[lookbook(default = vec![String::from("A")])]
    tags: Json<Vec<String>>,
) -> Element {
    rsx!(
        div {
            h4 { "{label}" }
            p { "{content}" }
            div { { tags.0.iter().map(|tag| rsx!(li { "{tag}" })) } }
        }
    )
}

#[component]
fn app() -> Element {
    rsx!(LookBook {
        home: |()| rsx!("Home"),
        previews: [TaskPreview]
    })
}

fn main() {
    dioxus::launch(app)
}

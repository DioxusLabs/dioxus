use crate::server_fns::codec::TextStream;
use dioxus::prelude::*;
use dioxus_fullstack::{launch::LaunchBuilder, prelude::*};
use futures::StreamExt;

fn app(cx: Scope) -> Element {
    let prompt = use_ref(cx, String::new);
    let response = use_ref(cx, String::new);

    cx.render(rsx! {
        div { display: "flex", flex_direction: "column",
            textarea {
                value: "{&*prompt.read()}",
                oninput: move |e| {
                    *prompt.write() = e.data.value.clone();
                }
            }
            button {
                onclick: move |_| {
                    to_owned![prompt, response];
                    async move {
                        let initial_prompt = { prompt.read().clone() };
                        response.write().clear();
                        if let Ok(stream) = mistral(initial_prompt).await {
                            let mut stream = stream.into_inner();
                            while let Some(Ok(text)) = stream.next().await {
                                response.write().push_str(&text);
                            }
                        }
                    }
                },
                "Respond"
            }
            "Response: {&*response.read()}"
        }
    })
}

#[server(output = StreamingText)]
pub async fn mistral(text: String) -> Result<TextStream, ServerFnError> {
    use kalosm_llama::prelude::*;

    static MISTRAL: once_cell::sync::Lazy<Llama> = once_cell::sync::Lazy::new(|| Llama::new_chat());

    let model = &*MISTRAL;
    let message = model.system_prompt_marker().to_string()
        + "You are a helpful assistant who responds to user input with concise, helpful answers."
        + model.end_system_prompt_marker()
        + model.user_marker()
        + &text
        + model.end_user_marker()
        + model.assistant_marker();

    Ok(TextStream::from(
        model
            .stream_text(&message)
            .with_max_length(1000)
            .with_stop_on(model.end_assistant_marker().to_string())
            .await
            .unwrap(),
    ))
}

fn main() {
    LaunchBuilder::new(app).launch()
}

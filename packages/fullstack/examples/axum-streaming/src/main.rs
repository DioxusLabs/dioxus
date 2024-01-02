use dioxus::prelude::*;
use dioxus_fullstack::{launch::LaunchBuilder, prelude::*};
use futures::StreamExt;
use crate::server_fns::codec::TextStream;

fn app(cx: Scope) -> Element {
    let prompt = use_ref(cx, || String::new());

    cx.render(rsx! {
        textarea {
            value: "{&*prompt.read()}",
            oninput: move |e| {
                *prompt.write() = e.data.value.clone();
            }
        }
        button {
            onclick: move |_| {
                to_owned![prompt];
                async move {
                    let initial_prompt = { prompt.read().clone() };
                    if let Ok(stream) = mistral(initial_prompt).await {
                        let mut stream = stream.into_inner();
                        while let Some(Ok(text)) = stream.next().await {
                            prompt.write().push_str(&text);
                        }
                    }
                }
            },
            "Run a server function!"
        }
    })
}

#[server(output = StreamingText)]
pub async fn mistral(text: String) -> Result<TextStream, ServerFnError> {
    static PHI: once_cell::sync::Lazy<rphi::Phi> =
        once_cell::sync::Lazy::new(|| rphi::Phi::v2().unwrap());
    use rphi::prelude::*;

    Ok(TextStream::from(PHI.stream_text(&text).with_max_length(1000).await.unwrap()))
}

fn main() {
    LaunchBuilder::new(app).launch()
}

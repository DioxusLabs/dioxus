#![allow(non_snake_case)]

use dioxus_router::prelude::*;

use dioxus::{html::input_data::keyboard_types::Key, prelude::*};
use dioxus_signals::{use_signal, Signal};
use kalosm::language::*;

fn main() {
    launch(app);
}

fn app() -> Element {
    render! {
        Router::<Route> {}
    }
}

#[derive(Clone, Routable, Debug, PartialEq)]
enum Route {
    #[route("/")]
    Setup {},
    #[route("/chat/:assistant_description")]
    Home { assistant_description: String },
}

#[component]
fn Setup() -> Element {
    let assistant_description = use_signal(String::new);
    let navigator = use_navigator();

    render! {
        div {
            class: "flex flex-col h-screen bg-slate-300",

            div {
                class: "flex flex-col flex-1 p-4 space-y-4 overflow-y-auto",

                div {
                    class: "flex flex-col space-y-4",

                    label {
                        class: "text-xl font-bold",
                        "Assistant Description"
                    }

                    input {
                        class: "p-2 bg-white rounded-lg shadow-md",
                        placeholder: "Type a description...",
                        value: "{assistant_description}",
                        oninput: move |event| {
                            assistant_description.set(event.value.clone())
                        },
                        onkeydown: move |event| {
                            if event.key() == Key::Enter {
                                navigator.push(Route::Home {
                                    assistant_description: assistant_description().clone(),
                                });
                            }
                        },
                    }

                    button {
                        class: "p-2 bg-white rounded-lg shadow-md",
                        onclick: move |_| {
                            let assistant_description = assistant_description().clone();
                            navigator.push(Route::Home {
                                assistant_description: if assistant_description.is_empty() {
                                    "Always assist with care, respect, and truth. Respond with utmost utility yet securely. Avoid harmful, unethical, prejudiced, or negative content. Ensure replies promote fairness and positivity.".to_string()
                                } else {
                                    assistant_description
                                },
                            });
                        },
                        "Start Chatting"
                    }
                }
            }
        }
    }
}

#[component]
fn Home(assistant_description: String) -> Element {
    let current_message = use_signal(String::new);
    let messages: Signal<Vec<Signal<Message>>> = use_signal(Vec::new);
    let assistant_responding = use_signal(|| false);
    let model = use_hook(Llama::new_chat);
    let chat = use_signal(cx, || {
        Chat::builder(model)
            .with_system_prompt(assistant_description.clone())
            .build()
    });

    render! {
        div {
            class: "flex flex-col h-screen bg-slate-300",

            div {
                class: "flex flex-col flex-1 p-4 space-y-4 overflow-y-auto",

                for message in messages().iter().copied() {
                    Message {
                        message: message,
                    }
                }

                div {
                    class: "flex flex-row space-x-4",

                    input {
                        class: "flex-1 p-2 bg-white rounded-lg shadow-md",
                        placeholder: "Type a message...",
                        value: "{current_message}",
                        oninput: move |event| {
                            if !*assistant_responding() {
                                current_message.set(event.value.clone())
                            }
                        },
                        onkeydown: move |event| {
                            if !*assistant_responding() && event.key() == Key::Enter {
                                let mut current_message = current_message.write();
                                let mut messages = messages.write();
                                messages.push(Signal::new(Message {
                                    user: User::User,
                                    text: current_message.clone()
                                }));
                                let final_message = current_message.clone();
                                assistant_responding.set(true);
                                let assistant_response = Signal::new(Message {
                                    user: User::Assistant,
                                    text: String::new(),
                                });
                                messages.push(assistant_response);
                                cx.spawn(async move {
                                    let mut chat = chat.write();
                                    if let Ok(mut stream) = chat.add_message(final_message).await {
                                        while let Some(new_text) = stream.next().await {
                                            assistant_response.write().text += &new_text;
                                        }
                                    }
                                    assistant_responding.set(false);
                                });
                                current_message.clear();
                            }
                        },
                    }
                }
            }
        }
    }
}

#[derive(PartialEq, Clone)]
enum User {
    Assistant,
    User,
}

impl User {
    fn background_color(&self) -> &'static str {
        match self {
            User::Assistant => "bg-red-500",
            User::User => "bg-blue-500",
        }
    }
}

#[derive(PartialEq, Clone)]
struct Message {
    user: User,
    text: String,
}

#[component]
fn Message(cx: Scope, message: Signal<Message>) -> Element {
    let message = message();
    let align = if message.user == User::Assistant {
        "self-start"
    } else {
        "self-end"
    };
    let text = &message.text;
    let assistant_placeholder = message.user == User::Assistant && text.is_empty();
    let text = if assistant_placeholder {
        "Thinking..."
    } else {
        text
    };

    let text_color = if assistant_placeholder {
        "text-gray-400"
    } else {
        ""
    };
    render! {
        div {
            class: "w-2/3 p-2 bg-white rounded-lg shadow-md {align} {text_color}",
            background_color: message.user.background_color(),
            "{text}"
        }
    }
}

use dioxus::prelude::*;
use futures::StreamExt;
use server_fn::codec::{StreamingText, TextStream};

fn main() {
    launch(app)
}

fn app() -> Element {
    let mut prompt = use_signal(String::new);
    let mut response = use_signal(String::new);

    rsx! {
        div { display: "flex", flex_direction: "column", width: "100vw",
            textarea {
                value: "{prompt}",
                wrap: "soft",
                oninput: move |e| {
                    prompt.set(e.value());
                }
            }
            button {
                onclick: move |_| {
                    async move {
                        let initial_prompt = prompt();
                        response.set("Thinking...".into());
                        if let Ok(stream) = mistral(initial_prompt).await {
                            let mut stream = stream.into_inner();
                            let mut first_token = true;
                            while let Some(Ok(text)) = stream.next().await {
                                if first_token {
                                    response.write().clear();
                                    first_token = false;
                                }
                                response.write().push_str(&text);
                            }
                        }
                    }
                },
                "Respond"
            }
            div {
                white_space: "pre-wrap",
                "Response:\n{response}"
            }
        }
    }
}

#[server(output = StreamingText)]
pub async fn mistral(text: String) -> Result<TextStream, ServerFnError> {
    use kalosm_llama::prelude::*;
    use once_cell::sync::OnceCell;

    static MISTRAL: OnceCell<Llama> = OnceCell::new();

    let model = match MISTRAL.get() {
        Some(model) => model,
        None => {
            let model = Llama::new_chat().await.unwrap();
            let _ = MISTRAL.set(model);
            MISTRAL.get().unwrap()
        }
    };
    let markers = model.chat_markers().unwrap();
    let message = markers.system_prompt_marker.to_string()
        + "You are a helpful assistant who responds to user input with concise, helpful answers."
        + markers.end_system_prompt_marker
        + markers.user_marker
        + &text
        + markers.end_user_marker
        + markers.assistant_marker;

    let stream = model
        .stream_text(&message)
        .with_max_length(1000)
        .with_stop_on(markers.end_assistant_marker.to_string())
        .await
        .unwrap();

    Ok(TextStream::new(stream.map(Ok)))
}

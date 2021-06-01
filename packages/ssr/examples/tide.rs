//!
//!
//!
use std::{borrow::Borrow, rc::Rc, sync::Arc};

use async_std::{prelude::*, sync::RwLock};
use dioxus::{events::on::MouseEvent, virtual_dom::VirtualDom};
use dioxus_core::prelude::*;
use tide::{Body, Request, Response};
use tide_websockets::{Message, WebSocket};

#[derive(PartialEq, Props)]
struct ExampleProps {
    initial_name: String,
}

static Example: FC<ExampleProps> = |ctx| {
    let dispaly_name = use_state_new(&ctx, move || ctx.initial_name.clone());

    let buttons = ["Jack", "Jill", "Bob"].iter().map(|name| {
        rsx!{
            button {
                class: "inline-block py-4 px-8 mr-6 leading-none text-white bg-indigo-600 hover:bg-indigo-900 font-semibold rounded shadow"
                onmouseover: move |_| dispaly_name.set(name.to_string())
                "{name}"
            }
        }
    });

    ctx.render(rsx! {
        div {
            class: "py-12 px-4 text-center w-full max-w-2xl mx-auto",
            // classes: [Some("asd")]
            // style: {
            //     a: "asd"
            //     b: "ad"
            // }
            span {
                class: "text-sm font-semibold"
                "Dioxus Example: Jack and Jill"
            }
            h2 {
                class: "text-5xl mt-2 mb-6 leading-tight font-semibold font-heading"
                "Hello, {dispaly_name}"
            }
            {buttons}
        }
    })
};

const TEMPLATE: &str = include_str!("./template.html");

#[async_std::main]
async fn main() -> Result<(), std::io::Error> {
    let mut app = tide::new();

    app.at("/").get(|_| async {
        Ok(Response::builder(200)
            .body(TEMPLATE)
            .content_type(tide::http::mime::HTML)
            .build())
    });

    app.at("/session/:name")
        .get(WebSocket::new(|req: Request<()>, mut stream| async move {
            let initial_name: String = req.param("name")?.parse().unwrap_or("...?".to_string());

            let mut dom = VirtualDom::new_with_props(Example, ExampleProps { initial_name });

            let edits = dom.rebuild().unwrap();
            stream.send_json(&edits).await?;

            // while let Some(Ok(Message::Text(input))) = stream.next().await {
            //     let output: String = input.chars().rev().collect();
            //     stream
            //         .send_string(format!("{} | {}", &input, &output))
            //         .await?;
            // }

            Ok(())
        }));

    app.listen("127.0.0.1:8080").await?;

    Ok(())
}

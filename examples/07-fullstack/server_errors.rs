#![allow(non_snake_case)]
/*
we support anyhow::Error on the bounds, but you just get the error message, not the actual type.


*/

use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    let mut send_request = use_action(move |msg: String| async move {
        // This will lose the error type information, and only let us see the error message, or downcast to serverfn error for more info
        let response = through_anyhow(msg.clone()).await;
        if let Err(Ok(e)) = response.map_err(|e| e.downcast::<ServerFnError>()) {
            todo!()
        }

        // We can go through serverfn directly.
        let response = through_serverfn_err(msg.clone()).await;
        if let Err(e) = response {
            match e {
                ServerFnError::Args(msg) => {
                    println!("Args error: {}", msg);
                }
                _ => {}
            }
        }

        // We can go through our own concrete error type that implements From<ServerFnError>
        let response = through_serverfn_result(msg.clone()).await;

        // We can go through the axum endpoint directly.
        let res = reqwest::get("http://localhost:8000/api/chat")
            .await?
            .json::<i32>()
            .await;

        dioxus::Ok(())
    });

    rsx! {
        button { onclick: move |_| send_request.call("yay".to_string()), "Send" }
    }
}

#[post("/api/chat")]
async fn through_anyhow(user_message: String) -> Result<i32> {
    let abc = std::fs::read_to_string("does_not_exist.txt")?;
    todo!()
}

#[post("/api/chat")]
async fn through_serverfn_err(user_message: String) -> Result<i32, ServerFnError> {
    let abc = std::fs::read_to_string("does_not_exist.txt").context("Failed to read file")?;

    todo!()
}

#[derive(thiserror::Error, Serialize, Deserialize, Debug)]
pub enum CustomFromServerfnError {
    #[error("I/O error: {0}")]
    FailedToEat(String),

    #[error("Sleep error: {0}")]
    FailedToSleep(i32),

    #[error("Coding error: {0}")]
    FailedToCode(String),

    #[error("Comms error: {0}")]
    CommsError(#[from] ServerFnError),
}

#[post("/api/chat")]
async fn through_serverfn_result(user_message: String) -> Result<i32, CustomFromServerfnError> {
    let abc = std::fs::read_to_string("does_not_exist.txt").or_else(|e| {
        Err(CustomFromServerfnError::FailedToEat(format!(
            "Failed to read file: {}",
            e
        )))
    })?;

    let t = Some("yay").ok_or_else(|| CustomFromServerfnError::FailedToCode("no yay".into()))?;

    Ok(123)
}

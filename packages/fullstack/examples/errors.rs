#![allow(non_snake_case)]
/*
we support anyhow::Error on the bounds, but you just get the error message, not the actual type.


*/

use axum::response::IntoResponse;
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    let mut send_request = use_action(move |msg: String| async move {
        let mut response = through_anyhow(msg.clone()).await;

        if let Err(Ok(e)) = response.map_err(|e| e.downcast::<ServerFnError>()) {
            todo!()
        }

        let mut response = through_serverfn_err(msg.clone()).await;

        dioxus::Ok(())
    });

    rsx! {
        button { onclick: move |_| send_request.dispatch("yay".to_string()), "Send" }
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

#[derive(thiserror::Error, Debug)]
enum CustomIntoResponseError {
    #[error("I/O error: {0}")]
    Eat(#[from] std::io::Error),

    #[error("Sleep error: {0}")]
    Sleep(i32),

    #[error("Coding error: {0}")]
    Code(String),
}

impl IntoResponse for CustomIntoResponseError {
    fn into_response(self) -> axum::response::Response {
        todo!()
    }
}

#[post("/api/chat")]
async fn custom_errors(user_message: String) -> Result<i32, CustomIntoResponseError> {
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

    todo!()
}

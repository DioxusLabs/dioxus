use dioxus::prelude::{
    dioxus_stores::{use_store, Store},
    *,
};
use std::num::ParseIntError;

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    let result = use_store(|| {
        "0".parse::<u32>()
            .map(|data| TodoState { data })
            .map_err(|error| ParseError { error })
    });
    match result.as_result() {
        Ok(data) => {
            rsx! {
                HandleOk { data }
            }
        }
        Err(error) => {
            rsx! {
                HandleError { error }
            }
        }
    }
}

#[component]
fn HandleError(error: Store<ParseError>) -> Element {
    rsx! {
        h1 { "Error parsing number" }
        p { "An error occurred while parsing the number: {error.error()}" }
    }
}

#[component]
fn HandleOk(data: Store<TodoState>) -> Element {
    rsx! {
        h1 { "Parsed number successfully" }
        p { "The parsed number is: {data.data()}" }
    }
}

#[derive(Store)]
struct TodoState {
    #[store(foreign)]
    data: u32,
}

#[derive(Store)]
struct ParseError {
    #[store(foreign)]
    error: ParseIntError,
}

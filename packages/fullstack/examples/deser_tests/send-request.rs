use axum::Json;
use dioxus_fullstack::{
    fetch::{fetch, make_request},
    ServerFnError, ServerFnRequestExt,
};

#[tokio::main]
async fn main() {
    #[derive(serde::Deserialize, serde::Serialize, Debug)]
    struct UrlParams {
        // id: i32,
        amount: Option<u32>,
        offset: Option<u32>,
    }

    // /item/{id}?amount&offset
    let id = 123;

    #[derive(serde::Serialize, serde::Deserialize, Debug)]
    struct YourObject {
        id: i32,
        amount: Option<i32>,
        offset: Option<i32>,
    }

    let res = make_request::<Json<YourObject>, _>(
        http::Method::GET,
        &format!("http://localhost:3000/item/{}", id),
        &UrlParams {
            amount: Some(10),
            offset: None,
        },
        // None,
    )
    .await;

    println!("first {:#?}", res.unwrap());

    let res = make_request::<YourObject, _>(
        http::Method::GET,
        &format!("http://localhost:3000/item/{}", id),
        &UrlParams {
            amount: Some(11),
            offset: None,
        },
        // None,
    )
    .await;

    println!("second {:#?}", res.unwrap());
}

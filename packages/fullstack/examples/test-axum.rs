use anyhow::Result;
use std::{
    any::TypeId,
    marker::PhantomData,
    prelude::rust_2024::{Future, IntoFuture},
    process::Output,
};

use axum::{
    extract::State,
    response::{Html, IntoResponse},
    routing::MethodRouter,
    Json,
};
use bytes::Bytes;
use dioxus::prelude::*;
use dioxus_fullstack::{
    fetch::{FileUpload, WebSocket},
    route, serverfn_sugar, DioxusServerState, ServerFunction,
};
use http::Method;
use reqwest::RequestBuilder;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use url::Url;

#[tokio::main]
async fn main() {
    // let res = home_page(123).await;

    ServerFunction::serve(|| {
        let routes = ServerFunction::collect();

        use_future(|| async move {
            let mut ws = ws_endpoint().await.unwrap();
            while let Ok(res) = ws.recv().await {
                // Handle incoming WebSocket messages
            }
        });

        rsx! {
            h1 { "We have dioxus fullstack at home!" }
            div { "Our routes:" }
            ul {
                for r in routes {
                    li {
                        a { href: "{r.path()}", "{r.method()} {r.path()}" }
                    }
                }
                button {
                    onclick: move |_| async move {
                        // let res = get_item(1, None, None).await?;
                    }
                }
                button {
                    onclick: move |_| async move {
                        let mut file = FileUpload::from_stream(
                            "myfile.png".to_string(),
                            "image/png".to_string(),
                            futures::stream::iter(vec![
                                Ok(Bytes::from_static(b"hello")),
                                Ok(Bytes::from_static(b"world")),
                            ]),
                        );

                        let uuid = streaming_file(file).await.unwrap();
                    }
                }

            }
        }
    })
    .await;
}
/*

an fn that returns an IntoFuture / async fn
- is clearer that it's an async fn....
- still shows up as a function
- can guard against being called on the client with IntoFuture?
- can be used as a handler directly
- requires a trait to be able to mess with it
- codegen for handling inputs seems more straightforward?

a static that implements Deref to a function pointer
- can guard against being called on the client
- can be used as a handler directly
- has methods on the static itself (like .path(), .method()) as well as the result
- does not show up as a proper function in docs
- callable types are a weird thing to do. deref is always weird to overload
- can have a builder API!

qs:
- should we even make it so you can access its props directly?
*/

#[get("/home")]
async fn home(state: State<DioxusServerState>) -> String {
    format!("hello home!")
}

#[get("/home/{id}")]
async fn home_page(id: String) -> String {
    format!("hello home {}", id)
}

#[get("/upload/image/")]
async fn streaming_file(body: FileUpload) -> Result<Json<i32>> {
    todo!()
}

#[get("/")]
async fn ws_endpoint() -> Result<WebSocket<String, String>> {
    todo!()
}

#[get("/item/{id}?amount&offset")]
async fn get_item(id: i32, amount: Option<i32>, offset: Option<i32>) -> Json<YourObject> {
    Json(YourObject { id, amount, offset })
}

#[get("/item/{id}?amount&offset")]
async fn get_item2(id: i32, amount: Option<i32>, offset: Option<i32>) -> Result<Json<YourObject>> {
    Ok(Json(YourObject { id, amount, offset }))
}

#[get("/item/{id}?amount&offset")]
async fn get_item3(id: i32, amount: Option<i32>, offset: Option<i32>) -> Result<YourObject> {
    Ok(YourObject { id, amount, offset })
}

#[get("/item/{id}?amount&offset")]
async fn try_get_item(
    id: i32,
    amount: Option<i32>,
    offset: Option<i32>,
) -> Result<Json<YourObject>, ServerFnError> {
    Ok(Json(YourObject { id, amount, offset }))
}

#[derive(serde::Serialize, serde::Deserialize)]
struct YourObject {
    id: i32,
    amount: Option<i32>,
    offset: Option<i32>,
}

#[post("/work")]
async fn post_work() -> Html<&'static str> {
    Html("post work")
}

#[get("/work")]
async fn get_work() -> Html<&'static str> {
    Html("get work")
}

#[get("/play")]
async fn go_play() -> Html<&'static str> {
    Html("hello play")
}

#[get("/dx-element")]
async fn get_element() -> Html<String> {
    Html(dioxus_ssr::render_element(rsx! {
        div { "we have ssr at home..." }
    }))
}

struct ServerOnlyEndpoint<In, Out> {
    _t: PhantomData<In>,
    _o: PhantomData<Out>,
    make_req: fn(In) -> Pending<Out>,
    handler: fn() -> MethodRouter<DioxusServerState>,
}

impl<Arg1, Arg2, Arg3, Res, Err> std::ops::Deref
    for ServerOnlyEndpoint<(Arg1, Arg2, Arg3), Result<Res, Err>>
{
    type Target = fn(Arg1, Arg2, Arg3) -> Pending<Result<Res, Err>>;

    fn deref(&self) -> &Self::Target {
        todo!()
    }
}

struct Pending<T> {
    _p: PhantomData<T>,
}
impl<T> IntoFuture for Pending<T> {
    type Output = T;
    type IntoFuture = std::pin::Pin<Box<dyn std::future::Future<Output = Self::Output>>>;

    fn into_future(self) -> Self::IntoFuture {
        todo!()
    }
}

static my_endpoint: ServerOnlyEndpoint<
    (i32, Option<i32>, Option<i32>),
    Result<YourObject, String>,
> = ServerOnlyEndpoint {
    _t: PhantomData,
    _o: PhantomData,
    handler: || axum::routing::get(|| async move {}),
    make_req: |(id, amount, offset)| {
        //

        // let host = "http://localhost:3000";
        // let url = format!("{host}/blah/{}?amount={:?}&offset={:?}", id, amount, offset);
        // let wip = reqwest::Request::new(reqwest::Method::GET, Url::parse(&url).unwrap());

        todo!()
    },
};

async fn send_client(id: i32, amount: Option<i32>, offset: Option<i32>) {
    // let client = reqwest::Client::new();
    // let body = serde_json::json!({});
    // let res = client
    //     .get("http://localhost:3000/{id}/")
    //     .query(query)
    //     .body(serde_json::to_vec(&body).unwrap())
    //     .header("Content-Type", "application/json")
    //     .send()
    //     .await;
    todo!()
}

async fn it_works() {
    let res = my_endpoint(1, None, None).await;
}

// impl<T> ServerOnlyEndpoint<T> {
//     const fn new(_t: fn() -> T) -> ServerOnlyEndpoint<T> {
//         ServerOnlyEndpoint { _t }
//     }
// }

// impl<T: Serialize, E: Serialize> std::ops::Deref for ServerOnlyEndpoint<Result<T, E>> {
//     type Target = fn() -> Result<T, E>;

//     fn deref(&self) -> &Self::Target {
//         todo!()
//     }
// }

// static MyEndpoint: ServerOnlyEndpoint<(i32, String), Result<YourObject, String>> =
//     ServerOnlyEndpoint::new(|| {
//         Ok(YourObject {
//             id: 1,
//             amount: None,
//             offset: None,
//         })
//     });

// static MyEndpoint2: ServerOnlyEndpoint<YourObject> = ServerOnlyEndpoint::new(|| YourObject {
//     id: 1,
//     amount: None,
//     offset: None,
// });

// trait EndpointResult {
//     type Output;
// }
// struct M1;
// impl<T: Serialize, E: Serialize> EndpointResult for fn() -> Result<T, E> {
//     type Output = String;
// }

// struct M2;
// struct CantCall;
// impl<T> EndpointResult for &fn() -> T {
//     type Output = CantCall;
// }

// fn e1() -> Result<YourObject, ServerFnError> {
//     todo!()
// }
// fn e2() -> YourObject {
//     todo!()
// }

// fn how_to_make_calling_a_compile_err() {
//     fn call_me_is_comp_error() {}

//     // let res = MyEndpoint();
//     // let res = MyEndpoint2();

//     trait ItWorksGood {
//         #[deny(deprecated)]
//         #[deprecated(note = "intentionally make this a compile error")]
//         fn do_it(&self);
//     }
// }

// struct PendingServerResponse<R, M = ()> {
//     _p: PhantomData<R>,
//     _m: PhantomData<M>,
// }

// impl<T: Serialize, E: DeserializeOwned> std::future::IntoFuture
//     for PendingServerResponse<Result<T, E>>
// {
//     type Output = String;
//     type IntoFuture = std::pin::Pin<Box<dyn std::future::Future<Output = Self::Output>>>;
//     fn into_future(self) -> Self::IntoFuture {
//         Box::pin(async move { todo!() })
//     }
// }

// struct CantRespondMarker;
// impl<T> std::future::IntoFuture for &PendingServerResponse<T> {
//     type Output = CantRespondMarker;
//     type IntoFuture = std::pin::Pin<Box<dyn std::future::Future<Output = Self::Output>>>;
//     fn into_future(self) -> Self::IntoFuture {
//         Box::pin(async move { todo!() })
//     }
// }

// // trait WhichMarker<T> {
// //     type Marker;
// // }

// async fn it_works_maybe() {
//     fn yay1() -> PendingServerResponse<Result<YourObject, String>> {
//         PendingServerResponse {
//             _p: PhantomData,
//             _m: PhantomData,
//         }
//     }

//     fn yay2() -> PendingServerResponse<i32> {
//         PendingServerResponse {
//             _p: PhantomData,
//             _m: PhantomData,
//         }
//     }

//     // let a = yay1().await;
//     // let a = yay2().await;
// }

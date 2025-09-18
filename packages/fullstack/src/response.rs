use std::prelude::rust_2024::Future;

use axum::extract::FromRequest;
use bytes::Bytes;
use dioxus_fullstack_core::DioxusServerState;
use http::HeaderMap;
use serde::{
    de::{DeserializeOwned, DeserializeSeed},
    Deserialize, Serialize,
};

use crate::IntoRequest;

pub struct ServerResponse {
    headers: HeaderMap,
    status: http::StatusCode,
}

impl ServerResponse {
    pub async fn new_from_reqwest(res: reqwest::Response) -> Self {
        let status = res.status();
        let headers = res.headers();
        todo!()
    }
}

impl IntoRequest for axum::extract::Request {
    fn into_request(
        input: Self,
        request_builder: reqwest::RequestBuilder,
    ) -> impl Future<Output = Result<reqwest::Response, reqwest::Error>> + Send + 'static {
        async move { todo!() }
    }
}

#[derive(Serialize, Deserialize)]
struct UserInput<A, B, C> {
    name: A,
    age: B,
    extra: C,
}

async fn it_works() {
    trait ExtractIt<M> {
        type Input;
        type Output;
        async fn extract_one(
            &self,
            request: axum_core::extract::Request,
            body: Self::Input,
            first: fn(Self::Input) -> Self::Output,
        ) -> Self::Output;
    }

    struct ExtractOneMarker;

    struct Extractor<T, O> {
        _t: std::marker::PhantomData<T>,
        _o: std::marker::PhantomData<O>,
    }
    impl<T, O> Extractor<T, O> {
        fn new() -> Self {
            Self {
                _t: std::marker::PhantomData,
                _o: std::marker::PhantomData,
            }
        }
    }

    impl<T: Serialize + DeserializeOwned, O> ExtractIt<ExtractOneMarker> for &&Extractor<T, O> {
        type Input = T;
        type Output = O;
        async fn extract_one(
            &self,
            request: axum_core::extract::Request,
            body: Self::Input,
            first: fn(Self::Input) -> O,
        ) -> Self::Output {
            first(body)
        }
    }

    impl<T, O: FromRequest<DioxusServerState>> ExtractIt<ExtractOneMarker> for &Extractor<T, O> {
        type Input = T;
        type Output = O;

        async fn extract_one(
            &self,
            request: axum_core::extract::Request,
            body: Self::Input,
            first: fn(Self::Input) -> Self::Output,
        ) -> Self::Output {
            first(body)
        }
    }

    let request = axum_core::extract::Request::default();

    let e =
        Extractor::<UserInput<String, u32, Option<String>>, (String, u32, Option<String>)>::new();
    let res = (&&&&e)
        .extract_one(
            request,
            UserInput::<String, u32, Option<String>> {
                name: "Alice".to_string(),
                age: 30,
                extra: None::<String>,
            },
            |x| (x.name, x.age, x.extra),
        )
        .await;

    #[derive(Serialize, Deserialize)]
    struct SingleRequest<T> {
        request: T,
    }
    use axum_core::extract::Request;

    let e2 = Extractor::<SingleRequest<Request>, (Request,)>::new();
    let res = (&&&e2)
        .extract_one(
            Request::default(),
            SingleRequest {
                request: Request::default(),
            },
            |x| (x.request,),
        )
        .await;
}

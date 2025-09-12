use axum::{
    body::Body,
    extract::{FromRequest, FromRequestParts, Request, State},
    Json,
};
use bytes::Bytes;
use dioxus_fullstack::{DioxusServerState, ServerFnRejection};
use futures::StreamExt;
use http::{request::Parts, HeaderMap};
use http_body_util::BodyExt;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

#[tokio::main]
async fn main() {
    let state = State(DioxusServerState::default());

    let r = (&&&&&&DeSer::<(HeaderMap, HeaderMap, HeaderMap), _>::new())
        .extract(Request::new(Body::empty()), &state, ("", "", ""))
        .await;

    let r = (&&&&&&DeSer::<(HeaderMap, HeaderMap, String), _>::new())
        .extract(Request::new(Body::empty()), &state, ("", "", ""))
        .await;

    let r = (&&&&&&DeSer::<(HeaderMap, String, String), _>::new())
        .extract(Request::new(Body::empty()), &state, ("", "", ""))
        .await;

    let r = (&&&&&&DeSer::<(String, String, String), _>::new())
        .extract(Request::new(Body::empty()), &state, ("", "", ""))
        .await;

    let r = (&&&&&&DeSer::<(String, (), ()), _>::new())
        .extract(Request::new(Body::empty()), &state, ("", "", ""))
        .await;

    let r = (&&&&&&DeSer::<(HeaderMap, Json<()>, ()), _>::new())
        .extract(Request::new(Body::empty()), &state, ("", "", ""))
        .await;
}

struct DeSer<T, BodyTy, Body = Json<BodyTy>> {
    _t: std::marker::PhantomData<T>,
    _body: std::marker::PhantomData<BodyTy>,
    _encoding: std::marker::PhantomData<Body>,
}

impl<T, Encoding> DeSer<T, Encoding> {
    fn new() -> Self {
        DeSer {
            _t: std::marker::PhantomData,
            _body: std::marker::PhantomData,
            _encoding: std::marker::PhantomData,
        }
    }
}

trait ExtractP0<O> {
    async fn extract(
        &self,
        request: Request,
        state: &State<DioxusServerState>,
        names: (&'static str, &'static str, &'static str),
    ) -> Result<O, ServerFnRejection>;
}

impl<A, B, C> ExtractP0<(A, B, C)> for &&&&&DeSer<(A, B, C), ()>
where
    A: FromRequestParts<DioxusServerState>,
    B: FromRequestParts<DioxusServerState>,
    C: FromRequestParts<DioxusServerState>,
{
    async fn extract(
        &self,
        request: Request,
        state: &State<DioxusServerState>,
        names: (&'static str, &'static str, &'static str),
    ) -> Result<(A, B, C), ServerFnRejection> {
        let (mut parts, _) = request.into_parts();
        Ok((
            A::from_request_parts(&mut parts, state)
                .await
                .map_err(|_| ServerFnRejection {})?,
            B::from_request_parts(&mut parts, state)
                .await
                .map_err(|_| ServerFnRejection {})?,
            C::from_request_parts(&mut parts, state)
                .await
                .map_err(|_| ServerFnRejection {})?,
        ))
    }
}

trait Unit {}
impl Unit for () {}

trait ExtractPB0<O> {
    async fn extract(
        &self,
        request: Request,
        state: &State<DioxusServerState>,
        names: (&'static str, &'static str, &'static str),
    ) -> Result<O, ServerFnRejection>;
}

impl<A, B, C> ExtractPB0<(A, B, C)> for &&&&DeSer<(A, B, C), ()>
where
    A: FromRequestParts<DioxusServerState>,
    B: FromRequest<DioxusServerState>,
    C: Unit,
{
    async fn extract(
        &self,
        request: Request,
        state: &State<DioxusServerState>,
        names: (&'static str, &'static str, &'static str),
    ) -> Result<(A, B, C), ServerFnRejection> {
        todo!()
    }
}

trait ExtractP1<O> {
    async fn extract(
        &self,
        request: Request,
        state: &State<DioxusServerState>,
        names: (&'static str, &'static str, &'static str),
    ) -> Result<O, ServerFnRejection>;
}

impl<A, B, C> ExtractP1<(A, B, C)> for &&&DeSer<(A, B, C), (C,)>
where
    A: FromRequestParts<DioxusServerState>,
    B: FromRequestParts<DioxusServerState>,
    C: DeserializeOwned,
{
    async fn extract(
        &self,
        request: Request,
        state: &State<DioxusServerState>,
        names: (&'static str, &'static str, &'static str),
    ) -> Result<(A, B, C), ServerFnRejection> {
        let (mut parts, body) = request.into_parts();
        let a = A::from_request_parts(&mut parts, state)
            .await
            .map_err(|_| ServerFnRejection {})?;

        let b = B::from_request_parts(&mut parts, state)
            .await
            .map_err(|_| ServerFnRejection {})?;

        let bytes = body.collect().await.unwrap().to_bytes();
        let (_, _, c) = struct_to_named_tuple::<(), (), C>(bytes, ("", "", names.2));

        Ok((a, b, c))
    }
}

trait ExtractP2<O> {
    async fn extract(
        &self,
        request: Request,
        state: &State<DioxusServerState>,
        names: (&'static str, &'static str, &'static str),
    ) -> O;
}

impl<A, B, C> ExtractP2<(A, B, C)> for &&DeSer<(A, B, C), (B, C)>
where
    A: FromRequestParts<DioxusServerState>,
    B: DeserializeOwned,
    C: DeserializeOwned,
{
    async fn extract(
        &self,
        request: Request,
        state: &State<DioxusServerState>,
        names: (&'static str, &'static str, &'static str),
    ) -> (A, B, C) {
        todo!()
    }
}

trait ExtractP3<O> {
    async fn extract(
        &self,
        request: Request,
        state: &State<DioxusServerState>,
        names: (&'static str, &'static str, &'static str),
    ) -> O;
}
impl<A, B, C> ExtractP3<(A, B, C)> for &DeSer<(A, B, C), (A, B, C)>
where
    A: DeserializeOwned,
    B: DeserializeOwned,
    C: DeserializeOwned,
{
    async fn extract(
        &self,
        request: Request,
        state: &State<DioxusServerState>,
        names: (&'static str, &'static str, &'static str),
    ) -> (A, B, C) {
        todo!()
    }
}

/// Todo: make this more efficient with a custom visitor instead of using serde_json intermediate
fn struct_to_named_tuple<A, B, C>(
    body: Bytes,
    names: (&'static str, &'static str, &'static str),
) -> (A, B, C) {
    todo!()
}

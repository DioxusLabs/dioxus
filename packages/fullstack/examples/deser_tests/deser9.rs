use std::prelude::rust_2024::Future;

use axum::{
    extract::{FromRequest, FromRequestParts},
    handler::Handler,
    Json,
};
use dioxus_fullstack::DioxusServerState;
use http::HeaderMap;
use serde::{de::DeserializeOwned, Deserialize};

fn main() {
    #[derive(Deserialize)]
    struct Both;
    impl FromRequestParts<()> for Both {
        type Rejection = ();

        async fn from_request_parts(
            _parts: &mut axum::http::request::Parts,
            _state: &(),
        ) -> Result<Self, Self::Rejection> {
            Ok(Both)
        }
    }

    // fn assert_handler<T, F: MyHandler<T>>(_: F) -> T {
    //     todo!()
    // }

    async fn handler1() {}
    async fn handler2(t: HeaderMap) {}
    async fn handler3(t: HeaderMap, body: Json<String>) {}
    async fn handler4(t: HeaderMap, a: HeaderMap) {}
    async fn handler5(t: HeaderMap, a: i32) {}
    async fn handler6(t: HeaderMap, a: Both) {}

    let a = handler1;
    let a = handler2;
    let a = handler3;
    let a = handler4;
    let a = handler5;
    let a = handler6;

    let res = MyDe::new()
        .queue::<HeaderMap, _>()
        .queue::<i32, _>()
        .queue::<String, _>()
        .queue::<bool, _>()
        .execute();
}

struct MyDe<TypeChain, M> {
    _phantom: std::marker::PhantomData<TypeChain>,
    _marker: std::marker::PhantomData<M>,
}

impl MyDe<(), ()> {
    fn new() -> Self {
        MyDe {
            _phantom: std::marker::PhantomData,
            _marker: std::marker::PhantomData,
        }
    }
    fn queue<NewType: MyExtract<M>, M>(self) -> MyDe<(NewType,), (M,)> {
        MyDe {
            _phantom: std::marker::PhantomData,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<A, M1> MyDe<(A,), (M1,)> {
    fn queue<NewType: MyExtract<M2>, M2>(self) -> MyDe<(A, NewType), (M1, M2)> {
        MyDe {
            _phantom: std::marker::PhantomData,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<A, B, M1, M2> MyDe<(A, B), (M1, M2)> {
    fn queue<NewType: MyExtract<M3>, M3>(self) -> MyDe<(A, B, NewType), (M1, M2, M3)> {
        MyDe {
            _phantom: std::marker::PhantomData,
            _marker: std::marker::PhantomData,
        }
    }
}
impl<A, B, C, M1, M2, M3> MyDe<(A, B, C), (M1, M2, M3)> {
    fn queue<NewType: MyExtract<M4>, M4>(self) -> MyDe<(A, B, C, NewType), (M1, M2, M3, M4)> {
        MyDe {
            _phantom: std::marker::PhantomData,
            _marker: std::marker::PhantomData,
        }
    }
}

trait MyExtract<M> {
    type Out;
}

struct ViaPartsMarker;
impl<T> MyExtract<ViaPartsMarker> for T
where
    T: FromRequestParts<DioxusServerState>,
{
    type Out = T;
}

struct DeserializeMarker;
impl<T> MyExtract<DeserializeMarker> for T
where
    T: DeserializeOwned,
{
    type Out = T;
}

// impl<A, B, C, D, M1, M2, M3, M4> MyDe<(A, B, C, D), (M1, M2, M3, M4)>
// where
//     A: MyExtract<M1>,
//     B: MyExtract<M2>,
//     C: MyExtract<M3>,
//     D: MyExtract<M4>,
// {
//     fn execute(self) -> (A::Out, B::Out, C::Out, D::Out) {
//         todo!()
//     }
// }

impl<A, B, C, D>
    MyDe<
        (A, B, C, D),
        (
            ViaPartsMarker,
            DeserializeMarker,
            DeserializeMarker,
            DeserializeMarker,
        ),
    >
where
    A: MyExtract<ViaPartsMarker>,
    B: MyExtract<DeserializeMarker>,
    C: MyExtract<DeserializeMarker>,
    D: MyExtract<DeserializeMarker>,
{
    fn execute(self) -> (A::Out, B::Out, C::Out, D::Out) {
        todo!()
    }
}

impl<A, B, C, D>
    MyDe<
        (A, B, C, D),
        (
            ViaPartsMarker,
            ViaPartsMarker,
            DeserializeMarker,
            DeserializeMarker,
        ),
    >
where
    A: MyExtract<ViaPartsMarker>,
    B: MyExtract<ViaPartsMarker>,
    C: MyExtract<DeserializeMarker>,
    D: MyExtract<DeserializeMarker>,
{
    fn execute(self) -> (A::Out, B::Out, C::Out, D::Out) {
        todo!()
    }
}

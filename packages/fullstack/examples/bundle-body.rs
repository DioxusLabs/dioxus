use axum::{
    extract::{FromRequest, FromRequestParts, Request, State},
    handler::Handler,
    Json,
};
use dioxus_fullstack::{post, DioxusServerState, ServerFnRejection};
use dioxus_fullstack::{ServerFnSugar, ServerFunction};
use http::{request::Parts, HeaderMap};
use serde::{de::DeserializeOwned, Deserialize};
use std::{marker::PhantomData, prelude::rust_2024::Future};

fn main() {}

// #[post("/user/{id}")]
// async fn create_user(headers: HeaderMap, id: i32, name: String, age: serde_json::Value) -> Json<SomeCoolBody> {
// -> Body { name, age }
//

// header: HeaderMap,

#[derive(Deserialize)]
struct SomeCoolBody {
    id: i32,
    date: i32,
    name: String,
    age: serde_json::Value,
}

/*
How it works:
- remove all query params and route params
- we should be left with just FromRequestParts and the Body
- our extractor runs through the items, pushing each into a separate list
- if there is only one item in the body list and it is a FromRequest, we just use that
- we run the FromRequestParts extractors first
- then we deserialize the body into the target items usually a handrolled deserializer.

Potential ways of "tightening" this:
- FromRequestParts items must come *first*... which is only logical...
- either a series of T: Deserialize or a single FromRequest
- single-string bodies become json... *or* we could also accept bare strings

Ideas
- queue and shuffle types left to right (or right to left). typestate prevents invalid order.
- overload fromrequest for the final item
- temporarily only allow FromRequestParts to be in the server declaration, all others must be deserializable
- only do the automatic deserialize thing if no FromRequestParts are present
*/
async fn extract_some_cool_body_from_request(
    state: State<DioxusServerState>,
    r: Request,
) -> anyhow::Result<()> {
    let (mut parts, body) = r.into_parts();
    let id = parts.uri.path().to_string();

    // MyDe::extract4::<HeaderMap, i32, String, serde_json::Value, _, _, _, _>().await?;
    // let (header, date, name, age) = MyDe::new()
    //     .queue::<HeaderMap>()
    //     .queue::<i32>()
    //     .queue::<String>()
    //     .queue::<serde_json::Value>()
    //     .extract(&state, &mut parts, body, ("header", "date", "name", "age"))
    //     .await?;

    // let headers = HeaderMap::from_request_parts(&mut parts, &state).await?;

    todo!()
}

struct OurCustomBody {
    headers: HeaderMap,
    date: i32,
    name: String,
    age: serde_json::Value,
}

struct OurCustomBodyVanilla {
    headers: HeaderMap,
    body: Json<()>,
}

// trait OverloadedArguments<Mark, State, Args, This> {}

// impl<M, A, B, C, S, T> OverloadedArguments<M, S, (A, B, C), T> for (A, B, C) where
//     T: Handler<(M, A, B, C), S>
// {
// }

// struct MyMarker;
// impl<A, B, C, D> OverloadedArguments<MyMarker> for (A, B, C, D)
// where
//     A: DeserializeOwned,
//     B: DeserializeOwned,
//     C: DeserializeOwned,
//     D: DeserializeOwned,
// {
// }

// fn assert_known_handler<M, S>(t: impl Handler<(M, State<DioxusServerState>), S>) {}

// fn assert_overloaded<Args: OverloadedArguments<Mark, DioxusServerState, Args, This>, Mark, This>() {
// }
// fn assert_overloaded<Args: OverloadedArguments<Mark, State, Args, This>, Mark, State, This>() {}

fn it_works() {
    async fn handler1(state: State<DioxusServerState>) {}

    // assert_overloaded::<(State<DioxusServerState>, HeaderMap, String), _, _>();

    // assert_known_handler(handler1);
    // async fn handler2(a: i32, b: String, c: i32, d: i32) {}

    // assert_overloaded(handler1);
    // assert_overloaded((123, "hello".to_string(), 456, 789));
}

// trait CantDeserialize<T> {}
// impl<T, S, A, B, C, D, E, F, G, H> CantDeserialize<(T, S)> for (A, B, C, D, E, F, G, H) wher
// {
// }
// trait CantDeserialize<T> {}
// impl<T, S, A, B, C, D, E, F, G, H> CantDeserialize<(T, S)> for (A, B, C, D, E, F, G, H) where
//     A: Handler<T, S>
// {
// }

// trait IsAxumExtractor {}
// impl<S, T> IsAxumExtractor for T where T: FromRequestParts<S> {}

// fn hmm(body: &[u8]) {
//     impl<'de> Deserialize<'de> for OurCustomBody {
//         fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
//         where
//             D: serde::Deserializer<'de>,
//         {
//             // deserializer.deserialize_i32(visitor)
//             todo!()
//         }
//     }
// }

// // let de = serde_json::Deserializer::from_slice(body);

// struct SpecialDeserializer<M> {
//     _marker: std::marker::PhantomData<M>,
// }
// impl<
//         M1: DeserializeOrExtract,
//         M2: DeserializeOrExtract,
//         M3: DeserializeOrExtract,
//         M4: DeserializeOrExtract,
//     > SpecialDeserializer<(M1, M2, M3, M4)>
// {
//     async fn deserialize(
//         request: Request,
//         names: (&'static str, &'static str, &'static str, &'static str),
//     ) -> anyhow::Result<(M1::Out, M2::Out, M3::Out, M4::Out)> {
//         let (mut parts, _body) = request.into_parts();
//         let state = DioxusServerState::default();
//         let a = M1::deserialize_or_extract(&state, &mut parts).await?;
//         let b = M2::deserialize_or_extract(&state, &mut parts).await?;
//         let c = M3::deserialize_or_extract(&state, &mut parts).await?;
//         let d = M4::deserialize_or_extract(&state, &mut parts).await?;
//         Ok((a, b, c, d))
//     }
// }

// trait DeserializeOrExtract {
//     type Out;
//     async fn deserialize_or_extract(
//         state: &DioxusServerState,
//         parts: &mut Parts,
//     ) -> anyhow::Result<Self::Out>;
// }

// trait ExtractGroup {
//     type Names;
//     fn extract_group(r: Request, f: Self::Names);
// }

// impl<T> ExtractGroup for (T,) {
//     type Names = (&'static str,);
//     fn extract_group(r: Request, f: Self::Names) {
//         todo!()
//     }
// }

// #[derive(Deserialize)]
// struct WeirdThingImplementsBoth;
// impl<S> FromRequestParts<S> for WeirdThingImplementsBoth {
//     type Rejection = ServerFnRejection;

//     fn from_request_parts(
//         parts: &mut Parts,
//         state: &S,
//     ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send {
//         async move { todo!() }
//     }
// }

// fn it_works() {}

#[post("/api/user/{id}/?age")]
async fn update_user(
    id: i32,
    age: i32,
    headers: HeaderMap,
    state: State<DioxusServerState>,
    // date: i32,
    // name: String,
    // age: serde_json::Value,
) -> anyhow::Result<()> {
    Ok(())
}

// struct MyDe<Queue> {
//     _phantom: std::marker::PhantomData<Queue>,
// }
// impl MyDe<()> {
//     fn new() -> MyDe<()> {
//         MyDe {
//             _phantom: std::marker::PhantomData,
//         }
//     }
//     fn queue<T>(self, name: &'static str) -> MyDe<(T,)> {
//         MyDe {
//             _phantom: std::marker::PhantomData,
//         }
//     }
// }
// impl<P> MyDe<(P,)> {
//     fn queue<T>(self, name: &'static str) -> MyDe<(P, T)> {
//         MyDe {
//             _phantom: std::marker::PhantomData,
//         }
//     }
// }

// impl<P1, P2> MyDe<(P1, P2)> {
//     fn queue<T>(self, name: &'static str) -> MyDe<(P1, P2, T)> {
//         MyDe {
//             _phantom: std::marker::PhantomData,
//         }
//     }
// }

// impl<P1, P2, P3> MyDe<(P1, P2, P3)> {
//     fn queue<T>(self, name: &'static str) -> MyDe<(P1, P2, P3, T)> {
//         MyDe {
//             _phantom: std::marker::PhantomData,
//         }
//     }
// }

// impl<P1, P2, P3, P4, P5, P6, P7, P8> MyDe<(P1, P2, P3, P4, P5, P6, P7, P8)> {
//     fn extract<T>(self, name: &'static str) -> MyDe<(P1, P2, P3, P4, P5, P6, P7, P8, T)> {
//         MyDe {
//             _phantom: std::marker::PhantomData,
//         }
//     }
// }

// trait MyExtract<M> {
//     type Out;
// }

// struct ViaPartsMarker;
// impl<T> MyExtract<ViaPartsMarker> for T
// where
//     T: FromRequestParts<DioxusServerState>,
// {
//     type Out = i32;
// }

// struct ViaDeserializeMarker;
// impl<T> MyExtract<ViaDeserializeMarker> for T
// where
//     T: DeserializeOwned,
// {
//     type Out = i32;
// }

// impl MyDe {
//     async fn extract4<A, B, C, D, M1, M2, M3, M4>(
//     ) -> anyhow::Result<(A::Out, B::Out, C::Out, D::Out)>
//     where
//         A: MyExtract<M1>,
//         B: MyExtract<M2>,
//         C: MyExtract<M3>,
//         D: MyExtract<M4>,
//     {
//         todo!()
//     }
// }

// fn extract1<A>() -> () {}
// fn extract2<A, B>() -> (A::Out, B::Out)
// where
//     A: MyExtract,
//     B: MyExtract,
// {
//     todo!()
// }
// fn extract3<A, B, C>() {
//     todo!()
// }
// struct BundledBody {}

// impl<S> FromRequest<S> for BundledBody {
//     type Rejection = ServerFnRejection;

//     fn from_request(
//         req: Request,
//         state: &S,
//     ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send {
//         async move {
//             //
//             todo!()
//         }
//     }
// }

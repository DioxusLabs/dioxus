use axum::extract::{FromRequestParts, Request};
use dioxus_fullstack::DioxusServerState;
use http::HeaderMap;
use serde::{
    self as _serde,
    de::{DeserializeOwned, DeserializeSeed, SeqAccess, Visitor},
};

fn main() {}

pub struct Args {
    pub header: HeaderMap,
    pub name: String,
    pub age: u32,
}

// struct ArgsDeserializer {
//     request: Request,
// }

// impl<'de> DeserializeSeed<'de> for Args {
//     type Value = Args;

//     fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
//     where
//         D: serde::Deserializer<'de>,
//     {
//         struct Visitor {
//             request: Request,
//         }
//         impl<'de> serde::de::Visitor<'de> for Visitor {
//             type Value = Args;

//             fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
//                 todo!()
//             }

//             fn visit_seq<A>(mut self, mut seq: A) -> Result<Self::Value, A::Error>
//             where
//                 A: serde::de::SeqAccess<'de>,
//             {
//                 let header = match (&&ExtractMe::<HeaderMap>::new())
//                     .extract_it(&mut self.request, &mut seq).unwrap();

//                 let next = (&&ExtractMe::<String>::new()).extract_it(&mut self.request, &mut seq);

//                 todo!()
//             }

//             fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
//             where
//                 A: serde::de::MapAccess<'de>,
//             {
//                 todo!()
//             }
//         }
//         todo!()
//     }
// }

// struct ExtractMe<T>(std::marker::PhantomData<T>);
// impl<T> ExtractMe<T> {
//     fn new() -> Self {
//         ExtractMe(std::marker::PhantomData)
//     }
// }

// /// Pull things out of the request
// trait ExtractAsRequest<T> {
//     fn extract_it<'a>(&self, req: &mut Request, de: &mut impl SeqAccess<'a>) -> Option<T>;
// }
// impl<T> ExtractAsRequest<T> for ExtractMe<T>
// where
//     T: FromRequestParts<DioxusServerState>,
// {
//     fn extract_it<'a>(&self, req: &mut Request, de: &mut impl SeqAccess<'a>) -> Option<T> {
//         todo!()
//     }
// }

// trait ExtractAsSerde<T> {
//     fn extract_it<'a>(&self, req: &mut Request, de: &mut impl SeqAccess<'a>) -> Option<T>;
// }
// impl<T> ExtractAsSerde<T> for ExtractMe<T>
// where
//     T: DeserializeOwned,
// {
//     fn extract_it<'a>(&self, req: &mut Request, de: &mut impl SeqAccess<'a>) -> Option<T> {
//         todo!()
//     }
// }

// impl<'de> serde::Deserialize<'de> for Args {
//     fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
//     where
//         D: serde::Deserializer<'de>,
//     {
//         let mut __field0: _serde::__private::Option<HeaderMap> = _serde::__private::None;
//         let mut __field1: _serde::__private::Option<String> = _serde::__private::None;
//         let mut __field2: _serde::__private::Option<u32> = _serde::__private::None;

//         // serde::Deserializer::deserialize_struct(
//         //     deserializer,
//         //     "Args",
//         //     &["header", "name", "age"],
//         //     Visitor,
//         // )
//         // let mut __field0: _serde::__private::Option<String> = _serde::__private::None;
//         // let mut __field1: _serde::__private::Option<String> = _serde::__private::None;
//         // let mut __field2: _serde::__private::Option<i32> = _serde::__private::None;

//         // serde::Deserializer::deserialize_struct(
//         //     deserializer,
//         //     "Args",
//         //     &["header", "name", "age"],
//         //     Visitor,
//         // )
//     }
// }

use dioxus_fullstack_core::ServerFnError;
use http::StatusCode;
use serde::{de::DeserializeOwned, Serialize};

// pub trait TransportError: Sized {
//     fn from_reqwest_error(err: reqwest::Error) -> Self {
//         // let serverfn_err: ServerFnError = err.into();
//         // Self::from_serverfn_error(serverfn_err)
//         todo!()
//     }
//     fn from_serverfn_error(err: ServerFnError) -> Self;
// }

// // struct BlanketError;
// // impl<E> TransportError<BlanketError> for E
// // where
// //     E: From<ServerFnError> + DeserializeOwned + Serialize,
// // {
// //     fn from_serverfn_error(err: ServerFnError) -> Self {
// //         todo!()
// //     }
// // }

// // struct SpecificError;
// // impl TransportError<SpecificError> for StatusCode {
// //     fn from_serverfn_error(err: ServerFnError) -> Self {
// //         todo!()
// //     }
// // }

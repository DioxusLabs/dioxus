use std::any::TypeId;

use dioxus::fullstack::extract::State;
use dioxus::prelude::*;

fn main() {
    dioxus::serve(|| async move { todo!() });
}

// #[router("/auth")]
// async fn auth_provider() -> State<AuthProvider> {
//     dioxus_auth::Provider::new()
// }

// struct AuthProvider {}

// impl AuthProvider {
//     #[post("/login")]
//     async fn login(&self) -> Result<()> {
//         let p = TypeId::of::<Self>();
//         todo!()
//     }

//     // #[post("/logout")]
//     async fn logout() -> Result<()> {
//         todo!()
//     }
// }

// pub mod one {
//     pub mod two {
//         pub mod three {
//             pub fn do_it() -> &'static str {
//                 // https://doc.rust-lang.org/cargo/reference/environment-variables.html
//                 // file!(), line!(), column!(), module_path!(), crate_name!(), crate
//                 // env!("CARGO_MANIFEST_DIR")
//                 todo!()
//             }
//         }
//     }
// }

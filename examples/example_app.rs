//! Example App
//! --------------
//!
//! This example shows how to use the cross-platform abilities of dioxus to generate everything a dioxus app might need.
//! All of your apps will look like this.
//!
//! cargo run --features dioxus/static

use std::u32;

use dioxus::prelude::Context;

// #[allow(unused_lifetimes)]
#[derive(Debug, PartialEq, Hash)]
struct Pool<'a> {
    a: u32,
    b: &'a str,
}

struct UserData {}
type Query<In, Out> = fn(&Pool, In) -> Result<Out, ()>;
// type Query<In, Out> = fn(&Pool, In) -> Out;

static GET_USER: Query<String, Vec<UserData>> = |pool, name| {
    //
    let b = Ok(())?;
    let b = Ok(())?;
    let b = Ok(())?;
    let b = Ok(())?;
    let b = Ok(())?;
    let b = Ok(())?;
    todo!()
};

static SET_USER: Query<String, Vec<UserData>> = |pool, name| {
    //
    todo!()
};

fn main() {
    //     // returns a future
    //     let user_data = use_db(&ctx, GET_USER, || "Bill");

    //     use_try_suspense(&ctx, async move {
    //         match user_data.await? {
    //             Ok() => {}
    //             Err(err) => {}
    //         }
    //     })
    // }

    // fn use_try_suspense(ctx: &Context<()>) {
    //     let c: Result<(), ()> = {
    //         // let b = Ok(());
    //         // b?
    //     };
}

mod launches {
    #[cfg(feature = "wasm")]
    fn launch() {
        // launch the wasm_rednerer
    }

    #[cfg(feature = "static")]
    fn launch() {
        // render the tree to text
    }

    // #[cfg(features = "server")]
    // fn launch() {
    //     // launch the app
    // }

    // #[cfg(features = "liveview")]
    // fn launch() {
    //     // launch the app
    // }

    // #[cfg(features = "desktop")]
    // fn launch() {
    //     // launch the app
    // }

    // #[cfg(features = "android")]
    // fn launch() {
    //     // launch a simulator in dev mode
    // }

    // #[cfg(features = "ios")]
    // fn launch() {
    //     // launch a simulator in dev mode
    // }
}

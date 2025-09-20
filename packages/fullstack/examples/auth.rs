#![allow(non_snake_case)]

use std::{collections::HashMap, sync::OnceLock};

use dashmap::mapref::one::RefMut;
#[cfg(feature = "server")]
use dashmap::DashMap;
use dioxus::{prelude::*, server::ServerState};
use dioxus_fullstack::{HttpError, Streaming};
use http::StatusCode;

fn main() {
    dioxus::launch(|| app());
}

fn app() -> Element {
    let mut chat_response = use_signal(String::default);

    // let mut signup = use_action(move |()| async move { todo!() });

    rsx! {}
}

static COUNT: GlobalSignal<i32> = GlobalSignal::new(|| 123);

#[cfg(feature = "server")]
static DATABASE: ServerState<DashMap<String, String>> =
    ServerState::new(|| async move { DashMap::new() });

static DATABASE2: OnceLock<DashMap<String, String>> = OnceLock::new();

#[post("/api/signup")]
async fn signup(email: String, password: String) -> Result<()> {
    // DATABASE2.get()
    todo!()
}

static DB2: once_cell::race::OnceBox<DashMap<String, String>> = once_cell::race::OnceBox::new();

static DB3: once_cell::sync::Lazy<DashMap<String, String>> =
    once_cell::sync::Lazy::new(|| DashMap::new());

#[post("/api/login")]
async fn login(email: String, password: String) -> Result<()> {
    let res = DB2.get().unwrap();
    let res: RefMut<'static, String, String> = res.get_mut(&email).unwrap();
    DB3.insert(email, password);
    todo!()
}

#[link_section = "some-cool-section"]
pub extern "C" fn my_thing<T: 'static>() {
    println!(
        "hello from my_thing with type {}",
        std::any::type_name::<T>()
    );
}

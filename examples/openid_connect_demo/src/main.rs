#![allow(non_snake_case)]
use dioxus::prelude::*;
use fermi::*;
use gloo_storage::{LocalStorage, Storage};
use log::LevelFilter;
pub(crate) mod constants;
mod env;
pub(crate) mod model;
pub(crate) mod oidc;
pub(crate) mod router;
pub(crate) mod storage;
pub(crate) mod views;
use oidc::{AuthRequestState, AuthTokenState};
use router::Route;

use dioxus_router::prelude::*;

use crate::{
    constants::{DIOXUS_FRONT_AUTH_REQUEST, DIOXUS_FRONT_AUTH_TOKEN},
    oidc::ClientState,
};
pub static FERMI_CLIENT: fermi::AtomRef<ClientState> = AtomRef(|_| ClientState::default());
pub static FERMI_AUTH_TOKEN: fermi::AtomRef<AuthTokenState> =
    AtomRef(|_| AuthTokenState::default());
pub static FERMI_AUTH_REQUEST: fermi::AtomRef<AuthRequestState> =
    AtomRef(|_| AuthRequestState::default());

fn App(cx: Scope) -> Element {
    use_init_atom_root(cx);

    // Retrieve the value stored in the browser's storage
    let stored_auth_token = LocalStorage::get(DIOXUS_FRONT_AUTH_TOKEN)
        .ok()
        .unwrap_or(AuthTokenState::default());
    let fermi_auth_token = use_atom_ref(cx, &FERMI_AUTH_TOKEN);
    *fermi_auth_token.write() = stored_auth_token;

    let stored_auth_request = LocalStorage::get(DIOXUS_FRONT_AUTH_REQUEST)
        .ok()
        .unwrap_or(AuthRequestState::default());

    let fermi_auth_request = use_atom_ref(cx, &FERMI_AUTH_REQUEST);
    *fermi_auth_request.write() = stored_auth_request;
    render! { Router::<Route> {} }
}

fn main() {
    dioxus_logger::init(LevelFilter::Info).expect("failed to init logger");
    console_error_panic_hook::set_once();
    log::info!("starting app");
    dioxus_web::launch(App);
}

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
pub static FERMI_CLIENT: fermi::Atom<ClientState> = Atom(|_| ClientState { oidc_client: None });
pub static FERMI_AUTH_TOKEN: fermi::Atom<AuthTokenState> = Atom(|_| AuthTokenState {
    id_token: None,
    refresh_token: None,
});
pub static FERMI_AUTH_REQUEST: fermi::Atom<AuthRequestState> =
    Atom(|_| AuthRequestState { auth_request: None });

fn App(cx: Scope) -> Element {
    use_init_atom_root(cx);

    // Retrieve the value stored in the browser's storage
    let stored_auth_token =
        LocalStorage::get(DIOXUS_FRONT_AUTH_TOKEN)
            .ok()
            .unwrap_or(AuthTokenState {
                id_token: None,
                refresh_token: None,
            });
    let fermi_auth_token_write = use_set(cx, &FERMI_AUTH_TOKEN);
    fermi_auth_token_write(stored_auth_token);

    let stored_auth_request = LocalStorage::get(DIOXUS_FRONT_AUTH_REQUEST)
        .ok()
        .unwrap_or(AuthRequestState { auth_request: None });

    let fermi_auth_request_write = use_set(cx, &FERMI_AUTH_REQUEST);
    fermi_auth_request_write(stored_auth_request);
    render! { Router::<Route> {} }
}

fn main() {
    dioxus_logger::init(LevelFilter::Info).expect("failed to init logger");
    console_error_panic_hook::set_once();
    log::info!("starting app");
    dioxus_web::launch(App);
}

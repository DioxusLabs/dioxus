#![allow(non_snake_case)]
use dioxus::prelude::*;
use fermi::*;
use gloo_storage::{LocalStorage, Storage};
use log::LevelFilter;
pub(crate) mod constants;
pub(crate) mod errors;
pub(crate) mod model;
pub(crate) mod oidc;
pub(crate) mod props;
pub(crate) mod router;
pub(crate) mod storage;
pub(crate) mod views;
use oidc::{AuthRequestState, AuthTokenState};
use router::Route;

use crate::{
    constants::{DIOXUS_FRONT_AUTH_REQUEST, DIOXUS_FRONT_AUTH_TOKEN},
    oidc::ClientState,
};
pub static FERMI_CLIENT: fermi::AtomRef<ClientState> = AtomRef(|_| ClientState::default());

// An option is required to prevent the component from being constantly refreshed
pub static FERMI_AUTH_TOKEN: fermi::AtomRef<Option<AuthTokenState>> = AtomRef(|_| None);
pub static FERMI_AUTH_REQUEST: fermi::AtomRef<Option<AuthRequestState>> = AtomRef(|_| None);

pub static DIOXUS_FRONT_ISSUER_URL: &str = env!("DIOXUS_FRONT_ISSUER_URL");
pub static DIOXUS_FRONT_CLIENT_ID: &str = env!("DIOXUS_FRONT_CLIENT_ID");
pub static DIOXUS_FRONT_URL: &str = env!("DIOXUS_FRONT_URL");

fn App() -> Element {
    use_init_atom_root(cx);

    // Retrieve the value stored in the browser's storage
    let stored_auth_token = LocalStorage::get(DIOXUS_FRONT_AUTH_TOKEN)
        .ok()
        .unwrap_or(AuthTokenState::default());
    let fermi_auth_token = use_atom_ref(&FERMI_AUTH_TOKEN);
    if fermi_auth_token.read().is_none() {
        *fermi_auth_token.write() = Some(stored_auth_token);
    }

    let stored_auth_request = LocalStorage::get(DIOXUS_FRONT_AUTH_REQUEST)
        .ok()
        .unwrap_or(AuthRequestState::default());
    let fermi_auth_request = use_atom_ref(&FERMI_AUTH_REQUEST);
    if fermi_auth_request.read().is_none() {
        *fermi_auth_request.write() = Some(stored_auth_request);
    }
    rsx! { Router::<Route> {} }
}

fn main() {
    dioxus_logger::init(LevelFilter::Info).expect("failed to init logger");
    console_error_panic_hook::set_once();
    log::info!("starting app");
    dioxus_web::launch(App);
}

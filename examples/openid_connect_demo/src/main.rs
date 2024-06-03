#![allow(non_snake_case)]
use dioxus::prelude::*;
use dioxus_logger::tracing::Level;
use oidc::{AuthRequestState, AuthTokenState};
use router::Route;

use crate::{oidc::ClientState, storage::PersistentInit};

pub(crate) mod constants;
pub(crate) mod model;
pub(crate) mod oidc;
pub(crate) mod props;
pub(crate) mod router;
pub(crate) mod storage;
pub(crate) mod views;

pub static CLIENT: GlobalSignal<ClientState> = Signal::global(ClientState::default);

pub static AUTH_TOKEN: GlobalSignal<Option<AuthTokenState>> = Signal::global(|| None);
pub static AUTH_REQUEST: GlobalSignal<Option<AuthRequestState>> = Signal::global(|| None);

pub static DIOXUS_FRONT_ISSUER_URL: &str = env!("DIOXUS_FRONT_ISSUER_URL");
pub static DIOXUS_FRONT_CLIENT_ID: &str = env!("DIOXUS_FRONT_CLIENT_ID");
pub static DIOXUS_FRONT_CLIENT_SECRET: &str = env!("DIOXUS_FRONT_CLIENT_SECRET");
pub static DIOXUS_FRONT_URL: &str = env!("DIOXUS_FRONT_URL");

fn App() -> Element {
    AuthRequestState::persistent_init();
    AuthTokenState::persistent_init();
    rsx! { Router::<Route> {} }
}

fn main() {
    dioxus_logger::init(Level::DEBUG).expect("failed to init logger");
    dioxus_sdk::set_dir!();
    console_error_panic_hook::set_once();
    log::info!("starting app");
    launch(App);
}

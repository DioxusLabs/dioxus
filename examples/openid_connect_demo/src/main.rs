#![allow(non_snake_case)]
use dioxus::prelude::*;
use dioxus_logger::tracing::Level;
use router::Route;

use crate::oidc::ClientState;
use crate::storage::{use_auth_request_provider, use_auth_token_provider};

pub(crate) mod constants;
pub(crate) mod model;
pub(crate) mod oidc;
pub(crate) mod props;
pub(crate) mod router;
pub(crate) mod storage;
pub(crate) mod views;

pub static CLIENT: GlobalSignal<ClientState> = Signal::global(ClientState::default);

pub static DIOXUS_FRONT_ISSUER_URL: &str = env!("DIOXUS_FRONT_ISSUER_URL");
pub static DIOXUS_FRONT_CLIENT_ID: &str = env!("DIOXUS_FRONT_CLIENT_ID");
pub static DIOXUS_FRONT_CLIENT_SECRET: &str = env!("DIOXUS_FRONT_CLIENT_SECRET");
pub static DIOXUS_FRONT_URL: &str = env!("DIOXUS_FRONT_URL");

fn App() -> Element {
    use_auth_request_provider();
    use_auth_token_provider();
    rsx! { Router::<Route> {} }
}

fn main() {
    dioxus_logger::init(Level::DEBUG).expect("failed to init logger");
    dioxus_sdk::set_dir!();
    console_error_panic_hook::set_once();
    log::info!("starting app");
    launch(App);
}

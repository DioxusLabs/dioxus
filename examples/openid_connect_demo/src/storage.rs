use dioxus::prelude::*;
use dioxus_sdk::storage::*;

use crate::{
    constants::{DIOXUS_FRONT_AUTH_REQUEST, DIOXUS_FRONT_AUTH_TOKEN},
    oidc::{AuthRequestState, AuthTokenState},
};

pub fn use_auth_token_provider() {
    let stored_token =
        use_storage::<LocalStorage, _>(DIOXUS_FRONT_AUTH_TOKEN.to_owned(), AuthTokenState::default);

    use_context_provider(move || stored_token);
}

pub fn use_auth_token() -> Signal<AuthTokenState> {
    use_context()
}

pub fn use_auth_request_provider() {
    let stored_req = use_storage::<LocalStorage, _>(
        DIOXUS_FRONT_AUTH_REQUEST.to_owned(),
        AuthRequestState::default,
    );

    use_context_provider(move || stored_req);
}

pub fn use_auth_request() -> Signal<AuthRequestState> {
    use_context()
}

pub fn auth_request() -> Signal<AuthRequestState> {
    consume_context()
}

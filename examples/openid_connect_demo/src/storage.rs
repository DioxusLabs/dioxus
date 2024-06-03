use dioxus::prelude::*;
use dioxus_sdk::storage::*;

use crate::{
    constants::{DIOXUS_FRONT_AUTH_REQUEST, DIOXUS_FRONT_AUTH_TOKEN},
    oidc::{AuthRequestState, AuthTokenState},
    AUTH_REQUEST, AUTH_TOKEN,
};

pub trait PersistentInit {
    fn persistent_init();
}

impl PersistentInit for AuthTokenState {
    fn persistent_init() {
        let stored_token = use_storage::<LocalStorage, _>(
            DIOXUS_FRONT_AUTH_TOKEN.to_owned(),
            AuthTokenState::default,
        );
        use_effect(move || {
            *AUTH_TOKEN.write() = Some(stored_token());
        });
    }
}

impl PersistentInit for AuthRequestState {
    fn persistent_init() {
        let stored_req = use_storage::<LocalStorage, _>(
            DIOXUS_FRONT_AUTH_REQUEST.to_owned(),
            AuthRequestState::default,
        );
        use_effect(move || {
            *AUTH_REQUEST.write() = Some(stored_req());
        });
    }
}

pub trait PersistentWrite {
    fn persistent_set(entry: Self);
}

impl PersistentWrite for AuthTokenState {
    fn persistent_set(entry: AuthTokenState) {
        let mut stored_token = use_storage::<LocalStorage, _>(
            DIOXUS_FRONT_AUTH_TOKEN.to_string(),
            AuthTokenState::default,
        );
        Signal::<AuthTokenState>::set(&mut stored_token, entry);
    }
}

impl PersistentWrite for AuthRequestState {
    fn persistent_set(entry: AuthRequestState) {
        let mut stored_req = use_storage::<LocalStorage, _>(
            DIOXUS_FRONT_AUTH_REQUEST.to_string(),
            AuthRequestState::default,
        );
        *stored_req.write() = entry;
    }
}

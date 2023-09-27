use gloo_storage::{LocalStorage, Storage};
use serde::{Deserialize, Serialize};
use std::rc::Rc;

use crate::{
    constants::{DIOXUS_FRONT_AUTH_REQUEST, DIOXUS_FRONT_AUTH_TOKEN},
    oidc::{AuthRequestState, AuthTokenState},
};

#[derive(Serialize, Deserialize, Clone)]
pub struct StorageEntry<T> {
    pub key: String,
    pub value: T,
}

pub fn use_persistent_set<T: 'static + Serialize + Clone>(
    write_function: &Rc<dyn Fn(T)>,
    entry: StorageEntry<T>,
) {
    LocalStorage::set(entry.key, entry.value.clone()).unwrap();
    write_function(entry.value);
}

pub type AuthTokenEntry = StorageEntry<AuthTokenState>;

impl AuthTokenEntry {
    pub fn new(auth_token_state: AuthTokenState) -> Self {
        Self {
            key: DIOXUS_FRONT_AUTH_TOKEN.to_string(),
            value: auth_token_state,
        }
    }

    pub fn none() -> Self {
        Self {
            key: DIOXUS_FRONT_AUTH_TOKEN.to_string(),
            value: AuthTokenState::default(),
        }
    }
}

pub type AuthRequestEntry = StorageEntry<AuthRequestState>;

impl AuthRequestEntry {
    pub fn new(auth_token_state: AuthRequestState) -> Self {
        Self {
            key: DIOXUS_FRONT_AUTH_REQUEST.to_string(),
            value: auth_token_state,
        }
    }

    pub fn none() -> Self {
        Self {
            key: DIOXUS_FRONT_AUTH_REQUEST.to_string(),
            value: AuthRequestState::default(),
        }
    }
}

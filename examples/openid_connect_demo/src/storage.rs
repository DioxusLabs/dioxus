use fermi::UseAtomRef;
use gloo_storage::{LocalStorage, Storage};
use serde::{Deserialize, Serialize};

use crate::{
    constants::{DIOXUS_FRONT_AUTH_REQUEST, DIOXUS_FRONT_AUTH_TOKEN},
    oidc::{AuthRequestState, AuthTokenState},
};

#[derive(Serialize, Deserialize, Clone)]
pub struct StorageEntry<T> {
    pub key: String,
    pub value: T,
}

pub trait PersistentWrite<T: Serialize + Clone> {
    fn use_persistent_set(atom_ref: &UseAtomRef<T>, entry: T);
}

impl PersistentWrite<AuthTokenState> for AuthTokenState {
    fn use_persistent_set(atom_ref: &UseAtomRef<AuthTokenState>, entry: AuthTokenState) {
        atom_ref.write().id_token = entry.clone().id_token;
        atom_ref.write().refresh_token = entry.clone().refresh_token;
        LocalStorage::set(DIOXUS_FRONT_AUTH_TOKEN, entry).unwrap();
    }
}

impl PersistentWrite<AuthRequestState> for AuthRequestState {
    fn use_persistent_set(atom_ref: &UseAtomRef<AuthRequestState>, entry: AuthRequestState) {
        atom_ref.write().auth_request = entry.clone().auth_request;
        LocalStorage::set(DIOXUS_FRONT_AUTH_REQUEST, entry).unwrap();
    }
}

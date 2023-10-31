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
    fn persistent_set(atom_ref: &UseAtomRef<Option<T>>, entry: Option<T>);
}

impl PersistentWrite<AuthTokenState> for AuthTokenState {
    fn persistent_set(
        atom_ref: &UseAtomRef<Option<AuthTokenState>>,
        entry: Option<AuthTokenState>,
    ) {
        *atom_ref.write() = entry.clone();
        LocalStorage::set(DIOXUS_FRONT_AUTH_TOKEN, entry).unwrap();
    }
}

impl PersistentWrite<AuthRequestState> for AuthRequestState {
    fn persistent_set(
        atom_ref: &UseAtomRef<Option<AuthRequestState>>,
        entry: Option<AuthRequestState>,
    ) {
        *atom_ref.write() = entry.clone();
        LocalStorage::set(DIOXUS_FRONT_AUTH_REQUEST, entry).unwrap();
    }
}

//! Fixture crate for cross-crate visibility tests on `#[derive(Store)]`.
//!
//! Defines a public struct with fields at every visibility tier so downstream
//! fixtures can probe which accessors are reachable across a crate boundary.

use dioxus_stores::Store;

#[derive(Store)]
pub struct Item {
    pub public_field: i32,
    pub(crate) crate_field: i32,
    private_field: i32,
}

impl Item {
    pub fn new() -> Self {
        Self {
            public_field: 0,
            crate_field: 0,
            private_field: 0,
        }
    }
}

impl Default for Item {
    fn default() -> Self {
        Self::new()
    }
}

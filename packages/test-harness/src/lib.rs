//! A small libtest-compatible harness shared by native and web Dioxus tests.

use std::{future::Future, pin::Pin};

pub struct TestCase {
    pub name: &'static str,
    pub file: &'static str,
    pub line: u32,
    pub ignore: bool,
    pub should_panic: bool,
    pub timeout_ms: Option<u64>,
    pub tags: &'static [&'static str],
    pub run: fn() -> Pin<Box<dyn Future<Output = ()>>>,
}

inventory::collect!(TestCase);
pub use dioxus_test_harness_macro::test;
pub use inventory;

#[macro_export]
macro_rules! main {
    () => {
        fn main() {
            $crate::run();
        }
    };
}

#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(not(target_arch = "wasm32"))]
pub use native::run;

#[cfg(target_arch = "wasm32")]
mod web;
#[cfg(target_arch = "wasm32")]
pub use web::run;

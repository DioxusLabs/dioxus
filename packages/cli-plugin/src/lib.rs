#![doc = include_str!("../README.md")]
#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]
#![allow(unused_macros)]

use once_cell::sync::Lazy;

#[doc(hidden)]
pub use inventory;

use crate::exports::plugins::main::definitions::Guest;

wit_bindgen::generate!({
    path: "./wit",
    world: "plugin-world",
    exports: {
        world: ExportedDefinitions,
        "plugins:main/definitions": ExportedDefinitions
    },
});

pub trait DynGuest {
    fn on_rebuild(&self) -> bool;
    fn on_hot_reload(&self) -> bool;
}

#[doc(hidden)]
pub struct LazyGuest(fn() -> Box<dyn DynGuest + Send + Sync>);

impl LazyGuest {
    pub const fn new(constructor: fn() -> Box<dyn DynGuest + Send + Sync>) -> Self {
        Self(constructor)
    }
}

inventory::collect!(LazyGuest);

static EXPORTED_DEFINITIONS: Lazy<Box<dyn DynGuest + Send + Sync>> = Lazy::new(|| {
    (inventory::iter::<LazyGuest>
        .into_iter()
        .next()
        .expect("no plugin exported")
        .0)()
});

struct ExportedDefinitions;

impl Guest for ExportedDefinitions {
    fn on_hot_reload() {
        EXPORTED_DEFINITIONS.on_hot_reload();
    }

    fn on_rebuild() -> bool {
        EXPORTED_DEFINITIONS.on_rebuild()
    }
}

#[macro_export]
macro_rules! export_plugin {
    ($name:ident) => {
        $crate::inventory::submit! {
            $crate::LazyGuest::new(|| {
                Box::new($name)
            })
        }
    };
}

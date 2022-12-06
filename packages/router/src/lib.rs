pub(crate) mod contexts {
    pub(crate) mod router;
}

pub mod hooks {
    mod use_router;
    pub use use_router::*;
}

pub mod prelude {
    pub use dioxus_router_core::prelude::*;
}

pub mod components {
    mod outlet;
    pub use outlet::*;
}

mod contexts {
    pub(crate) mod router;
}

pub mod hooks {
    mod use_router;
    pub use use_router::*;
}

pub mod prelude {
    pub use dioxus_router_core::prelude::*;
}

mod utils {
    pub(crate) mod use_router_internal;
}

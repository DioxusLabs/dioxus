pub mod components {
    pub(crate) mod default_errors;

    mod link;
    pub use link::*;

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

    pub use crate::components::*;
    pub use crate::hooks::*;

    use dioxus::core::Component;
    pub fn comp(component: Component) -> ContentAtom<Component> {
        ContentAtom(component)
    }
}

mod utils {
    pub(crate) mod use_router_internal;
}

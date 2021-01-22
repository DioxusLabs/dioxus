pub mod prelude {
    pub use dioxus_core::prelude::*;
    pub use dioxus_core_macro::fc;
    pub use dioxus_hooks::prelude::use_state;
}

use dioxus_core::prelude::FC;

// Re-export core completely
pub use dioxus_core as core;

mod keys;
pub use keys::*;

macro_rules! impl_event {
    (
        $data:ty;
        $(
            $( #[$attr:meta] )*
            $name:ident
        )*
    ) => {
        $(
            $( #[$attr] )*
            pub fn $name<'a>(_cx: &'a ::dioxus_core::ScopeState, _f: impl FnMut(::dioxus_core::UiEvent<$data>) + 'a) -> ::dioxus_core::Attribute<'a> {
                todo!()
            }
        )*
    };
}

mod mouse;
pub use mouse::*;

mod animation;
pub use animation::*;

mod composition;
pub use composition::*;

mod drag;
pub use drag::*;

mod focus;
pub use focus::*;

mod form;
pub use form::*;

mod image;
pub use image::*;

mod keyboard;
pub use keyboard::*;

mod media;
pub use media::*;

mod pointer;
pub use pointer::*;

mod selection;
pub use selection::*;

mod toggle;
pub use toggle::*;

mod touch;
pub use touch::*;

mod transition;
pub use transition::*;

mod wheel;
pub use wheel::*;

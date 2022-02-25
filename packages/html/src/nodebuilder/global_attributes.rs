use super::ElementBuilder;
pub use crate::builder::IntoAttributeValue;
use dioxus_core::Attribute;

macro_rules! no_namespace_trait_methods {
    (
        $(
            $(#[$attr:meta])*
            $name:ident;
        )*
    ) => {
        $(
            $(#[$attr])*
            pub fn $name(mut self, val: impl IntoAttributeValue<'a>) -> Self {
                let (value, is_static) = val.into_str(self.fac);
                self.attrs.push(Attribute {
                    name: stringify!($name),
                    value,
                    is_static,
                    namespace: None,
                    is_volatile: false,
                });
                self
            }
        )*
    };
}

impl<'a, T> ElementBuilder<'a, T> {
    no_namespace_trait_methods! {
        /// accesskey
        accesskey;

        /// class
        class;

        /// contenteditable
        contenteditable;

        /// data
        data;

        /// dir
        dir;

        /// draggable
        draggable;

        /// hidden
        hidden;

        /// Set the value of the `id` attribute.
        id;

        /// lang
        lang;

        /// spellcheck
        spellcheck;

        /// style
        style;

        /// tabindex
        tabindex;

        /// title
        title;

        /// translate
        translate;

        /// role
        role;

        /// dangerous_inner_html
        dangerous_inner_html;
    }
}

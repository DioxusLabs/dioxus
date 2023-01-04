#[macro_export]
macro_rules! custom_elements {
    (
        $trait_impl:ident;
        $element_type:ident;

        $(
            $(#[$attr:meta])*
            $name:ident $namespace:tt {
                $(
                    $(#[$attr_method:meta])*
                    $fil:ident: $vil:ident $extra:tt,
                )*
            };
        )*

        [
            $(
                $(#[$attr_attr:meta])*
                $name_name:ident $(: $($arg:literal),*)*;
            )*
        ]

    ) => {

        /// The raw definition of an element
        ///
        /// This should (hopefully) be compiled away
        pub struct $element_type<E = ()> {
            pub tag: &'static str,
            pub namespace: Option<&'static str>,
            _t: std::marker::PhantomData<E>,
        }

        impl<E> $element_type<E> {
            pub const fn new(tag: &'static str, namespace: Option<&'static str>) -> Self {
                Self {
                    tag,
                    namespace,
                    _t: std::marker::PhantomData,
                }
            }
        }

        impl $trait_impl for dioxus_elements {}
        pub trait $trait_impl {
            $(
                const $name: $element_type<elements::$name> = $element_type::new(stringify!($name), None);
            )*
        }

        #[allow(non_snake_case, non_camel_case_types)]
        mod elements {
            use super::*;

            $(
                $(#[$attr])*
                pub struct $name;

                impl $element_type<$name> {
                    $(
                        $(#[$attr_method])*
                        pub fn $fil(&self) -> AttributeDescription {
                            AttributeDescription {
                                name: stringify!($fil),
                                namespace: None,
                                volatile: false,
                            }
                        }
                    )*
                }
            )*

            impl<T> $element_type<T> {
                $(
                    $(#[$attr_attr])*
                    pub fn $name_name(&self) -> AttributeDescription {
                        todo!()
                    }
                )*
            }
        }
    };
}

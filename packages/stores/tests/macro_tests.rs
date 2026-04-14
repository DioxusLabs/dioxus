//! Compile-time tests for the Store derive macro.
//!
//! These tests verify that the macro generates valid, well-typed code. They are not
//! executed at runtime—the test functions exist only to ensure the generated code compiles.
//! Visibility enforcement is validated via `compile_fail` doctests in `lib.rs`.

#[allow(unused)]
#[allow(clippy::disallowed_names)]
mod macro_tests {
    use dioxus_signals::*;
    use dioxus_stores::*;
    use std::collections::HashMap;

    fn derive_unit() {
        #[derive(Store)]
        struct TodoItem;
    }

    fn derive_struct() {
        #[derive(Store)]
        struct TodoItem {
            checked: bool,
            contents: String,
        }

        #[store]
        impl Store<TodoItem> {
            fn is_checked(&self) -> bool {
                self.checked().cloned()
            }

            fn check(&mut self) {
                self.checked().set(true);
            }
        }

        let mut store = use_store(|| TodoItem {
            checked: false,
            contents: "Learn about stores".to_string(),
        });

        // The store macro creates an extension trait with methods for each field
        // that returns a store scoped to that field.
        let checked: Store<bool, _> = store.checked();
        let contents: Store<String, _> = store.contents();
        let checked: bool = checked();
        let contents: String = contents();

        // It also generates a `transpose` method returns a variant of your structure
        // with stores wrapping each of your data types. This can be very useful when destructuring
        // or matching your type
        let TodoItemStoreTransposed { checked, contents } = store.transpose();
        let checked: bool = checked();
        let contents: String = contents();

        let is_checked = store.is_checked();
        store.check();
    }

    fn derive_generic_struct() {
        #[derive(Store)]
        struct Item<T> {
            checked: bool,
            contents: T,
        }

        #[store]
        impl<T> Store<Item<T>> {
            fn is_checked(&self) -> bool
            where
                T: 'static,
            {
                self.checked().cloned()
            }

            fn check(&mut self)
            where
                T: 'static,
            {
                self.checked().set(true);
            }
        }

        let mut store = use_store(|| Item {
            checked: false,
            contents: "Learn about stores".to_string(),
        });

        let checked: Store<bool, _> = store.checked();
        let contents: Store<String, _> = store.contents();
        let checked: bool = checked();
        let contents: String = contents();

        let ItemStoreTransposed { checked, contents } = store.transpose();
        let checked: bool = checked();
        let contents: String = contents();

        let is_checked = store.is_checked();
        store.check();
    }

    fn derive_generic_struct_with_bounds() {
        #[derive(Store)]
        struct Item<T: ?Sized>
        where
            T: 'static,
        {
            checked: bool,
            contents: &'static T,
        }

        #[store]
        impl<T: ?Sized + 'static> Store<Item<T>> {
            fn is_checked(&self) -> bool
            where
                T: 'static,
            {
                self.checked().cloned()
            }

            fn check(&mut self)
            where
                T: 'static,
            {
                self.checked().set(true);
            }
        }

        let mut store = use_store(|| Item {
            checked: false,
            contents: "Learn about stores",
        });

        let checked: Store<bool, _> = store.checked();
        let contents: Store<&'static str, _> = store.contents();
        let checked: bool = checked();
        let contents: &'static str = contents();

        let ItemStoreTransposed { checked, contents } = store.transpose();
        let checked: bool = checked();
        let contents: &'static str = contents();

        let is_checked = store.is_checked();
        store.check();
    }

    fn derive_generic_struct_with_private_field() {
        mod inner {
            use dioxus_stores::*;

            #[derive(Store)]
            pub struct Item<T> {
                pub visible: T,
                secret: T,
            }

            impl<T: Default> Item<T> {
                pub fn new() -> Self {
                    Self {
                        visible: T::default(),
                        secret: T::default(),
                    }
                }
            }

            pub fn touch_secret(store: Store<Item<i32>>) {
                // The private-field accessor is reachable here (same module).
                let _ = store.secret();
            }
        }

        use dioxus_signals::*;
        use dioxus_stores::*;
        use inner::{Item, ItemStoreExt};

        let store = use_store(Item::<i32>::new);
        let _: Store<i32, _> = store.visible();
        inner::touch_secret(store);
    }

    fn derive_generic_struct_transposed_passthrough() {
        #[derive(Store)]
        struct Item<const COUNT: usize, T> {
            contents: T,
        }

        let mut store = use_store(|| Item::<0, _> {
            contents: "Learn about stores".to_string(),
        });

        let Item { contents } = store.transpose();
        let contents: String = contents();
    }

    fn derive_tuple() {
        #[derive(Store, PartialEq, Clone, Debug)]
        struct Item(bool, String);

        let store = use_store(|| Item(true, "Hello".to_string()));

        let first = store.field_0();
        let first: bool = first();

        let transposed = store.transpose();
        let first = transposed.0;
        let second = transposed.1;
        let first: bool = first();
        let second: String = second();
    }

    fn derive_enum() {
        #[derive(Store, PartialEq, Clone, Debug)]
        #[non_exhaustive]
        enum Enum {
            Foo,
            Bar(String),
            Baz { foo: i32, bar: String },
            FooBar(u32, String),
            BarFoo { foo: String },
        }

        #[store]
        impl Store<Enum> {
            fn is_foo_or_bar(&self) -> bool {
                matches!(self.cloned(), Enum::Foo | Enum::Bar(_))
            }

            fn make_foo(&mut self) {
                self.set(Enum::Foo);
            }
        }

        let mut store = use_store(|| Enum::Bar("Hello".to_string()));

        let foo = store.is_foo();
        let bar = store.is_bar();
        let baz = store.is_baz();
        let foobar = store.is_foo_bar();
        let barfoo = store.is_bar_foo();

        let foo = store.bar().unwrap();
        let foo: String = foo();
        let bar = store.bar_foo().unwrap();
        let bar: String = bar();

        let transposed = store.transpose();
        use EnumStoreTransposed::*;
        match transposed {
            EnumStoreTransposed::Foo => {}
            Bar(bar) => {
                let bar: String = bar();
            }
            Baz { foo, bar } => {
                let foo: i32 = foo();
                let bar: String = bar();
            }
            FooBar(foo, bar) => {
                let foo: u32 = foo();
                let bar: String = bar();
            }
            BarFoo { foo } => {
                let foo: String = foo();
            }
        }

        let is_foo_or_bar = store.is_foo_or_bar();
        store.make_foo();
    }

    fn derive_generic_enum() {
        #[derive(Store, PartialEq, Clone, Debug)]
        #[non_exhaustive]
        enum Enum<T> {
            Foo,
            Bar(T),
            Baz { foo: i32, bar: T },
            FooBar(u32, T),
            BarFoo { foo: T },
        }

        #[store]
        impl<T> Store<Enum<T>> {
            fn is_foo_or_bar(&self) -> bool
            where
                T: Clone + 'static,
            {
                matches!(self.cloned(), Enum::Foo | Enum::Bar(_))
            }

            fn make_foo(&mut self)
            where
                T: 'static,
            {
                self.set(Enum::Foo);
            }
        }

        let mut store = use_store(|| Enum::Bar("Hello".to_string()));

        let foo = store.is_foo();
        let bar = store.is_bar();
        let baz = store.is_baz();
        let foobar = store.is_foo_bar();
        let barfoo = store.is_bar_foo();

        let foo = store.bar().unwrap();
        let foo: String = foo();
        let bar = store.bar_foo().unwrap();
        let bar: String = bar();

        let transposed = store.transpose();
        use EnumStoreTransposed::*;
        match transposed {
            EnumStoreTransposed::Foo => {}
            Bar(bar) => {
                let bar: String = bar();
            }
            Baz { foo, bar } => {
                let foo: i32 = foo();
                let bar: String = bar();
            }
            FooBar(foo, bar) => {
                let foo: u32 = foo();
                let bar: String = bar();
            }
            BarFoo { foo } => {
                let foo: String = foo();
            }
        }

        let is_foo_or_bar = store.is_foo_or_bar();
        store.make_foo();
    }

    fn derive_generic_enum_with_bounds() {
        #[derive(Store, PartialEq, Clone, Debug)]
        #[non_exhaustive]
        enum Enum<T: ?Sized>
        where
            T: 'static,
        {
            Foo,
            Bar(&'static T),
            Baz { foo: i32, bar: &'static T },
            FooBar(u32, &'static T),
            BarFoo { foo: &'static T },
        }

        #[store]
        impl<T: ?Sized + 'static> Store<Enum<T>> {
            fn is_foo_or_bar(&self) -> bool
            where
                T: Clone + 'static,
            {
                matches!(self.cloned(), Enum::Foo | Enum::Bar(_))
            }

            fn make_foo(&mut self)
            where
                T: 'static,
            {
                self.set(Enum::Foo);
            }
        }

        let mut store = use_store(|| Enum::Bar("Hello"));

        let foo = store.is_foo();
        let bar = store.is_bar();
        let baz = store.is_baz();
        let foobar = store.is_foo_bar();
        let barfoo = store.is_bar_foo();

        let foo = store.bar().unwrap();
        let foo: &'static str = foo();
        let bar = store.bar_foo().unwrap();
        let bar: &'static str = bar();

        let transposed = store.transpose();
        use EnumStoreTransposed::*;
        match transposed {
            EnumStoreTransposed::Foo => {}
            Bar(bar) => {
                let bar: &'static str = bar();
            }
            Baz { foo, bar } => {
                let foo: i32 = foo();
                let bar: &'static str = bar();
            }
            FooBar(foo, bar) => {
                let foo: u32 = foo();
                let bar: &'static str = bar();
            }
            BarFoo { foo } => {
                let foo: &'static str = foo();
            }
        }
    }

    // Named struct with mixed visibility: pub field + private field
    fn derive_struct_private_fields() {
        #[derive(Store)]
        pub struct Item {
            pub name: String,
            count: u32,
        }

        let store = use_store(|| Item {
            name: "hello".to_string(),
            count: 42,
        });

        // Public field is accessible through the public extension trait
        let name: Store<String, _> = store.name();
        let name: String = name();

        // Private field is accessible through the private extension trait
        // (within this module, both traits are in scope)
        let count: Store<u32, _> = store.count();
        let count: u32 = count();

        // Transpose is on the public trait and returns a struct with the original field visibility
        let transposed = store.transpose();
        let _name: Store<String, _> = transposed.name;
        // count is private in the transposed struct, but accessible within this module
        let _count: Store<u32, _> = transposed.count;
    }

    // Tuple struct with mixed visibility
    fn derive_tuple_private_fields() {
        #[derive(Store, PartialEq, Clone, Debug)]
        pub struct Item(pub bool, String);

        let store = use_store(|| Item(true, "Hello".to_string()));

        // Public field accessible through public trait
        let first = store.field_0();
        let first: bool = first();

        // Private field accessible through private trait (within this module)
        let second = store.field_1();
        let second: String = second();

        // Transpose works and preserves field visibility
        let transposed = store.transpose();
        let _first = transposed.0;
        let _second = transposed.1;
    }

    // All fields private on a pub struct: public trait only has transpose,
    // all field accessors go on the private trait
    fn derive_struct_all_private_fields() {
        #[derive(Store)]
        pub struct Item {
            name: String,
            count: u32,
        }

        let store = use_store(|| Item {
            name: "hello".to_string(),
            count: 42,
        });

        // Both fields are on the private trait (accessible within this module)
        let name: Store<String, _> = store.name();
        let count: Store<u32, _> = store.count();
        let name: String = name();
        let count: u32 = count();

        // Transpose is still on the public trait
        let transposed = store.transpose();
        let _name: Store<String, _> = transposed.name;
        let _count: Store<u32, _> = transposed.count;
    }

    // Private struct with no explicit field visibility: all fields go on
    // the single public-to-the-struct-level trait (no private trait generated)
    fn derive_private_struct_no_split() {
        #[derive(Store)]
        struct Item {
            name: String,
            count: u32,
        }

        let store = use_store(|| Item {
            name: "hello".to_string(),
            count: 42,
        });

        // Both fields go on the single trait since the struct is private
        let name: Store<String, _> = store.name();
        let count: Store<u32, _> = store.count();
        let name: String = name();
        let count: u32 = count();

        let transposed = store.transpose();
        let _name: Store<String, _> = transposed.name;
        let _count: Store<u32, _> = transposed.count;
    }

    // Generic struct with private fields: generics should work across both traits
    fn derive_generic_struct_private_fields() {
        #[derive(Store)]
        pub struct Item<T> {
            pub visible: T,
            hidden: bool,
        }

        let store = use_store(|| Item {
            visible: "hello".to_string(),
            hidden: true,
        });

        let visible: Store<String, _> = store.visible();
        let hidden: Store<bool, _> = store.hidden();
        let visible: String = visible();
        let hidden: bool = hidden();

        let ItemStoreTransposed { visible, hidden } = store.transpose();
        let visible: String = visible();
        let hidden: bool = hidden();
    }

    // Generic struct with bounds and private fields
    fn derive_generic_struct_with_bounds_private_fields() {
        #[derive(Store)]
        pub struct Item<T: ?Sized>
        where
            T: 'static,
        {
            pub visible: &'static T,
            hidden: bool,
        }

        let store = use_store(|| Item {
            visible: "hello",
            hidden: true,
        });

        let visible: Store<&'static str, _> = store.visible();
        let hidden: Store<bool, _> = store.hidden();
        let visible: &'static str = visible();
        let hidden: bool = hidden();

        let ItemStoreTransposed { visible, hidden } = store.transpose();
        let visible: &'static str = visible();
        let hidden: bool = hidden();
    }

    // #[store] impl blocks work alongside private field splitting
    fn derive_struct_private_fields_with_store_impl() {
        #[derive(Store)]
        pub struct Item {
            pub name: String,
            count: u32,
        }

        #[store]
        impl Store<Item> {
            fn name_len(&self) -> usize {
                self.name().to_string().len()
            }

            fn increment_count(&mut self) {
                let count = self.count().cloned();
                self.count().set(count + 1);
            }
        }

        let mut store = use_store(|| Item {
            name: "hello".to_string(),
            count: 42,
        });

        let _len = store.name_len();
        store.increment_count();
    }

    // pub(crate) fields on a pub struct go on the crate trait (pub(crate) visibility)
    fn derive_struct_pub_crate_fields() {
        #[derive(Store)]
        pub struct Item {
            pub(crate) name: String,
            pub count: u32,
        }

        let store = use_store(|| Item {
            name: "hello".to_string(),
            count: 42,
        });

        // pub(crate) field is on crate trait, pub field is on public trait
        // (all traits in scope within this crate)
        let name: Store<String, _> = store.name();
        let count: Store<u32, _> = store.count();
        let name: String = name();
        let count: u32 = count();

        let transposed = store.transpose();
        let _name: Store<String, _> = transposed.name;
        let _count: Store<u32, _> = transposed.count;
    }

    // Multiple private fields alongside multiple public fields
    fn derive_struct_many_mixed_fields() {
        #[derive(Store)]
        pub struct Config {
            pub title: String,
            pub enabled: bool,
            secret_key: String,
            internal_count: u64,
        }

        let store = use_store(|| Config {
            title: "App".to_string(),
            enabled: true,
            secret_key: "shhh".to_string(),
            internal_count: 0,
        });

        // Public fields
        let title: Store<String, _> = store.title();
        let enabled: Store<bool, _> = store.enabled();

        // Private fields
        let secret: Store<String, _> = store.secret_key();
        let count: Store<u64, _> = store.internal_count();

        // Transpose includes all fields
        let ConfigStoreTransposed {
            title,
            enabled,
            secret_key,
            internal_count,
        } = store.transpose();
        let _: String = title();
        let _: bool = enabled();
        let _: String = secret_key();
        let _: u64 = internal_count();
    }

    // Cross-module test: public fields are accessible from outside the defining module
    // via the public extension trait, while the private trait stays module-private
    mod inner_module {
        use dioxus_signals::*;
        use dioxus_stores::*;

        #[derive(Store)]
        pub struct Config {
            pub title: String,
            pub enabled: bool,
            secret: String,
        }

        impl Config {
            pub fn new() -> Self {
                Config {
                    title: "App".to_string(),
                    enabled: true,
                    secret: "hidden".to_string(),
                }
            }
        }

        // The private trait is accessible within the defining module
        fn _inner_access() {
            let store = use_store(Config::new);
            let _secret: Store<String, _> = store.secret();
        }
    }

    fn derive_cross_module_public_access() {
        use inner_module::*;

        let store = use_store(inner_module::Config::new);

        // Public fields are accessible from outside the module
        let title: Store<String, _> = store.title();
        let enabled: Store<bool, _> = store.enabled();
        let _: String = title();
        let _: bool = enabled();

        // Transpose is accessible (it's on the public trait)
        let transposed = store.transpose();
        let _title: Store<String, _> = transposed.title;
        let _enabled: Store<bool, _> = transposed.enabled;
        // transposed.secret is private — cannot be accessed here
    }

    // Arbitrary `pub(super)` and `pub(in path)` visibilities propagate to the
    // generated accessor — callable from exactly the scope that could have
    // named the field itself.
    mod arbitrary_vis_parent {
        use dioxus_signals::*;
        use dioxus_stores::*;

        pub mod defining {
            use dioxus_signals::*;
            use dioxus_stores::*;

            #[derive(Store)]
            pub struct Item {
                pub open: String,
                pub(super) visible_to_parent: u32,
                pub(in crate::macro_tests::arbitrary_vis_parent) visible_to_parent_by_path: u64,
            }

            impl Item {
                pub fn new() -> Self {
                    Self {
                        open: "hi".into(),
                        visible_to_parent: 0,
                        visible_to_parent_by_path: 0,
                    }
                }
            }

            // All three callable from the defining module.
            fn _defining_can_call_all() {
                let store = use_store(Item::new);
                let _: Store<String, _> = store.open();
                let _: Store<u32, _> = store.visible_to_parent();
                let _: Store<u64, _> = store.visible_to_parent_by_path();
            }
        }

        // The parent module can reach the two restricted accessors.
        fn _parent_can_call_restricted() {
            use defining::ItemStoreExt;
            let store = use_store(defining::Item::new);
            let _: Store<String, _> = store.open();
            let _: Store<u32, _> = store.visible_to_parent();
            let _: Store<u64, _> = store.visible_to_parent_by_path();
        }
    }

    fn derive_generic_enum_transpose_passthrough() {
        #[derive(Store, PartialEq, Clone, Debug)]
        #[non_exhaustive]
        enum Enum<const COUNT: usize, T> {
            Foo,
            Bar(T),
            BarFoo { foo: T },
        }

        let mut store = use_store(|| Enum::<0, _>::Bar("Hello".to_string()));

        let transposed = store.transpose();
        use Enum::*;
        match transposed {
            Enum::Foo => {}
            Bar(bar) => {
                let bar: String = bar();
            }
            BarFoo { foo } => {
                let foo: String = foo();
            }
        }
    }
}

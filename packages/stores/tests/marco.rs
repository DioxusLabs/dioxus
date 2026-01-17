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

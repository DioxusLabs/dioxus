#[allow(unused)]
mod macro_tests {
    use dioxus_stores::*;
    use std::collections::HashMap;

    #[derive(Store, PartialEq, Clone, Debug)]
    struct TodoState {
        todos: HashMap<u32, TodoItem>,
        filter: FilterState,
    }

    #[derive(PartialEq, Eq, Clone, Copy, Debug)]
    enum FilterState {
        All,
        Active,
        Completed,
    }

    #[derive(Store, PartialEq, Clone, Debug)]
    struct TodoItem {
        checked: bool,
        contents: String,
    }

    fn access_todos(store: Store<TodoItem>) {
        let checked = store.checked();
        let contents = store.contents();
        let checked: bool = checked();
        let contents: String = contents();

        let transposed = store.transpose();
        let checked = transposed.checked;
        let contents = transposed.contents;
        let checked: bool = checked();
        let contents: String = contents();
    }

    #[derive(Store, PartialEq, Clone, Debug)]
    struct Item(bool, String);

    fn access_todos_tuple(store: Store<Item>) {
        let first = store.field_0();
        let first: bool = first();

        let transposed = store.transpose();
        let first = transposed.0;
        let second = transposed.1;
        let first: bool = first();
        let second: String = second();
    }

    #[derive(Store, PartialEq, Clone, Debug)]
    #[non_exhaustive]
    enum Enum {
        Foo,
        Bar(String),
        Baz { foo: i32, bar: String },
        FooBar(u32, String),
        BarFoo { foo: String },
    }

    fn access_enum(store: Store<Enum>) {
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
    }
}

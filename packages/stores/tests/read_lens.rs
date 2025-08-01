use dioxus_stores::*;

fn read_lens() {
    #[derive(Store)]
    struct TodoItem {
        checked: bool,
        contents: String,
    }

    let store = use_store(|| TodoItem {
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
}

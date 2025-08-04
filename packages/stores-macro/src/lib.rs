use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput, ItemImpl};

use crate::extend::ExtendArgs;

mod derive;
mod extend;

/// # `derive(Store)`
///
/// The `Store` macro is used to create an extension trait for stores that makes it possible to access the fields or variants
/// of an item as stores.
///
/// ## Expansion
///
/// The macro expands to two different items:
/// - An extension trait which is implemented for `Store<YourType, W>` with methods to access fields and variants for your type.
/// - A transposed version of your type which contains the fields or variants as stores.
///
/// ### Structs
///
/// For structs, the store macro generates methods for each field that returns a store scoped to that field and a `transpose` method that returns a struct with all fields as stores:
///
/// ```rust, no_run
/// use dioxus::prelude::*;
/// use dioxus_stores::*;
///
/// #[derive(Store)]
/// struct TodoItem {
///     checked: bool,
///     contents: String,
/// }
///
/// let store = use_store(|| TodoItem {
///     checked: false,
///     contents: "Learn about stores".to_string(),
/// });
///
/// // The store macro creates an extension trait with methods for each field
/// // that returns a store scoped to that field.
/// let checked: Store<bool, _> = store.checked();
/// let contents: Store<String, _> = store.contents();
///
/// // It also generates a `transpose` method returns a variant of your structure
/// // with stores wrapping each of your data types. This can be very useful when destructuring
/// // or matching your type
/// let TodoItemStoreTransposed { checked, contents } = store.transpose();
/// let checked: bool = checked();
/// let contents: String = contents();
/// ```
///
///
/// ### Enums
///
/// For enums, the store macro generates methods for each variant that checks if the store is that variant. It also generates a `transpose` method that returns an enum with all fields as stores.
///
/// ```rust, no_run
/// use dioxus::prelude::*;
/// use dioxus_stores::*;
///
/// #[derive(Store, PartialEq, Clone, Debug)]
/// enum Enum {
///     Foo(String),
///     Bar { foo: i32, bar: String },
/// }
///
/// let store = use_store(|| Enum::Foo("Hello".to_string()));
/// // The store macro creates an extension trait with methods for each variant to check
/// // if the store is that variant.
/// let foo: bool = store.is_foo();
/// let bar: bool = store.is_bar();
///
/// // If there is only one field in the variant, it also generates a method to try
/// // to downcast the store to that variant.
/// let foo: Option<Store<String, _>> = store.foo();
/// if let Some(foo) = foo {
///     println!("Foo: {foo}");
/// }
///
/// // It also generates a `transpose` method that returns a variant of your enum where all
/// // the fields are stores. You can use this to match your enum
/// let transposed = store.transpose();
/// use EnumStoreTransposed::*;
/// match transposed {
///     EnumStoreTransposed::Foo(foo) => println!("Foo: {foo}"),
///     EnumStoreTransposed::Bar { foo, bar } => {
///         let foo: i32 = foo();
///         let bar: String = bar();
///         println!("Bar: foo = {foo}, bar = {bar}");
///     }
/// }
/// ```
#[proc_macro_derive(Store)]
pub fn derive_store(input: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let input = parse_macro_input!(input as DeriveInput);

    let expanded = match derive::derive_store(input) {
        Ok(tokens) => tokens,
        Err(err) => {
            // If there was an error, return it as a compile error
            return err.to_compile_error().into();
        }
    };

    // Hand the output tokens back to the compiler
    TokenStream::from(expanded)
}

/// # `#[store]`
///
/// The `store` attribute macro is used to create an extension trait for store implementations. The extension traits lets you add
/// methods to the store even though the type is not defined in your crate.
///
/// ## Arguments
///
/// - `pub`: Makes the generated extension trait public. If not provided, the trait will be private.
/// - `name = YourExtensionName`: The name of the extension trait. If not provided, it will be generated based on the type name.
///
/// ## Bounds
///
/// The generated extension trait will have bounds on the lens generic parameter to ensure it implements `Readable` or `Writable` as needed.
///
/// - If a method accepts `&self`, the lens will require `Readable` which lets you read the value of the store.
/// - If a method accepts `&mut self`, the lens will require `Writable` which lets you change the value of the store.
///
/// ## Example
///
/// ```rust, no_run
/// use dioxus::prelude::*;
/// use dioxus_stores::*;
///
/// #[derive(Store)]
/// struct TodoItem {
///     checked: bool,
///     contents: String,
/// }
///
/// // You can use the store attribute macro to add methods to your stores
/// #[store]
/// impl<Lens> Store<TodoItem, Lens> {
///    // Since this method takes &mut self, the lens will require Writable automatically. It cannot be used
///    // with ReadStore<TodoItem>
///    fn toggle_checked(&mut self) {
///        self.checked().toggle();
///    }
///
///    // Since this method takes &self, the lens will require Readable automatically
///    fn checked_contents(&self) -> Option<String> {
///        self.checked().cloned().then(|| self.contents().to_string())
///    }
/// }
///
/// let mut store = use_store(|| TodoItem {
///     checked: false,
///     contents: "Learn about stores".to_string(),
/// });
///
/// // You can use the methods defined in the extension trait
/// store.toggle_checked();
/// let contents: Option<String> = store.checked_contents();
/// ```
#[proc_macro_attribute]
pub fn store(args: TokenStream, input: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let args = parse_macro_input!(args as ExtendArgs);
    let input = parse_macro_input!(input as ItemImpl);

    let expanded = match extend::extend_store(args, input) {
        Ok(tokens) => tokens,
        Err(err) => {
            // If there was an error, return it as a compile error
            return err.to_compile_error().into();
        }
    };

    // Hand the output tokens back to the compiler
    TokenStream::from(expanded)
}

use proc_macro2::TokenStream;

use syn::parse::Error;
use syn::spanned::Spanned;
use syn::{parse_macro_input, DeriveInput};

use quote::quote;

mod field_info;
mod struct_info;
mod util;

/// `TypedBuilder` is not a real type - deriving it will generate a `::builder()` method on your
/// struct that will return a compile-time checked builder. Set the fields using setters with the
/// same name as the struct's fields and call `.build()` when you are done to create your object.
///
/// Trying to set the same fields twice will generate a compile-time error. Trying to build without
/// setting one of the fields will also generate a compile-time error - unless that field is marked
/// as `#[builder(default)]`, in which case the `::default()` value of it's type will be picked. If
/// you want to set a different default, use `#[builder(default=...)]`.
///
/// # Examples
///
/// ```
/// use typed_builder::TypedBuilder;
///
/// #[derive(PartialEq, TypedBuilder)]
/// struct Foo {
///     // Mandatory Field:
///     x: i32,
///
///     // #[default] without parameter - use the type's default
///     // #[builder(setter(strip_option))] - wrap the setter argument with `Some(...)`
///     #[builder(default, setter(strip_option))]
///     y: Option<i32>,
///
///     // Or you can set the default
///     #[builder(default=20)]
///     z: i32,
/// }
///
/// assert!(
///     Foo::builder().x(1).y(2).z(3).build()
///     == Foo { x: 1, y: Some(2), z: 3, });
///
/// // Change the order of construction:
/// assert!(
///     Foo::builder().z(1).x(2).y(3).build()
///     == Foo { x: 2, y: Some(3), z: 1, });
///
/// // Optional fields are optional:
/// assert!(
///     Foo::builder().x(1).build()
///     == Foo { x: 1, y: None, z: 20, });
///
/// // This will not compile - because we did not set x:
/// // Foo::builder().build();
///
/// // This will not compile - because we set y twice:
/// // Foo::builder().x(1).y(2).y(3);
/// ```
///
/// # Customisation with attributes
///
/// In addition to putting `#[derive(TypedBuilder)]` on a type, you can specify a `#[builder(…)]`
/// attribute on the type, and on any fields in it.
///
/// On the **type**, the following values are permitted:
///
/// - `doc`: enable documentation of the builder type. By default, the builder type is given
///   `#[doc(hidden)]`, so that the `builder()` method will show `FooBuilder` as its return type,
///   but it won’t be a link. If you turn this on, the builder type and its `build` method will get
///   sane defaults. The field methods on the builder will be undocumented by default.
///
/// - `builder_method_doc = "…"` replaces the default documentation that will be generated for the
///   `builder()` method of the type for which the builder is being generated.
///
/// - `builder_type_doc = "…"` replaces the default documentation that will be generated for the
///   builder type. Setting this implies `doc`.
///
/// - `build_method_doc = "…"` replaces the default documentation that will be generated for the
///   `build()` method of the builder type. Setting this implies `doc`.
///
/// - `field_defaults(...)` is structured like the `#[builder(...)]` attribute you can put on the
///   fields and sets default options for fields of the type. If specific field need to revert some
///   options to the default defaults they can prepend `!` to the option they need to revert, and
///   it would ignore the field defaults for that option in that field.
///    ```
///    use typed_builder::TypedBuilder;
///
///    #[derive(TypedBuilder)]
///    #[builder(field_defaults(default, setter(strip_option)))]
///    struct Foo {
///        // Defaults to None, options-stripping is performed:
///        x: Option<i32>,
///
///        // Defaults to 0, option-stripping is not performed:
///        #[builder(setter(!strip_option))]
///        y: i32,
///
///        // Defaults to Some(13), option-stripping is performed:
///        #[builder(default = Some(13))]
///        z: Option<i32>,
///    }
///    ```
///
/// On each **field**, the following values are permitted:
///
/// - `default`: make the field optional, defaulting to `Default::default()`. This requires that
///   the field type implement `Default`. Mutually exclusive with any other form of default.
///
/// - `default = …`: make the field optional, defaulting to the expression `…`.
///
/// - `default_code = "…"`: make the field optional, defaulting to the expression `…`. Note that
///    you need to enclose it in quotes, which allows you to use it together with other custom
///    derive proc-macro crates that complain about "expected literal".
///    Note that if `...` contains a string, you can use raw string literals to avoid escaping the
///    double quotes - e.g. `#[builder(default_code = r#""default text".to_owned()"#)]`.
///
/// - `setter(...)`: settings for the field setters. The following values are permitted inside:
///
///   - `doc = "…"`: sets the documentation for the field’s setter on the builder type. This will be
///     of no value unless you enable docs for the builder type with `#[builder(doc)]` or similar on
///     the type.
///
///   - `skip`: do not define a method on the builder for this field. This requires that a default
///     be set.
///
///   - `into`: automatically convert the argument of the setter method to the type of the field.
///     Note that this conversion interferes with Rust's type inference and integer literal
///     detection, so this may reduce ergonomics if the field type is generic or an unsigned integer.
///
///   - `strip_option`: for `Option<...>` fields only, this makes the setter wrap its argument with
///     `Some(...)`, relieving the caller from having to do this. Note that with this setting on
///     one cannot set the field to `None` with the setter - so the only way to get it to be `None`
///     is by using `#[builder(default)]` and not calling the field's setter.
#[proc_macro_derive(TypedBuilder, attributes(builder))]
pub fn derive_typed_builder(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match impl_my_derive(&input) {
        Ok(output) => output.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

fn impl_my_derive(ast: &syn::DeriveInput) -> Result<TokenStream, Error> {
    let data = match &ast.data {
        syn::Data::Struct(data) => match &data.fields {
            syn::Fields::Named(fields) => {
                let struct_info = struct_info::StructInfo::new(&ast, fields.named.iter())?;
                let builder_creation = struct_info.builder_creation_impl()?;
                let conversion_helper = struct_info.conversion_helper_impl()?;
                let fields = struct_info
                    .included_fields()
                    .map(|f| struct_info.field_impl(f))
                    .collect::<Result<Vec<_>, _>>()?;
                let fields = quote!(#(#fields)*).into_iter();
                let required_fields = struct_info
                    .included_fields()
                    .filter(|f| f.builder_attr.default.is_none())
                    .map(|f| struct_info.required_field_impl(f))
                    .collect::<Result<Vec<_>, _>>()?;
                let build_method = struct_info.build_method_impl();

                quote! {
                    #builder_creation
                    #conversion_helper
                    #( #fields )*
                    #( #required_fields )*
                    #build_method
                }
            }
            syn::Fields::Unnamed(_) => {
                return Err(Error::new(
                    ast.span(),
                    "TypedBuilder is not supported for tuple structs",
                ))
            }
            syn::Fields::Unit => {
                return Err(Error::new(
                    ast.span(),
                    "TypedBuilder is not supported for unit structs",
                ))
            }
        },
        syn::Data::Enum(_) => {
            return Err(Error::new(
                ast.span(),
                "TypedBuilder is not supported for enums",
            ))
        }
        syn::Data::Union(_) => {
            return Err(Error::new(
                ast.span(),
                "TypedBuilder is not supported for unions",
            ))
        }
    };
    Ok(data)
}

// It’d be nice for the compilation tests to live in tests/ with the rest, but short of pulling in
// some other test runner for that purpose (e.g. compiletest_rs), rustdoc compile_fail in this
// crate is all we can use.

#[doc(hidden)]
/// When a property is skipped, you can’t set it:
/// (“method `y` not found for this”)
///
/// ```compile_fail
/// use typed_builder::TypedBuilder;
///
/// #[derive(PartialEq, TypedBuilder)]
/// struct Foo {
///     #[builder(default, setter(skip))]
///     y: i8,
/// }
///
/// let _ = Foo::builder().y(1i8).build();
/// ```
///
/// But you can build a record:
///
/// ```
/// use typed_builder::TypedBuilder;
///
/// #[derive(PartialEq, TypedBuilder)]
/// struct Foo {
///     #[builder(default, setter(skip))]
///     y: i8,
/// }
///
/// let _ = Foo::builder().build();
/// ```
///
/// `skip` without `default` is disallowed:
/// (“error: #[builder(skip)] must be accompanied by default”)
///
/// ```compile_fail
/// use typed_builder::TypedBuilder;
///
/// #[derive(PartialEq, TypedBuilder)]
/// struct Foo {
///     #[builder(setter(skip))]
///     y: i8,
/// }
/// ```
///
/// `clone` does not work if non-Clone fields have already been set
///
/// ```compile_fail
/// use typed_builder::TypedBuilder;
///
/// #[derive(Default)]
/// struct Uncloneable;
///
/// #[derive(TypedBuilder)]
/// struct Foo {
///     x: Uncloneable,
///     y: i32,
/// }
///
/// let _ = Foo::builder().x(Uncloneable).clone();
/// ```
///
/// Same, but with generics
///
/// ```compile_fail
/// use typed_builder::TypedBuilder;
///
/// #[derive(Default)]
/// struct Uncloneable;
///
/// #[derive(TypedBuilder)]
/// struct Foo<T> {
///     x: T,
///     y: i32,
/// }
///
/// let _ = Foo::builder().x(Uncloneable).clone();
/// ```
fn _compile_fail_tests() {}

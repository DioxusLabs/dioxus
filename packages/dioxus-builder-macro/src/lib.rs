//! # Dioxus Builder Macro
//!
//! Provides the `#[derive(BuilderProps)]` macro that eliminates boilerplate
//! for component props that use bon::Builder.

use convert_case::{Case, Casing};
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Error, FnArg, Ident, ItemFn, Pat};

/// Derive macro that generates `Properties` and `IntoDynNode` implementations
/// for props structs that use `bon::Builder`.
///
/// # Usage
///
/// ```rust,ignore
/// use bon::Builder;
/// use dioxus_builder::BuilderProps;
///
/// #[derive(Builder, Clone, PartialEq, BuilderProps)]
/// #[builder_props(component = MyCoolComponent)]
/// struct MyComponentProps {
///     #[builder(into)]
///     title: String,
///     count: Signal<i32>,
/// }
///
/// #[allow(non_snake_case)]
/// fn MyCoolComponent(props: MyComponentProps) -> Element {
///     // ...
/// }
/// ```
///
/// This generates:
/// - `impl Properties for MyComponentProps` with builder() and memoize() methods
/// - `impl IntoDynNode for MyComponentPropsBuilder<S>` where S: IsComplete
#[proc_macro_derive(BuilderProps, attributes(builder_props))]
pub fn derive_builder_props(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match derive_builder_props_impl(input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

fn derive_builder_props_impl(input: DeriveInput) -> Result<proc_macro2::TokenStream, Error> {
    let struct_name = &input.ident;

    // Find the #[builder_props(component = ComponentName)] attribute
    let component_name = find_component_attr(&input)?;

    // Generate the builder type name (e.g., MyComponentProps -> MyComponentPropsBuilder)
    let builder_name = Ident::new(
        &format!("{}Builder", struct_name),
        struct_name.span(),
    );

    // Generate the builder module name (snake_case + _builder)
    let module_name = Ident::new(
        &format!("{}_builder", struct_name.to_string().to_case(Case::Snake)),
        struct_name.span(),
    );

    let expanded = quote! {
        impl ::dioxus_core::Properties for #struct_name {
            type Builder = #builder_name;

            fn builder() -> Self::Builder {
                #struct_name::builder()
            }

            fn memoize(&mut self, other: &Self) -> bool {
                self == other
            }
        }

        impl<S> ::dioxus_core::IntoDynNode for #builder_name<S>
        where
            S: #module_name::IsComplete,
        {
            fn into_dyn_node(self) -> ::dioxus_core::DynamicNode {
                ::dioxus_core::IntoDynNode::into_dyn_node(#component_name(self.build()))
            }
        }
    };

    Ok(expanded)
}

fn find_component_attr(input: &DeriveInput) -> Result<Ident, Error> {
    for attr in &input.attrs {
        if attr.path().is_ident("builder_props") {
            let mut component_name: Option<Ident> = None;
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("component") {
                    let value: Ident = meta.value()?.parse()?;
                    component_name = Some(value);
                    return Ok(());
                }

                Err(meta.error(
                    "Unsupported builder_props attribute; expected `component = Name`",
                ))
            })?;

            if let Some(name) = component_name {
                return Ok(name);
            }
        }
    }

    Err(Error::new_spanned(
        &input.ident,
        "Missing #[builder_props(component = ComponentName)] attribute. \
         BuilderProps requires specifying the component function name.",
    ))
}

/// Helper to convert PascalCase to snake_case
fn to_snake_case(s: &str) -> String {
    s.to_case(Case::Snake)
}

/// Attribute macro that generates a full builder-compatible component.
///
/// Transforms a function into a component with auto-generated props struct.
///
/// # Usage
///
/// ```rust,ignore
/// use dioxus_builder::builder_component;
///
/// #[builder_component]
/// fn Counter(initial: i32, #[builder(into)] label: String) -> Element {
///     let count = use_signal(|| initial);
///     div().text(format!("{}: {}", label, count())).build()
/// }
/// // Generates: CounterProps { initial: i32, label: String }
/// // With full Properties + IntoDynNode implementations
/// ```
#[proc_macro_attribute]
pub fn builder_component(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let func = parse_macro_input!(item as ItemFn);

    match builder_component_impl(func) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

fn builder_component_impl(func: ItemFn) -> Result<proc_macro2::TokenStream, Error> {
    let fn_name = &func.sig.ident;
    let fn_vis = &func.vis;
    let fn_block = &func.block;

    // Generate names
    let props_name = Ident::new(&format!("{}Props", fn_name), fn_name.span());
    let builder_name = Ident::new(&format!("{}PropsBuilder", fn_name), fn_name.span());
    let builder_mod = Ident::new(
        &format!("{}_props_builder", to_snake_case(&fn_name.to_string())),
        fn_name.span(),
    );

    // Extract function parameters as struct fields
    let mut field_names = Vec::new();
    let mut field_types = Vec::new();
    let mut field_attrs: Vec<Vec<_>> = Vec::new();
    let mut field_patterns: Vec<proc_macro2::TokenStream> = Vec::new();

    for arg in &func.sig.inputs {
        if let FnArg::Typed(pat_type) = arg {
            // Extract field name from pattern
            match pat_type.pat.as_ref() {
                Pat::Ident(pat_ident) => {
                    if pat_ident.subpat.is_some() {
                        return Err(Error::new_spanned(
                            &pat_type.pat,
                            "builder_component does not support subpatterns in parameters",
                        ));
                    }
                    field_names.push(&pat_ident.ident);
                    field_types.push(&pat_type.ty);
                    field_patterns.push(quote! { #pat_ident });
                    // Collect #[builder(...)] attributes
                    let builder_attrs: Vec<_> = pat_type
                        .attrs
                        .iter()
                        .filter(|a| a.path().is_ident("builder"))
                        .collect();
                    field_attrs.push(builder_attrs);
                }
                _ => {
                    return Err(Error::new_spanned(
                        &pat_type.pat,
                        "builder_component only supports identifier parameters (e.g. `value: Type`)",
                    ));
                }
            }
        }
    }

    let expanded = quote! {
        #[derive(::bon::Builder, Clone, PartialEq)]
        #fn_vis struct #props_name {
            #(
                #(#field_attrs)*
                pub #field_names: #field_types,
            )*
        }

        impl ::dioxus_core::Properties for #props_name {
            type Builder = #builder_name;

            fn builder() -> Self::Builder {
                #props_name::builder()
            }

            fn memoize(&mut self, other: &Self) -> bool {
                self == other
            }
        }

        impl<S> ::dioxus_core::IntoDynNode for #builder_name<S>
        where
            S: #builder_mod::IsComplete,
        {
            fn into_dyn_node(self) -> ::dioxus_core::DynamicNode {
                ::dioxus_core::IntoDynNode::into_dyn_node(#fn_name(self.build()))
            }
        }

        #[allow(non_snake_case)]
        #fn_vis fn #fn_name(props: #props_name) -> ::dioxus_core::Element {
            let #props_name { #(#field_patterns),* } = props;
            #fn_block
        }
    };

    Ok(expanded)
}

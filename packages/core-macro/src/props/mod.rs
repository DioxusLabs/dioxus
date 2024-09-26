//! This code mostly comes from idanarye/rust-typed-builder
//!
//! However, it has been adopted to fit the Dioxus Props builder pattern.
//!
//! For Dioxus, we make a few changes:
//! - [x] Automatically implement [`Into<Option>`] on the setters (IE the strip setter option)
//! - [x] Automatically implement a default of none for optional fields (those explicitly wrapped with [`Option<T>`])

use proc_macro2::TokenStream;

use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{parse::Error, PathArguments};

use quote::quote;
use syn::{parse_quote, GenericArgument, PathSegment, Type};

pub fn impl_my_derive(ast: &syn::DeriveInput) -> Result<TokenStream, Error> {
    let data = match &ast.data {
        syn::Data::Struct(data) => match &data.fields {
            syn::Fields::Named(fields) => {
                let struct_info = struct_info::StructInfo::new(ast, fields.named.iter())?;
                let builder_creation = struct_info.builder_creation_impl()?;
                let conversion_helper = struct_info.conversion_helper_impl()?;
                let fields = struct_info
                    .included_fields()
                    .map(|f| struct_info.field_impl(f))
                    .collect::<Result<Vec<_>, _>>()?;
                let extends = struct_info
                    .extend_fields()
                    .map(|f| struct_info.extends_impl(f))
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
                    #( #extends )*
                    #( #required_fields )*
                    #build_method
                }
            }
            syn::Fields::Unnamed(_) => {
                return Err(Error::new(
                    ast.span(),
                    "Props is not supported for tuple structs",
                ))
            }
            syn::Fields::Unit => {
                return Err(Error::new(
                    ast.span(),
                    "Props is not supported for unit structs",
                ))
            }
        },
        syn::Data::Enum(_) => {
            return Err(Error::new(ast.span(), "Props is not supported for enums"))
        }
        syn::Data::Union(_) => {
            return Err(Error::new(ast.span(), "Props is not supported for unions"))
        }
    };
    Ok(data)
}

mod util {
    use quote::ToTokens;

    pub fn path_to_single_string(path: &syn::Path) -> Option<String> {
        if path.leading_colon.is_some() {
            return None;
        }
        let mut it = path.segments.iter();
        let segment = it.next()?;
        if it.next().is_some() {
            // Multipart path
            return None;
        }
        if segment.arguments != syn::PathArguments::None {
            return None;
        }
        Some(segment.ident.to_string())
    }

    pub fn expr_to_single_string(expr: &syn::Expr) -> Option<String> {
        if let syn::Expr::Path(path) = expr {
            path_to_single_string(&path.path)
        } else {
            None
        }
    }

    pub fn ident_to_type(ident: syn::Ident) -> syn::Type {
        let mut path = syn::Path {
            leading_colon: None,
            segments: Default::default(),
        };
        path.segments.push(syn::PathSegment {
            ident,
            arguments: Default::default(),
        });
        syn::Type::Path(syn::TypePath { qself: None, path })
    }

    pub fn empty_type() -> syn::Type {
        syn::TypeTuple {
            paren_token: Default::default(),
            elems: Default::default(),
        }
        .into()
    }

    pub fn type_tuple(elems: impl Iterator<Item = syn::Type>) -> syn::TypeTuple {
        let mut result = syn::TypeTuple {
            paren_token: Default::default(),
            elems: elems.collect(),
        };
        if !result.elems.empty_or_trailing() {
            result.elems.push_punct(Default::default());
        }
        result
    }

    pub fn empty_type_tuple() -> syn::TypeTuple {
        syn::TypeTuple {
            paren_token: Default::default(),
            elems: Default::default(),
        }
    }

    pub fn make_punctuated_single<T, P: Default>(value: T) -> syn::punctuated::Punctuated<T, P> {
        let mut punctuated = syn::punctuated::Punctuated::new();
        punctuated.push(value);
        punctuated
    }

    pub fn modify_types_generics_hack<F>(
        ty_generics: &syn::TypeGenerics,
        mut mutator: F,
    ) -> syn::AngleBracketedGenericArguments
    where
        F: FnMut(&mut syn::punctuated::Punctuated<syn::GenericArgument, syn::token::Comma>),
    {
        let mut abga: syn::AngleBracketedGenericArguments =
            syn::parse(ty_generics.clone().into_token_stream().into()).unwrap_or_else(|_| {
                syn::AngleBracketedGenericArguments {
                    colon2_token: None,
                    lt_token: Default::default(),
                    args: Default::default(),
                    gt_token: Default::default(),
                }
            });
        mutator(&mut abga.args);
        abga
    }

    pub fn strip_raw_ident_prefix(mut name: String) -> String {
        if name.starts_with("r#") {
            name.replace_range(0..2, "");
        }
        name
    }
}

mod field_info {
    use crate::props::type_from_inside_option;
    use proc_macro2::TokenStream;
    use quote::quote;
    use syn::spanned::Spanned;
    use syn::{parse::Error, punctuated::Punctuated};
    use syn::{parse_quote, Expr, Path};

    use super::util::{
        expr_to_single_string, ident_to_type, path_to_single_string, strip_raw_ident_prefix,
    };

    #[derive(Debug)]
    pub struct FieldInfo<'a> {
        pub ordinal: usize,
        pub name: &'a syn::Ident,
        pub generic_ident: syn::Ident,
        pub ty: &'a syn::Type,
        pub builder_attr: FieldBuilderAttr,
    }

    impl<'a> FieldInfo<'a> {
        pub fn new(
            ordinal: usize,
            field: &syn::Field,
            field_defaults: FieldBuilderAttr,
        ) -> Result<FieldInfo, Error> {
            if let Some(ref name) = field.ident {
                let mut builder_attr = field_defaults.with(&field.attrs)?;

                // children field is automatically defaulted to None
                if name == "children" {
                    builder_attr.default =
                        Some(syn::parse(quote!(dioxus_core::VNode::empty()).into()).unwrap());
                }

                // String fields automatically use impl Display
                if field.ty == parse_quote!(::std::string::String)
                    || field.ty == parse_quote!(std::string::String)
                    || field.ty == parse_quote!(string::String)
                    || field.ty == parse_quote!(String)
                {
                    builder_attr.from_displayable = true;
                    // ToString is both more general and provides a more useful error message than From<String>. If the user tries to use `#[into]`, use ToString instead.
                    if builder_attr.auto_into {
                        builder_attr.auto_to_string = true;
                    }
                    builder_attr.auto_into = false;
                }

                // extended field is automatically empty
                if !builder_attr.extends.is_empty() {
                    builder_attr.default = Some(
                        syn::parse(quote!(::core::default::Default::default()).into()).unwrap(),
                    );
                }

                // auto detect optional
                let strip_option_auto = builder_attr.strip_option
                    || !builder_attr.ignore_option && type_from_inside_option(&field.ty).is_some();
                if !builder_attr.strip_option && strip_option_auto {
                    builder_attr.strip_option = true;
                    builder_attr.default = Some(
                        syn::parse(quote!(::core::default::Default::default()).into()).unwrap(),
                    );
                }

                Ok(FieldInfo {
                    ordinal,
                    name,
                    generic_ident: syn::Ident::new(
                        &format!("__{}", strip_raw_ident_prefix(name.to_string())),
                        name.span(),
                    ),
                    ty: &field.ty,
                    builder_attr,
                })
            } else {
                Err(Error::new(field.span(), "Nameless field in struct"))
            }
        }

        pub fn generic_ty_param(&self) -> syn::GenericParam {
            syn::GenericParam::Type(self.generic_ident.clone().into())
        }

        pub fn type_ident(&self) -> syn::Type {
            ident_to_type(self.generic_ident.clone())
        }

        pub fn tuplized_type_ty_param(&self) -> syn::Type {
            let mut types = syn::punctuated::Punctuated::default();
            types.push(self.ty.clone());
            types.push_punct(Default::default());
            syn::TypeTuple {
                paren_token: Default::default(),
                elems: types,
            }
            .into()
        }
    }

    #[derive(Debug, Default, Clone)]
    pub struct FieldBuilderAttr {
        pub default: Option<syn::Expr>,
        pub docs: Vec<syn::Attribute>,
        pub skip: bool,
        pub auto_into: bool,
        pub auto_to_string: bool,
        pub from_displayable: bool,
        pub strip_option: bool,
        pub ignore_option: bool,
        pub extends: Vec<Path>,
    }

    impl FieldBuilderAttr {
        pub fn with(mut self, attrs: &[syn::Attribute]) -> Result<Self, Error> {
            let mut skip_tokens = None;
            for attr in attrs {
                if attr.path().is_ident("doc") {
                    self.docs.push(attr.clone());
                    continue;
                }

                if path_to_single_string(attr.path()).as_deref() != Some("props") {
                    continue;
                }

                match &attr.meta {
                    syn::Meta::List(list) => {
                        if list.tokens.is_empty() {
                            continue;
                        }
                    }
                    _ => {
                        continue;
                    }
                }

                let as_expr = attr.parse_args_with(
                    Punctuated::<Expr, syn::Token![,]>::parse_separated_nonempty,
                )?;

                for expr in as_expr.into_iter() {
                    self.apply_meta(expr)?;
                }

                // Stash its span for later (we don’t yet know if it’ll be an error)
                if self.skip && skip_tokens.is_none() {
                    skip_tokens = Some(attr.meta.clone());
                }
            }

            if self.skip && self.default.is_none() {
                return Err(Error::new_spanned(
                    skip_tokens.unwrap(),
                    "#[props(skip)] must be accompanied by default or default_code",
                ));
            }

            Ok(self)
        }

        pub fn apply_meta(&mut self, expr: syn::Expr) -> Result<(), Error> {
            match expr {
                // #[props(default = "...")]
                syn::Expr::Assign(assign) => {
                    let name = expr_to_single_string(&assign.left)
                        .ok_or_else(|| Error::new_spanned(&assign.left, "Expected identifier"))?;
                    match name.as_str() {
                        "extends" => {
                            if let syn::Expr::Path(path) = *assign.right {
                                self.extends.push(path.path);
                                Ok(())
                            } else {
                                Err(Error::new_spanned(
                                    assign.right,
                                    "Expected simple identifier",
                                ))
                            }
                        }
                        "default" => {
                            self.default = Some(*assign.right);
                            Ok(())
                        }
                        "default_code" => {
                            if let syn::Expr::Lit(syn::ExprLit {
                                lit: syn::Lit::Str(code),
                                ..
                            }) = *assign.right
                            {
                                use std::str::FromStr;
                                let tokenized_code = TokenStream::from_str(&code.value())?;
                                self.default = Some(
                                    syn::parse(tokenized_code.into())
                                        .map_err(|e| Error::new_spanned(code, format!("{e}")))?,
                                );
                            } else {
                                return Err(Error::new_spanned(assign.right, "Expected string"));
                            }
                            Ok(())
                        }
                        _ => Err(Error::new_spanned(
                            &assign,
                            format!("Unknown parameter {name:?}"),
                        )),
                    }
                }

                // #[props(default)]
                syn::Expr::Path(path) => {
                    let name = path_to_single_string(&path.path)
                        .ok_or_else(|| Error::new_spanned(&path, "Expected identifier"))?;
                    match name.as_str() {
                        "default" => {
                            self.default = Some(
                                syn::parse(quote!(::core::default::Default::default()).into())
                                    .unwrap(),
                            );
                            Ok(())
                        }

                        "optional" => {
                            self.default = Some(
                                syn::parse(quote!(::core::default::Default::default()).into())
                                    .unwrap(),
                            );
                            self.strip_option = true;
                            Ok(())
                        }

                        "extend" => {
                            self.extends.push(path.path);
                            Ok(())
                        }

                        _ => {
                            macro_rules! handle_fields {
                                ( $( $flag:expr, $field:ident, $already:expr; )* ) => {
                                    match name.as_str() {
                                        $(
                                            $flag => {
                                                if self.$field {
                                                    Err(Error::new(path.span(), concat!("Illegal setting - field is already ", $already)))
                                                } else {
                                                    self.$field = true;
                                                    Ok(())
                                                }
                                            }
                                        )*
                                        _ => Err(Error::new_spanned(
                                                &path,
                                                format!("Unknown setter parameter {:?}", name),
                                        ))
                                    }
                                }
                            }
                            handle_fields!(
                                "skip", skip, "skipped";
                                "into", auto_into, "calling into() on the argument";
                                "displayable", from_displayable, "calling to_string() on the argument";
                                "strip_option", strip_option, "putting the argument in Some(...)";
                            )
                        }
                    }
                }

                syn::Expr::Unary(syn::ExprUnary {
                    op: syn::UnOp::Not(_),
                    expr,
                    ..
                }) => {
                    if let syn::Expr::Path(path) = *expr {
                        let name = path_to_single_string(&path.path)
                            .ok_or_else(|| Error::new_spanned(&path, "Expected identifier"))?;
                        match name.as_str() {
                            "default" => {
                                self.default = None;
                                Ok(())
                            }
                            "skip" => {
                                self.skip = false;
                                Ok(())
                            }
                            "auto_into" => {
                                self.auto_into = false;
                                Ok(())
                            }
                            "displayable" => {
                                self.from_displayable = false;
                                Ok(())
                            }
                            "optional" => {
                                self.strip_option = false;
                                self.ignore_option = true;
                                Ok(())
                            }
                            _ => Err(Error::new_spanned(path, "Unknown setting".to_owned())),
                        }
                    } else {
                        Err(Error::new_spanned(
                            expr,
                            "Expected simple identifier".to_owned(),
                        ))
                    }
                }
                _ => Err(Error::new_spanned(expr, "Expected (<...>=<...>)")),
            }
        }
    }
}

fn type_from_inside_option(ty: &Type) -> Option<&Type> {
    let Type::Path(type_path) = ty else {
        return None;
    };

    if type_path.qself.is_some() {
        return None;
    }

    let path = &type_path.path;
    let seg = path.segments.last()?;

    // If the segment is a supported optional type, provide the inner type.
    // Return the inner type if the pattern is `Option<T>` or `ReadOnlySignal<Option<T>>``
    if seg.ident == "ReadOnlySignal" {
        // Get the inner type. E.g. the `u16` in `ReadOnlySignal<u16>` or `Option` in `ReadOnlySignal<Option<bool>>`
        let inner_type = extract_inner_type_from_segment(seg)?;
        let Type::Path(inner_path) = inner_type else {
            // If it isn't a path, the inner type isn't option
            return None;
        };

        // If we're entering an `Option`, we must get the innermost type
        let inner_seg = inner_path.path.segments.last()?;
        if inner_seg.ident == "Option" {
            // Get the innermost type.
            let innermost_type = extract_inner_type_from_segment(inner_seg)?;
            return Some(innermost_type);
        }
    } else if seg.ident == "Option" {
        // Grab the inner time. E.g. Option<u16>
        let inner_type = extract_inner_type_from_segment(seg)?;
        return Some(inner_type);
    }

    None
}

// Extract the inner type from a path segment.
fn extract_inner_type_from_segment(segment: &PathSegment) -> Option<&Type> {
    let PathArguments::AngleBracketed(generic_args) = &segment.arguments else {
        return None;
    };

    let GenericArgument::Type(final_type) = generic_args.args.first()? else {
        return None;
    };

    Some(final_type)
}

mod struct_info {
    use convert_case::{Case, Casing};
    use proc_macro2::TokenStream;
    use quote::quote;
    use syn::parse::Error;
    use syn::punctuated::Punctuated;
    use syn::spanned::Spanned;
    use syn::{parse_quote, Expr, Ident};

    use crate::props::strip_option;

    use super::field_info::{FieldBuilderAttr, FieldInfo};
    use super::util::{
        empty_type, empty_type_tuple, expr_to_single_string, make_punctuated_single,
        modify_types_generics_hack, path_to_single_string, strip_raw_ident_prefix, type_tuple,
    };
    use super::{child_owned_type, looks_like_callback_type, looks_like_signal_type};

    #[derive(Debug)]
    pub struct StructInfo<'a> {
        pub vis: &'a syn::Visibility,
        pub name: &'a syn::Ident,
        pub generics: &'a syn::Generics,
        pub fields: Vec<FieldInfo<'a>>,

        pub builder_attr: TypeBuilderAttr,
        pub builder_name: syn::Ident,
        pub conversion_helper_trait_name: syn::Ident,
        #[allow(unused)]
        pub core: syn::Ident,
    }

    impl<'a> StructInfo<'a> {
        pub fn included_fields(&self) -> impl Iterator<Item = &FieldInfo<'a>> {
            self.fields
                .iter()
                .filter(|f| !f.builder_attr.skip && f.builder_attr.extends.is_empty())
        }

        pub fn extend_fields(&self) -> impl Iterator<Item = &FieldInfo<'a>> {
            self.fields
                .iter()
                .filter(|f| !f.builder_attr.extends.is_empty())
        }

        pub fn new(
            ast: &'a syn::DeriveInput,
            fields: impl Iterator<Item = &'a syn::Field>,
        ) -> Result<StructInfo<'a>, Error> {
            let builder_attr = TypeBuilderAttr::new(&ast.attrs)?;
            let builder_name = strip_raw_ident_prefix(format!("{}Builder", ast.ident));
            Ok(StructInfo {
                vis: &ast.vis,
                name: &ast.ident,
                generics: &ast.generics,
                fields: fields
                    .enumerate()
                    .map(|(i, f)| FieldInfo::new(i, f, builder_attr.field_defaults.clone()))
                    .collect::<Result<_, _>>()?,
                builder_attr,
                builder_name: syn::Ident::new(&builder_name, ast.ident.span()),
                conversion_helper_trait_name: syn::Ident::new(
                    &format!("{builder_name}_Optional"),
                    ast.ident.span(),
                ),
                core: syn::Ident::new(&format!("{builder_name}_core"), ast.ident.span()),
            })
        }

        fn modify_generics<F: FnMut(&mut syn::Generics)>(&self, mut mutator: F) -> syn::Generics {
            let mut generics = self.generics.clone();
            mutator(&mut generics);
            generics
        }

        /// Checks if the props have any fields that should be owned by the child. For example, when converting T to `ReadOnlySignal<T>`, the new signal should be owned by the child
        fn has_child_owned_fields(&self) -> bool {
            self.fields.iter().any(|f| child_owned_type(f.ty))
        }

        fn memoize_impl(&self) -> Result<TokenStream, Error> {
            // First check if there are any ReadOnlySignal fields, if there are not, we can just use the partialEq impl
            let signal_fields: Vec<_> = self
                .included_fields()
                .filter(|f| looks_like_signal_type(f.ty))
                .map(|f| {
                    let name = f.name;
                    quote!(#name)
                })
                .collect();

            let move_signal_fields = quote! {
                trait NonPartialEq: Sized {
                    fn compare(&self, other: &Self) -> bool;
                }

                impl<T> NonPartialEq for &&T {
                    fn compare(&self, other: &Self) -> bool {
                        false
                    }
                }

                trait CanPartialEq: PartialEq {
                    fn compare(&self, other: &Self) -> bool;
                }

                impl<T: PartialEq> CanPartialEq for T {
                    fn compare(&self, other: &Self) -> bool {
                        self == other
                    }
                }

                // If they are equal, we don't need to rerun the component we can just update the existing signals
                #(
                    // Try to memo the signal
                    let field_eq = {
                        let old_value: &_ = &*#signal_fields.peek();
                        let new_value: &_ = &*new.#signal_fields.peek();
                        (&old_value).compare(&&new_value)
                    };
                    if !field_eq {
                        (#signal_fields).__set(new.#signal_fields.__take());
                    }
                    // Move the old value back
                    self.#signal_fields = #signal_fields;
                )*
            };

            let event_handlers_fields: Vec<_> = self
                .included_fields()
                .filter(|f| looks_like_callback_type(f.ty))
                .collect();

            let regular_fields: Vec<_> = self
                .included_fields()
                .chain(self.extend_fields())
                .filter(|f| !looks_like_signal_type(f.ty) && !looks_like_callback_type(f.ty))
                .map(|f| {
                    let name = f.name;
                    quote!(#name)
                })
                .collect();

            let move_event_handlers: TokenStream = event_handlers_fields.iter().map(|field| {
                // If this is an optional event handler, we need to check if it's None before we try to update it
                let optional = strip_option(field.ty).is_some();
                let name = field.name;
                if optional {
                    quote! {
                        // If the event handler is None, we don't need to update it
                        if let (Some(old_handler), Some(new_handler)) = (self.#name.as_mut(), new.#name.as_ref()) {
                            old_handler.__set(new_handler.__take());
                        }
                    }
                } else {
                    quote! {
                        // Update the event handlers
                        self.#name.__set(new.#name.__take());
                    }
                }
            }).collect();

            // If there are signals, we automatically try to memoize the signals
            if !signal_fields.is_empty() {
                Ok(quote! {
                    // First check if the fields are equal. This will compare the signal fields by pointer
                    let exactly_equal = self == new;
                    if exactly_equal {
                        // If they are return early, they can be memoized without any changes
                        return true;
                    }

                    // If they are not, move the signal fields into self and check if they are equal now that the signal fields are equal
                    #(
                        let mut #signal_fields = self.#signal_fields;
                        self.#signal_fields = new.#signal_fields;
                    )*

                    // Then check if the fields are equal now that we know the signal fields are equal
                    // NOTE: we don't compare other fields individually because we want to let users opt-out of memoization for certain fields by implementing PartialEq themselves
                    let non_signal_fields_equal = self == new;

                    // If they are not equal, we need to move over all the fields that are not event handlers or signals to self
                    if !non_signal_fields_equal {
                        let new_clone = new.clone();
                        #(
                            self.#regular_fields = new_clone.#regular_fields;
                        )*
                    }
                    // Move any signal and event fields into their old container.
                    // We update signals and event handlers in place so that they are always up to date even if they were moved into a future in a previous render
                    #move_signal_fields
                    #move_event_handlers

                    non_signal_fields_equal
                })
            } else {
                Ok(quote! {
                    let equal = self == new;
                    // Move any signal and event fields into their old container.
                    #move_event_handlers
                    // If they are not equal, we need to move over all the fields that are not event handlers to self
                    if !equal {
                        let new_clone = new.clone();
                        #(
                            self.#regular_fields = new_clone.#regular_fields;
                        )*
                    }
                    equal
                })
            }
        }

        pub fn builder_creation_impl(&self) -> Result<TokenStream, Error> {
            let StructInfo {
                ref vis,
                ref name,
                ref builder_name,
                ..
            } = *self;

            let generics = self.generics.clone();
            let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
            let (_, b_initial_generics, _) = self.generics.split_for_impl();
            let all_fields_param = syn::GenericParam::Type(
                syn::Ident::new("TypedBuilderFields", proc_macro2::Span::call_site()).into(),
            );
            let b_generics = self.modify_generics(|g| {
                g.params.insert(0, all_fields_param.clone());
            });
            let empties_tuple = type_tuple(self.included_fields().map(|_| empty_type()));
            let generics_with_empty = modify_types_generics_hack(&b_initial_generics, |args| {
                args.insert(0, syn::GenericArgument::Type(empties_tuple.clone().into()));
            });
            let phantom_generics = self.generics.params.iter().filter_map(|param| match param {
                syn::GenericParam::Lifetime(lifetime) => {
                    let lifetime = &lifetime.lifetime;
                    Some(quote!(::core::marker::PhantomData<&#lifetime ()>))
                }
                syn::GenericParam::Type(ty) => {
                    let ty = &ty.ident;
                    Some(quote!(::core::marker::PhantomData<#ty>))
                }
                syn::GenericParam::Const(_cnst) => None,
            });
            let builder_method_doc = match self.builder_attr.builder_method_doc {
                Some(ref doc) => quote!(#doc),
                None => {
                    let doc = format!(
                        "
Create a builder for building `{name}`.
On the builder, call {setters} to set the values of the fields.
Finally, call `.build()` to create the instance of `{name}`.
                    ",
                        name = self.name,
                        setters = {
                            let mut result = String::new();
                            let mut is_first = true;
                            for field in self.included_fields() {
                                use std::fmt::Write;
                                if is_first {
                                    is_first = false;
                                } else {
                                    write!(&mut result, ", ").unwrap();
                                }
                                write!(&mut result, "`.{}(...)`", field.name).unwrap();
                                if field.builder_attr.default.is_some() {
                                    write!(&mut result, "(optional)").unwrap();
                                }
                            }
                            result
                        }
                    );
                    quote!(#doc)
                }
            };
            let builder_type_doc = if self.builder_attr.doc {
                match self.builder_attr.builder_type_doc {
                    Some(ref doc) => quote!(#[doc = #doc]),
                    None => {
                        let doc = format!(
                        "Builder for [`{name}`] instances.\n\nSee [`{name}::builder()`] for more info.",
                    );
                        quote!(#[doc = #doc])
                    }
                }
            } else {
                quote!(#[doc(hidden)])
            };

            let (_, _, b_generics_where_extras_predicates) = b_generics.split_for_impl();
            let mut b_generics_where: syn::WhereClause = syn::parse2(quote! {
                where Self: Clone
            })?;
            if let Some(predicates) = b_generics_where_extras_predicates {
                b_generics_where
                    .predicates
                    .extend(predicates.predicates.clone());
            }

            let memoize = self.memoize_impl()?;

            let global_fields = self
                .extend_fields()
                .map(|f| {
                    let name = f.name;
                    let ty = f.ty;
                    quote!(#name: #ty)
                })
                .chain(self.has_child_owned_fields().then(|| quote!(owner: Owner)));
            let global_fields_value = self
                .extend_fields()
                .map(|f| {
                    let name = f.name;
                    quote!(#name: Vec::new())
                })
                .chain(
                    self.has_child_owned_fields()
                        .then(|| quote!(owner: Owner::default())),
                );

            Ok(quote! {
                impl #impl_generics #name #ty_generics #where_clause {
                    #[doc = #builder_method_doc]
                    #[allow(dead_code, clippy::type_complexity)]
                    #vis fn builder() -> #builder_name #generics_with_empty {
                        #builder_name {
                            #(#global_fields_value,)*
                            fields: #empties_tuple,
                            _phantom: ::core::default::Default::default(),
                        }
                    }
                }

                #[must_use]
                #builder_type_doc
                #[allow(dead_code, non_camel_case_types, non_snake_case)]
                #vis struct #builder_name #b_generics {
                    #(#global_fields,)*
                    fields: #all_fields_param,
                    _phantom: (#( #phantom_generics ),*),
                }

                impl #impl_generics dioxus_core::prelude::Properties for #name #ty_generics
                #b_generics_where
                {
                    type Builder = #builder_name #generics_with_empty;
                    fn builder() -> Self::Builder {
                        #name::builder()
                    }
                    fn memoize(&mut self, new: &Self) -> bool {
                        #memoize
                    }
                }
            })
        }

        // TODO: once the proc-macro crate limitation is lifted, make this an util trait of this
        // crate.
        pub fn conversion_helper_impl(&self) -> Result<TokenStream, Error> {
            let trait_name = &self.conversion_helper_trait_name;
            Ok(quote! {
                #[doc(hidden)]
                #[allow(dead_code, non_camel_case_types, non_snake_case)]
                pub trait #trait_name<T> {
                    fn into_value<F: FnOnce() -> T>(self, default: F) -> T;
                }

                impl<T> #trait_name<T> for () {
                    fn into_value<F: FnOnce() -> T>(self, default: F) -> T {
                        default()
                    }
                }

                impl<T> #trait_name<T> for (T,) {
                    fn into_value<F: FnOnce() -> T>(self, _: F) -> T {
                        self.0
                    }
                }
            })
        }

        pub fn extends_impl(&self, field: &FieldInfo) -> Result<TokenStream, Error> {
            let StructInfo {
                ref builder_name, ..
            } = *self;

            let field_name = field.name;

            let descructuring = self.included_fields().map(|f| {
                if f.ordinal == field.ordinal {
                    quote!(_)
                } else {
                    let name = f.name;
                    quote!(#name)
                }
            });
            let reconstructing = self.included_fields().map(|f| f.name);

            let mut ty_generics: Vec<syn::GenericArgument> = self
                .generics
                .params
                .iter()
                .map(|generic_param| match generic_param {
                    syn::GenericParam::Type(type_param) => {
                        let ident = type_param.ident.clone();
                        syn::parse(quote!(#ident).into()).unwrap()
                    }
                    syn::GenericParam::Lifetime(lifetime_def) => {
                        syn::GenericArgument::Lifetime(lifetime_def.lifetime.clone())
                    }
                    syn::GenericParam::Const(const_param) => {
                        let ident = const_param.ident.clone();
                        syn::parse(quote!(#ident).into()).unwrap()
                    }
                })
                .collect();
            let mut target_generics_tuple = empty_type_tuple();
            let mut ty_generics_tuple = empty_type_tuple();
            let generics = self.modify_generics(|g| {
                let index_after_lifetime_in_generics = g
                    .params
                    .iter()
                    .filter(|arg| matches!(arg, syn::GenericParam::Lifetime(_)))
                    .count();
                for f in self.included_fields() {
                    if f.ordinal == field.ordinal {
                        ty_generics_tuple.elems.push_value(empty_type());
                        target_generics_tuple
                            .elems
                            .push_value(f.tuplized_type_ty_param());
                    } else {
                        g.params
                            .insert(index_after_lifetime_in_generics, f.generic_ty_param());
                        let generic_argument: syn::Type = f.type_ident();
                        ty_generics_tuple.elems.push_value(generic_argument.clone());
                        target_generics_tuple.elems.push_value(generic_argument);
                    }
                    ty_generics_tuple.elems.push_punct(Default::default());
                    target_generics_tuple.elems.push_punct(Default::default());
                }
            });
            let mut target_generics = ty_generics.clone();
            let index_after_lifetime_in_generics = target_generics
                .iter()
                .filter(|arg| matches!(arg, syn::GenericArgument::Lifetime(_)))
                .count();
            target_generics.insert(
                index_after_lifetime_in_generics,
                syn::GenericArgument::Type(target_generics_tuple.into()),
            );
            ty_generics.insert(
                index_after_lifetime_in_generics,
                syn::GenericArgument::Type(ty_generics_tuple.into()),
            );
            let (impl_generics, _, where_clause) = generics.split_for_impl();

            let forward_extended_fields = self.extend_fields().map(|f| {
                let name = f.name;
                quote!(#name: self.#name)
            });

            let forward_owner = self
                .has_child_owned_fields()
                .then(|| quote!(owner: self.owner))
                .into_iter();

            let extends_impl = field.builder_attr.extends.iter().map(|path| {
                let name_str = path_to_single_string(path).unwrap();
                let camel_name = name_str.to_case(Case::UpperCamel);
                let marker_name = Ident::new(
                    format!("{}Extension", &camel_name).as_str(),
                    path.span(),
                );
                quote! {
                    #[allow(dead_code, non_camel_case_types, missing_docs)]
                    impl #impl_generics dioxus_elements::extensions::#marker_name for #builder_name < #( #ty_generics ),* > #where_clause {}
                }
            });

            Ok(quote! {
                #[allow(dead_code, non_camel_case_types, missing_docs)]
                impl #impl_generics dioxus_core::prelude::HasAttributes for #builder_name < #( #ty_generics ),* > #where_clause {
                    fn push_attribute<L>(
                        mut self,
                        name: &'static str,
                        ns: Option<&'static str>,
                        attr: impl dioxus_core::prelude::IntoAttributeValue<L>,
                        volatile: bool
                    ) -> Self {
                        let ( #(#descructuring,)* ) = self.fields;
                        self.#field_name.push(
                            dioxus_core::Attribute::new(
                                name,
                                {
                                    use dioxus_core::prelude::IntoAttributeValue;
                                    attr.into_value()
                                },
                                ns,
                                volatile,
                            )
                        );
                        #builder_name {
                            #(#forward_extended_fields,)*
                            #(#forward_owner,)*
                            fields: ( #(#reconstructing,)* ),
                            _phantom: self._phantom,
                        }
                    }
                }

                #(#extends_impl)*
            })
        }

        pub fn field_impl(&self, field: &FieldInfo) -> Result<TokenStream, Error> {
            let FieldInfo {
                name: field_name, ..
            } = field;
            if *field_name == "key" {
                return Err(Error::new_spanned(field_name, "Naming a prop `key` is not allowed because the name can conflict with the built in key attribute. See https://dioxuslabs.com/learn/0.5/reference/dynamic_rendering#rendering-lists for more information about keys"));
            }
            let StructInfo {
                ref builder_name, ..
            } = *self;

            let descructuring = self.included_fields().map(|f| {
                if f.ordinal == field.ordinal {
                    quote!(_)
                } else {
                    let name = f.name;
                    quote!(#name)
                }
            });
            let reconstructing = self.included_fields().map(|f| f.name);

            let FieldInfo {
                name: field_name,
                ty: field_type,
                ..
            } = field;
            let mut ty_generics: Vec<syn::GenericArgument> = self
                .generics
                .params
                .iter()
                .map(|generic_param| match generic_param {
                    syn::GenericParam::Type(type_param) => {
                        let ident = type_param.ident.clone();
                        syn::parse(quote!(#ident).into()).unwrap()
                    }
                    syn::GenericParam::Lifetime(lifetime_def) => {
                        syn::GenericArgument::Lifetime(lifetime_def.lifetime.clone())
                    }
                    syn::GenericParam::Const(const_param) => {
                        let ident = const_param.ident.clone();
                        syn::parse(quote!(#ident).into()).unwrap()
                    }
                })
                .collect();
            let mut target_generics_tuple = empty_type_tuple();
            let mut ty_generics_tuple = empty_type_tuple();
            let generics = self.modify_generics(|g| {
                let index_after_lifetime_in_generics = g
                    .params
                    .iter()
                    .filter(|arg| matches!(arg, syn::GenericParam::Lifetime(_)))
                    .count();
                for f in self.included_fields() {
                    if f.ordinal == field.ordinal {
                        ty_generics_tuple.elems.push_value(empty_type());
                        target_generics_tuple
                            .elems
                            .push_value(f.tuplized_type_ty_param());
                    } else {
                        g.params
                            .insert(index_after_lifetime_in_generics, f.generic_ty_param());
                        let generic_argument: syn::Type = f.type_ident();
                        ty_generics_tuple.elems.push_value(generic_argument.clone());
                        target_generics_tuple.elems.push_value(generic_argument);
                    }
                    ty_generics_tuple.elems.push_punct(Default::default());
                    target_generics_tuple.elems.push_punct(Default::default());
                }
            });
            let mut target_generics = ty_generics.clone();
            let index_after_lifetime_in_generics = target_generics
                .iter()
                .filter(|arg| matches!(arg, syn::GenericArgument::Lifetime(_)))
                .count();
            target_generics.insert(
                index_after_lifetime_in_generics,
                syn::GenericArgument::Type(target_generics_tuple.into()),
            );
            ty_generics.insert(
                index_after_lifetime_in_generics,
                syn::GenericArgument::Type(ty_generics_tuple.into()),
            );

            let (impl_generics, _, where_clause) = generics.split_for_impl();
            let docs = &field.builder_attr.docs;

            let arg_type = field_type;
            // If the field is auto_into, we need to add a generic parameter to the builder for specialization
            let mut marker = None;
            let (arg_type, arg_expr) = if child_owned_type(arg_type) {
                let marker_ident = syn::Ident::new("__Marker", proc_macro2::Span::call_site());
                marker = Some(marker_ident.clone());
                (
                    quote!(impl dioxus_core::prelude::SuperInto<#arg_type, #marker_ident>),
                    // If this looks like a signal type, we automatically convert it with SuperInto and use the props struct as the owner
                    quote!(with_owner(self.owner.clone(), move || dioxus_core::prelude::SuperInto::super_into(#field_name))),
                )
            } else if field.builder_attr.auto_into || field.builder_attr.strip_option {
                let marker_ident = syn::Ident::new("__Marker", proc_macro2::Span::call_site());
                marker = Some(marker_ident.clone());
                (
                    quote!(impl dioxus_core::prelude::SuperInto<#arg_type, #marker_ident>),
                    quote!(dioxus_core::prelude::SuperInto::super_into(#field_name)),
                )
            } else if field.builder_attr.from_displayable {
                (
                    quote!(impl ::core::fmt::Display),
                    quote!(#field_name.to_string()),
                )
            } else {
                (quote!(#arg_type), quote!(#field_name))
            };

            let repeated_fields_error_type_name = syn::Ident::new(
                &format!(
                    "{}_Error_Repeated_field_{}",
                    builder_name,
                    strip_raw_ident_prefix(field_name.to_string())
                ),
                builder_name.span(),
            );
            let repeated_fields_error_message = format!("Repeated field {field_name}");

            let forward_fields = self
                .extend_fields()
                .map(|f| {
                    let name = f.name;
                    quote!(#name: self.#name)
                })
                .chain(
                    self.has_child_owned_fields()
                        .then(|| quote!(owner: self.owner)),
                );

            Ok(quote! {
                #[allow(dead_code, non_camel_case_types, missing_docs)]
                impl #impl_generics #builder_name < #( #ty_generics ),* > #where_clause {
                    #( #docs )*
                    #[allow(clippy::type_complexity)]
                    pub fn #field_name < #marker > (self, #field_name: #arg_type) -> #builder_name < #( #target_generics ),* > {
                        let #field_name = (#arg_expr,);
                        let ( #(#descructuring,)* ) = self.fields;
                        #builder_name {
                            #(#forward_fields,)*
                            fields: ( #(#reconstructing,)* ),
                            _phantom: self._phantom,
                        }
                    }
                }
                #[doc(hidden)]
                #[allow(dead_code, non_camel_case_types, non_snake_case)]
                pub enum #repeated_fields_error_type_name {}
                #[doc(hidden)]
                #[allow(dead_code, non_camel_case_types, missing_docs)]
                impl #impl_generics #builder_name < #( #target_generics ),* > #where_clause {
                    #[deprecated(
                        note = #repeated_fields_error_message
                    )]
                    #[allow(clippy::type_complexity)]
                    pub fn #field_name< #marker > (self, _: #repeated_fields_error_type_name) -> #builder_name < #( #target_generics ),* > {
                        self
                    }
                }
            })
        }

        pub fn required_field_impl(&self, field: &FieldInfo) -> Result<TokenStream, Error> {
            let StructInfo {
                ref name,
                ref builder_name,
                ..
            } = self;

            let FieldInfo {
                name: ref field_name,
                ..
            } = field;
            let mut builder_generics: Vec<syn::GenericArgument> = self
                .generics
                .params
                .iter()
                .map(|generic_param| match generic_param {
                    syn::GenericParam::Type(type_param) => {
                        let ident = &type_param.ident;
                        syn::parse(quote!(#ident).into()).unwrap()
                    }
                    syn::GenericParam::Lifetime(lifetime_def) => {
                        syn::GenericArgument::Lifetime(lifetime_def.lifetime.clone())
                    }
                    syn::GenericParam::Const(const_param) => {
                        let ident = &const_param.ident;
                        syn::parse(quote!(#ident).into()).unwrap()
                    }
                })
                .collect();
            let mut builder_generics_tuple = empty_type_tuple();
            let generics = self.modify_generics(|g| {
                let index_after_lifetime_in_generics = g
                    .params
                    .iter()
                    .filter(|arg| matches!(arg, syn::GenericParam::Lifetime(_)))
                    .count();
                for f in self.included_fields() {
                    if f.builder_attr.default.is_some() {
                        // `f` is not mandatory - it does not have it's own fake `build` method, so `field` will need
                        // to warn about missing `field` whether or not `f` is set.
                        assert!(
                            f.ordinal != field.ordinal,
                            "`required_field_impl` called for optional field {}",
                            field.name
                        );
                        g.params
                            .insert(index_after_lifetime_in_generics, f.generic_ty_param());
                        builder_generics_tuple.elems.push_value(f.type_ident());
                    } else if f.ordinal < field.ordinal {
                        // Only add a `build` method that warns about missing `field` if `f` is set. If `f` is not set,
                        // `f`'s `build` method will warn, since it appears earlier in the argument list.
                        builder_generics_tuple
                            .elems
                            .push_value(f.tuplized_type_ty_param());
                    } else if f.ordinal == field.ordinal {
                        builder_generics_tuple.elems.push_value(empty_type());
                    } else {
                        // `f` appears later in the argument list after `field`, so if they are both missing we will
                        // show a warning for `field` and not for `f` - which means this warning should appear whether
                        // or not `f` is set.
                        g.params
                            .insert(index_after_lifetime_in_generics, f.generic_ty_param());
                        builder_generics_tuple.elems.push_value(f.type_ident());
                    }

                    builder_generics_tuple.elems.push_punct(Default::default());
                }
            });

            let index_after_lifetime_in_generics = builder_generics
                .iter()
                .filter(|arg| matches!(arg, syn::GenericArgument::Lifetime(_)))
                .count();
            builder_generics.insert(
                index_after_lifetime_in_generics,
                syn::GenericArgument::Type(builder_generics_tuple.into()),
            );
            let (impl_generics, _, where_clause) = generics.split_for_impl();
            let (_, ty_generics, _) = self.generics.split_for_impl();

            let early_build_error_type_name = syn::Ident::new(
                &format!(
                    "{}_Error_Missing_required_field_{}",
                    builder_name,
                    strip_raw_ident_prefix(field_name.to_string())
                ),
                builder_name.span(),
            );
            let early_build_error_message = format!("Missing required field {field_name}");

            Ok(quote! {
                #[doc(hidden)]
                #[allow(dead_code, non_camel_case_types, non_snake_case)]
                pub enum #early_build_error_type_name {}
                #[doc(hidden)]
                #[allow(dead_code, non_camel_case_types, missing_docs, clippy::panic)]
                impl #impl_generics #builder_name < #( #builder_generics ),* > #where_clause {
                    #[deprecated(
                        note = #early_build_error_message
                    )]
                    pub fn build(self, _: #early_build_error_type_name) -> #name #ty_generics {
                        panic!()
                    }
                }
            })
        }

        pub fn build_method_impl(&self) -> TokenStream {
            let StructInfo {
                ref name,
                ref builder_name,
                ..
            } = *self;

            let generics = self.modify_generics(|g| {
                let index_after_lifetime_in_generics = g
                    .params
                    .iter()
                    .filter(|arg| matches!(arg, syn::GenericParam::Lifetime(_)))
                    .count();
                for field in self.included_fields() {
                    if field.builder_attr.default.is_some() {
                        let trait_ref = syn::TraitBound {
                            paren_token: None,
                            lifetimes: None,
                            modifier: syn::TraitBoundModifier::None,
                            path: syn::PathSegment {
                                ident: self.conversion_helper_trait_name.clone(),
                                arguments: syn::PathArguments::AngleBracketed(
                                    syn::AngleBracketedGenericArguments {
                                        colon2_token: None,
                                        lt_token: Default::default(),
                                        args: make_punctuated_single(syn::GenericArgument::Type(
                                            field.ty.clone(),
                                        )),
                                        gt_token: Default::default(),
                                    },
                                ),
                            }
                            .into(),
                        };
                        let mut generic_param: syn::TypeParam = field.generic_ident.clone().into();
                        generic_param.bounds.push(trait_ref.into());
                        g.params
                            .insert(index_after_lifetime_in_generics, generic_param.into());
                    }
                }
            });
            let (impl_generics, _, _) = generics.split_for_impl();

            let (original_impl_generics, ty_generics, where_clause) =
                self.generics.split_for_impl();

            let modified_ty_generics = modify_types_generics_hack(&ty_generics, |args| {
                args.insert(
                    0,
                    syn::GenericArgument::Type(
                        type_tuple(self.included_fields().map(|field| {
                            if field.builder_attr.default.is_some() {
                                field.type_ident()
                            } else {
                                field.tuplized_type_ty_param()
                            }
                        }))
                        .into(),
                    ),
                );
            });

            let descructuring = self.included_fields().map(|f| f.name);

            let helper_trait_name = &self.conversion_helper_trait_name;
            // The default of a field can refer to earlier-defined fields, which we handle by
            // writing out a bunch of `let` statements first, which can each refer to earlier ones.
            // This means that field ordering may actually be significant, which isn’t ideal. We could
            // relax that restriction by calculating a DAG of field default dependencies and
            // reordering based on that, but for now this much simpler thing is a reasonable approach.
            let assignments = self.fields.iter().map(|field| {
                let name = &field.name;
                if !field.builder_attr.extends.is_empty() {
                    quote!(let #name = self.#name;)
                } else if let Some(ref default) = field.builder_attr.default {

                    // If field has `into`, apply it to the default value.
                    // Ignore any blank defaults as it causes type inference errors.
                    let is_default = *default == parse_quote!(::core::default::Default::default());
                    let mut into = quote!{};

                    if !is_default {
                        if field.builder_attr.auto_into {
                            into = quote!{ .into() }
                        } else if field.builder_attr.auto_to_string {
                            into = quote!{ .to_string() }
                        }
                    }

                    if field.builder_attr.skip {
                        quote!(let #name = #default #into;)
                    } else {
                        quote!(let #name = #helper_trait_name::into_value(#name, || #default #into);)
                    }
                } else {
                    quote!(let #name = #name.0;)
                }
            });
            let field_names = self.fields.iter().map(|field| field.name);
            let doc = if self.builder_attr.doc {
                match self.builder_attr.build_method_doc {
                    Some(ref doc) => quote!(#[doc = #doc]),
                    None => {
                        // I’d prefer “a” or “an” to “its”, but determining which is grammatically
                        // correct is roughly impossible.
                        let doc =
                            format!("Finalize the builder and create its [`{name}`] instance");
                        quote!(#[doc = #doc])
                    }
                }
            } else {
                quote!()
            };

            if self.has_child_owned_fields() {
                let name = Ident::new(&format!("{}WithOwner", name), name.span());
                let original_name = &self.name;
                let vis = &self.vis;
                let generics_with_bounds = &self.generics;
                let where_clause = &self.generics.where_clause;

                quote! {
                    #[doc(hidden)]
                    #[allow(dead_code, non_camel_case_types, missing_docs)]
                    #[derive(Clone)]
                    #vis struct #name #generics_with_bounds #where_clause {
                        inner: #original_name #ty_generics,
                        owner: Owner,
                    }

                    impl #original_impl_generics PartialEq for #name #ty_generics #where_clause {
                        fn eq(&self, other: &Self) -> bool {
                            self.inner.eq(&other.inner)
                        }
                    }

                    impl #original_impl_generics #name #ty_generics #where_clause {
                        /// Create a component from the props.
                        pub fn into_vcomponent<M: 'static>(
                            self,
                            render_fn: impl dioxus_core::prelude::ComponentFunction<#original_name #ty_generics, M>,
                        ) -> dioxus_core::VComponent {
                            use dioxus_core::prelude::ComponentFunction;
                            let component_name = ::std::any::type_name_of_val(&render_fn);
                            dioxus_core::VComponent::new(move |wrapper: Self| render_fn.rebuild(wrapper.inner), self, component_name)
                        }
                    }

                    impl #original_impl_generics dioxus_core::prelude::Properties for #name #ty_generics #where_clause {
                        type Builder = ();
                        fn builder() -> Self::Builder {
                            unreachable!()
                        }
                        fn memoize(&mut self, new: &Self) -> bool {
                            self.inner.memoize(&new.inner)
                        }
                    }

                    #[allow(dead_code, non_camel_case_types, missing_docs)]
                    impl #impl_generics #builder_name #modified_ty_generics #where_clause {
                        #doc
                        pub fn build(self) -> #name #ty_generics {
                            let ( #(#descructuring,)* ) = self.fields;
                            #( #assignments )*
                            #name {
                                inner: #original_name {
                                    #( #field_names ),*
                                },
                                owner: self.owner,
                            }
                        }
                    }
                }
            } else {
                quote!(
                    #[allow(dead_code, non_camel_case_types, missing_docs)]
                    impl #impl_generics #builder_name #modified_ty_generics #where_clause {
                        #doc
                        pub fn build(self) -> #name #ty_generics {
                            let ( #(#descructuring,)* ) = self.fields;
                            #( #assignments )*
                            #name {
                                #( #field_names ),*
                            }
                        }
                    }
                )
            }
        }
    }

    #[derive(Debug, Default)]
    pub struct TypeBuilderAttr {
        /// Whether to show docs for the `TypeBuilder` type (rather than hiding them).
        pub doc: bool,

        /// Docs on the `Type::builder()` method.
        pub builder_method_doc: Option<syn::Expr>,

        /// Docs on the `TypeBuilder` type. Specifying this implies `doc`, but you can just specify
        /// `doc` instead and a default value will be filled in here.
        pub builder_type_doc: Option<syn::Expr>,

        /// Docs on the `TypeBuilder.build()` method. Specifying this implies `doc`, but you can just
        /// specify `doc` instead and a default value will be filled in here.
        pub build_method_doc: Option<syn::Expr>,

        pub field_defaults: FieldBuilderAttr,
    }

    impl TypeBuilderAttr {
        pub fn new(attrs: &[syn::Attribute]) -> Result<TypeBuilderAttr, Error> {
            let mut result = TypeBuilderAttr::default();
            for attr in attrs {
                if path_to_single_string(attr.path()).as_deref() != Some("builder") {
                    continue;
                }

                match &attr.meta {
                    syn::Meta::List(list) => {
                        if list.tokens.is_empty() {
                            continue;
                        }
                    }
                    _ => {
                        continue;
                    }
                }

                let as_expr = attr.parse_args_with(
                    Punctuated::<Expr, syn::Token![,]>::parse_separated_nonempty,
                )?;

                for expr in as_expr.into_iter() {
                    result.apply_meta(expr)?;
                }
            }

            Ok(result)
        }

        fn apply_meta(&mut self, expr: syn::Expr) -> Result<(), Error> {
            match expr {
                syn::Expr::Assign(assign) => {
                    let name = expr_to_single_string(&assign.left)
                        .ok_or_else(|| Error::new_spanned(&assign.left, "Expected identifier"))?;
                    match name.as_str() {
                        "builder_method_doc" => {
                            self.builder_method_doc = Some(*assign.right);
                            Ok(())
                        }
                        "builder_type_doc" => {
                            self.builder_type_doc = Some(*assign.right);
                            self.doc = true;
                            Ok(())
                        }
                        "build_method_doc" => {
                            self.build_method_doc = Some(*assign.right);
                            self.doc = true;
                            Ok(())
                        }
                        _ => Err(Error::new_spanned(
                            &assign,
                            format!("Unknown parameter {name:?}"),
                        )),
                    }
                }
                syn::Expr::Path(path) => {
                    let name = path_to_single_string(&path.path)
                        .ok_or_else(|| Error::new_spanned(&path, "Expected identifier"))?;
                    match name.as_str() {
                        "doc" => {
                            self.doc = true;
                            Ok(())
                        }
                        _ => Err(Error::new_spanned(
                            &path,
                            format!("Unknown parameter {name:?}"),
                        )),
                    }
                }
                syn::Expr::Call(call) => {
                    let subsetting_name = if let syn::Expr::Path(path) = &*call.func {
                        path_to_single_string(&path.path)
                    } else {
                        None
                    }
                    .ok_or_else(|| {
                        let call_func = &call.func;
                        let call_func = quote!(#call_func);
                        Error::new_spanned(
                            &call.func,
                            format!("Illegal builder setting group {call_func}"),
                        )
                    })?;
                    match subsetting_name.as_str() {
                        "field_defaults" => {
                            for arg in call.args {
                                self.field_defaults.apply_meta(arg)?;
                            }
                            Ok(())
                        }
                        _ => Err(Error::new_spanned(
                            &call.func,
                            format!("Illegal builder setting group name {subsetting_name}"),
                        )),
                    }
                }
                _ => Err(Error::new_spanned(expr, "Expected (<...>=<...>)")),
            }
        }
    }
}

/// A helper function for paring types with a single generic argument.
fn extract_base_type_without_generics(ty: &Type) -> Option<syn::Path> {
    let Type::Path(ty) = ty else {
        return None;
    };
    if ty.qself.is_some() {
        return None;
    }

    let path = &ty.path;

    let mut path_segments_without_generics = Vec::new();

    let mut generic_arg_count = 0;

    for segment in &path.segments {
        let mut segment = segment.clone();
        match segment.arguments {
            PathArguments::AngleBracketed(_) => generic_arg_count += 1,
            PathArguments::Parenthesized(_) => {
                return None;
            }
            _ => {}
        }
        segment.arguments = syn::PathArguments::None;
        path_segments_without_generics.push(segment);
    }

    // If there is more than the type and the single generic argument, it doesn't look like the type we want
    if generic_arg_count > 2 {
        return None;
    }

    let path_without_generics = syn::Path {
        leading_colon: None,
        segments: Punctuated::from_iter(path_segments_without_generics),
    };

    Some(path_without_generics)
}

/// Returns the type inside the Option wrapper if it exists
fn strip_option(type_: &Type) -> Option<Type> {
    if let Type::Path(ty) = &type_ {
        let mut segments_iter = ty.path.segments.iter().peekable();
        // Strip any leading std||core::option:: prefix
        let allowed_segments: &[&[&str]] = &[&["std", "core"], &["option"]];
        let mut allowed_segments_iter = allowed_segments.iter();
        while let Some(segment) = segments_iter.peek() {
            let Some(allowed_segments) = allowed_segments_iter.next() else {
                break;
            };
            if !allowed_segments.contains(&segment.ident.to_string().as_str()) {
                break;
            }
            segments_iter.next();
        }
        // The last segment should be Option
        let option_segment = segments_iter.next()?;
        if option_segment.ident == "Option" && segments_iter.next().is_none() {
            // It should have a single generic argument
            if let PathArguments::AngleBracketed(generic_arg) = &option_segment.arguments {
                if let Some(syn::GenericArgument::Type(ty)) = generic_arg.args.first() {
                    return Some(ty.clone());
                }
            }
        }
    }
    None
}

/// Remove the Option wrapper from a type
fn remove_option_wrapper(type_: Type) -> Type {
    strip_option(&type_).unwrap_or(type_)
}

/// Check if a type should be owned by the child component after conversion
fn child_owned_type(ty: &Type) -> bool {
    looks_like_signal_type(ty) || looks_like_callback_type(ty)
}

fn looks_like_signal_type(ty: &Type) -> bool {
    match extract_base_type_without_generics(ty) {
        Some(path_without_generics) => {
            path_without_generics == parse_quote!(dioxus_core::prelude::ReadOnlySignal)
                || path_without_generics == parse_quote!(prelude::ReadOnlySignal)
                || path_without_generics == parse_quote!(ReadOnlySignal)
        }
        None => false,
    }
}

fn looks_like_callback_type(ty: &Type) -> bool {
    let type_without_option = remove_option_wrapper(ty.clone());
    match extract_base_type_without_generics(&type_without_option) {
        Some(path_without_generics) => {
            path_without_generics == parse_quote!(dioxus_core::prelude::EventHandler)
                || path_without_generics == parse_quote!(prelude::EventHandler)
                || path_without_generics == parse_quote!(EventHandler)
                || path_without_generics == parse_quote!(dioxus_core::prelude::Callback)
                || path_without_generics == parse_quote!(prelude::Callback)
                || path_without_generics == parse_quote!(Callback)
        }
        None => false,
    }
}

#[test]
fn test_looks_like_type() {
    assert!(!looks_like_signal_type(&parse_quote!(
        Option<ReadOnlySignal<i32>>
    )));
    assert!(looks_like_signal_type(&parse_quote!(ReadOnlySignal<i32>)));
    assert!(looks_like_signal_type(
        &parse_quote!(ReadOnlySignal<i32, SyncStorage>)
    ));
    assert!(looks_like_signal_type(&parse_quote!(
        ReadOnlySignal<Option<i32>, UnsyncStorage>
    )));

    assert!(looks_like_callback_type(&parse_quote!(
        Option<EventHandler>
    )));
    assert!(looks_like_callback_type(&parse_quote!(
        std::option::Option<EventHandler<i32>>
    )));
    assert!(looks_like_callback_type(&parse_quote!(
        Option<EventHandler<MouseEvent>>
    )));

    assert!(looks_like_callback_type(&parse_quote!(EventHandler<i32>)));
    assert!(looks_like_callback_type(&parse_quote!(EventHandler)));

    assert!(looks_like_callback_type(&parse_quote!(Callback<i32>)));
    assert!(looks_like_callback_type(&parse_quote!(Callback<i32, u32>)));
}

#[test]
fn test_remove_option_wrapper() {
    let type_without_option = remove_option_wrapper(parse_quote!(Option<i32>));
    assert_eq!(type_without_option, parse_quote!(i32));

    let type_without_option = remove_option_wrapper(parse_quote!(Option<Option<i32>>));
    assert_eq!(type_without_option, parse_quote!(Option<i32>));

    let type_without_option = remove_option_wrapper(parse_quote!(Option<Option<Option<i32>>>));
    assert_eq!(type_without_option, parse_quote!(Option<Option<i32>>));
}

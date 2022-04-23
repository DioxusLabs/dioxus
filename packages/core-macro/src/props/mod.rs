//! This code mostly comes from idanarye/rust-typed-builder
//!
//! However, it has been adopted to fit the Dioxus Props builder pattern.
//!
//! For dioxus, we make a few changes:
//! - [ ] automatically implement Into<Option> on the setters (IE the strip setter option)
//! - [ ] automatically implement a default of none for optional fields (those explicitly wrapped with Option<T>)

use proc_macro2::TokenStream;
use quote::quote;
use syn::parse::Error;
use syn::spanned::Spanned;

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
        if let syn::Expr::Path(path) = &*expr {
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
    use std::str::FromStr;

    use proc_macro2::TokenStream;
    use quote::{quote, ToTokens};
    use syn::parse::Error;
    use syn::spanned::Spanned;
    use syn::Expr;

    use crate::props::injection::Selectors;

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
                        Some(syn::parse(quote!(Default::default()).into()).unwrap());
                }

                // auto detect optional
                let strip_option_auto = builder_attr.strip_option
                    || !builder_attr.ignore_option
                        && type_from_inside_option(&field.ty, true).is_some();
                if !builder_attr.strip_option && strip_option_auto {
                    builder_attr.strip_option = true;
                    builder_attr.default =
                        Some(syn::parse(quote!(Default::default()).into()).unwrap());
                }

                Ok(FieldInfo {
                    ordinal,
                    name,
                    generic_ident: syn::Ident::new(
                        &format!("__{}", strip_raw_ident_prefix(name.to_string())),
                        proc_macro2::Span::call_site(),
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

        pub fn type_from_inside_option(&self, check_option_name: bool) -> Option<&syn::Type> {
            type_from_inside_option(self.ty, check_option_name)
        }
    }

    #[derive(Debug, Default, Clone)]
    pub struct FieldBuilderAttr {
        pub default: Option<syn::Expr>,
        pub doc: Option<syn::Expr>,
        pub skip: bool,
        pub auto_into: bool,
        pub strip_option: bool,
        pub ignore_option: bool,
        pub inject_as: Option<String>,
        pub selectors: Option<Selectors>,
    }

    impl FieldBuilderAttr {
        pub fn with(mut self, attrs: &[syn::Attribute]) -> Result<Self, Error> {
            let mut skip_tokens = None;
            for attr in attrs {
                if path_to_single_string(&attr.path).as_deref() != Some("props") {
                    continue;
                }

                if attr.tokens.is_empty() {
                    continue;
                }

                let as_expr: syn::Expr = syn::parse2(attr.tokens.clone())?;
                match as_expr {
                    syn::Expr::Paren(body) => {
                        self.apply_meta(*body.expr)?;
                    }
                    syn::Expr::Tuple(body) => {
                        for expr in body.elems.into_iter() {
                            self.apply_meta(expr)?;
                        }
                    }
                    _ => {
                        return Err(Error::new_spanned(attr.tokens.clone(), "Expected (<...>)"));
                    }
                }
                // Stash its span for later (we don’t yet know if it’ll be an error)
                if self.skip && skip_tokens.is_none() {
                    skip_tokens = Some(attr.tokens.clone());
                }

                if self.inject_as.is_some() && self.selectors.is_none() {
                    return Err(Error::new_spanned(
                        attr,
                        r#"#[props(inject_as = "..")] must be accompanied by a "selector" declaration"#,
                    ));
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
                        "default" => {
                            self.default = Some(*assign.right);
                            Ok(())
                        }
                        "doc" => {
                            self.doc = Some(*assign.right);
                            Ok(())
                        }
                        "default_code" => {
                            if let syn::Expr::Lit(syn::ExprLit {
                                lit: syn::Lit::Str(code),
                                ..
                            }) = *assign.right
                            {
                                let tokenized_code = TokenStream::from_str(&code.value())?;
                                self.default = Some(
                                    syn::parse(tokenized_code.into())
                                        .map_err(|e| Error::new_spanned(code, format!("{}", e)))?,
                                );
                            } else {
                                return Err(Error::new_spanned(assign.right, "Expected string"));
                            }
                            Ok(())
                        }
                        "inject_as" => {
                            let inject_as = parse_string_literal(&assign.right)?;

                            self.inject_as = Some(inject_as);

                            Ok(())
                        }
                        // simple tag selector for applying event handler to inner element/component
                        "selector" => {
                            if self.selectors.is_some() {
                                Err(Error::new_spanned(
                                    assign,
                                    r#""selector" already defined, can only apply one"#,
                                ))
                            } else {
                                self.selectors = Some(parse_selectors(&assign.right)?);

                                Ok(())
                            }
                        }
                        _ => Err(Error::new_spanned(
                            &assign,
                            format!("Unknown parameter {:?}", name),
                        )),
                    }
                }

                // #[props(default)]
                syn::Expr::Path(path) => {
                    let name = path_to_single_string(&path.path)
                        .ok_or_else(|| Error::new_spanned(&path, "Expected identifier"))?;
                    match name.as_str() {
                        "default" => {
                            self.default =
                                Some(syn::parse(quote!(Default::default()).into()).unwrap());
                            Ok(())
                        }

                        "optional" => {
                            self.default =
                                Some(syn::parse(quote!(Default::default()).into()).unwrap());
                            self.strip_option = true;
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
                            "doc" => {
                                self.doc = None;
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

    #[inline]
    fn parse_selectors(source: &Expr) -> syn::Result<Selectors> {
        Selectors::from_str(&parse_string_literal(source)?)
            .map_err(|err| Error::new_spanned(&source, err))
    }

    fn parse_string_literal(source: &Expr) -> syn::Result<String> {
        let expr = (&source).into_token_stream().to_string();

        if expr.starts_with('"') && expr.ends_with('"') {
            let literal = expr.strip_prefix('"').unwrap().strip_suffix('"').unwrap();

            Ok(literal.to_string())
        } else {
            Err(Error::new_spanned(source, "expected a string literal"))
        }
    }
}

fn type_from_inside_option(ty: &syn::Type, check_option_name: bool) -> Option<&syn::Type> {
    let path = if let syn::Type::Path(type_path) = ty {
        if type_path.qself.is_some() {
            return None;
        } else {
            &type_path.path
        }
    } else {
        return None;
    };
    let segment = path.segments.last()?;
    if check_option_name && segment.ident != "Option" {
        return None;
    }
    let generic_params =
        if let syn::PathArguments::AngleBracketed(generic_params) = &segment.arguments {
            generic_params
        } else {
            return None;
        };
    if let syn::GenericArgument::Type(ty) = generic_params.args.first()? {
        Some(ty)
    } else {
        None
    }
}

mod struct_info {
    use proc_macro2::TokenStream;
    use quote::quote;
    use syn::parse::Error;

    use crate::props::injection::InjectedProperties;

    use super::field_info::{FieldBuilderAttr, FieldInfo};
    use super::util::{
        empty_type, empty_type_tuple, expr_to_single_string, make_punctuated_single,
        modify_types_generics_hack, path_to_single_string, strip_raw_ident_prefix, type_tuple,
    };

    #[derive(Debug)]
    pub struct StructInfo<'a> {
        pub vis: &'a syn::Visibility,
        pub name: &'a syn::Ident,
        pub generics: &'a syn::Generics,
        pub fields: Vec<FieldInfo<'a>>,

        pub builder_attr: TypeBuilderAttr,
        pub builder_name: syn::Ident,
        pub conversion_helper_trait_name: syn::Ident,
        pub core: syn::Ident,
    }

    impl<'a> StructInfo<'a> {
        pub fn included_fields(&self) -> impl Iterator<Item = &FieldInfo<'a>> {
            self.fields.iter().filter(|f| !f.builder_attr.skip)
        }

        pub fn new(
            ast: &'a syn::DeriveInput,
            fields: impl Iterator<Item = &'a syn::Field>,
        ) -> Result<StructInfo<'a>, Error> {
            let builder_attr = TypeBuilderAttr::new(&ast.attrs)?;
            let builder_name = strip_raw_ident_prefix(format!("{}Builder", ast.ident));
            let mut fields = fields
                .enumerate()
                .map(|(i, f)| FieldInfo::new(i, f, builder_attr.field_defaults.clone()))
                .collect::<Result<Vec<_>, _>>()?;

            InjectedProperties::add_injected_fields(&ast.ident.to_string(), &mut fields)?;

            Ok(StructInfo {
                vis: &ast.vis,
                name: &ast.ident,
                generics: &ast.generics,
                fields,
                builder_attr,
                builder_name: syn::Ident::new(&builder_name, proc_macro2::Span::call_site()),
                conversion_helper_trait_name: syn::Ident::new(
                    &format!("{}_Optional", builder_name),
                    proc_macro2::Span::call_site(),
                ),
                core: syn::Ident::new(
                    &format!("{}_core", builder_name),
                    proc_macro2::Span::call_site(),
                ),
            })
        }

        fn modify_generics<F: FnMut(&mut syn::Generics)>(&self, mut mutator: F) -> syn::Generics {
            let mut generics = self.generics.clone();
            mutator(&mut generics);
            generics
        }

        pub fn builder_creation_impl(&self) -> Result<TokenStream, Error> {
            let StructInfo {
                ref vis,
                ref name,
                ref builder_name,
                ..
            } = *self;

            // we're generating stuff that goes into unsafe code here
            // we use the heuristic: are there *any* generic parameters?
            // If so, then they might have non-static lifetimes and we can't compare two generic things that *might borrow*
            // Therefore, we will generate code that shortcircuits the "comparison" in memoization
            let are_there_generics = !self.generics.params.is_empty();

            let (impl_generics, ty_generics, where_clause) = self.generics.split_for_impl();
            let all_fields_param = syn::GenericParam::Type(
                syn::Ident::new("TypedBuilderFields", proc_macro2::Span::call_site()).into(),
            );
            let b_generics = self.modify_generics(|g| {
                g.params.insert(0, all_fields_param.clone());
            });
            let empties_tuple = type_tuple(self.included_fields().map(|_| empty_type()));
            let generics_with_empty = modify_types_generics_hack(&ty_generics, |args| {
                args.insert(0, syn::GenericArgument::Type(empties_tuple.clone().into()));
            });
            let phantom_generics = self.generics.params.iter().map(|param| match param {
                syn::GenericParam::Lifetime(lifetime) => {
                    let lifetime = &lifetime.lifetime;
                    quote!(core::marker::PhantomData<&#lifetime ()>)
                }
                syn::GenericParam::Type(ty) => {
                    let ty = &ty.ident;
                    quote!(core::marker::PhantomData<#ty>)
                }
                syn::GenericParam::Const(_cnst) => {
                    quote!()
                }
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
                        name = name
                    );
                        quote!(#[doc = #doc])
                    }
                }
            } else {
                quote!(#[doc(hidden)])
            };

            let (b_generics_impl, b_generics_ty, b_generics_where_extras_predicates) =
                b_generics.split_for_impl();
            let mut b_generics_where: syn::WhereClause = syn::parse2(quote! {
                where TypedBuilderFields: Clone
            })?;
            if let Some(predicates) = b_generics_where_extras_predicates {
                b_generics_where
                    .predicates
                    .extend(predicates.predicates.clone());
            }

            let can_memoize = match are_there_generics {
                true => quote! { false  },
                false => quote! { self == other },
            };

            let is_static = match are_there_generics {
                true => quote! { false  },
                false => quote! { true },
            };

            Ok(quote! {
                impl #impl_generics #name #ty_generics #where_clause {
                    #[doc = #builder_method_doc]
                    #[allow(dead_code)]
                    #vis fn builder() -> #builder_name #generics_with_empty {
                        #builder_name {
                            fields: #empties_tuple,
                            _phantom: core::default::Default::default(),
                        }
                    }
                }

                #[must_use]
                #builder_type_doc
                #[allow(dead_code, non_camel_case_types, non_snake_case)]
                #vis struct #builder_name #b_generics {
                    fields: #all_fields_param,
                    _phantom: (#( #phantom_generics ),*),
                }

                impl #b_generics_impl Clone for #builder_name #b_generics_ty #b_generics_where {
                    fn clone(&self) -> Self {
                        Self {
                            fields: self.fields.clone(),
                            _phantom: Default::default(),
                        }
                    }
                }

                impl #impl_generics dioxus::prelude::Properties for #name #ty_generics{
                    type Builder = #builder_name #generics_with_empty;
                    const IS_STATIC: bool = #is_static;
                    fn builder() -> Self::Builder {
                        #name::builder()
                    }
                    unsafe fn memoize(&self, other: &Self) -> bool {
                        #can_memoize
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

        pub fn field_impl(&self, field: &FieldInfo) -> Result<TokenStream, Error> {
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

            let &FieldInfo {
                name: ref field_name,
                ty: ref field_type,
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
            let doc = match field.builder_attr.doc {
                Some(ref doc) => quote!(#[doc = #doc]),
                None => quote!(),
            };

            // NOTE: both auto_into and strip_option affect `arg_type` and `arg_expr`, but the order of
            // nesting is different so we have to do this little dance.
            let arg_type = if field.builder_attr.strip_option {
                let internal_type = field.type_from_inside_option(false).ok_or_else(|| {
                    Error::new_spanned(
                        &field_type,
                        "can't `strip_option` - field is not `Option<...>`",
                    )
                })?;
                internal_type
            } else {
                field_type
            };
            let (arg_type, arg_expr) = if field.builder_attr.auto_into {
                (
                    quote!(impl core::convert::Into<#arg_type>),
                    quote!(#field_name.into()),
                )
            } else {
                (quote!(#arg_type), quote!(#field_name))
            };
            let arg_expr = if field.builder_attr.strip_option {
                quote!(Some(#arg_expr))
            } else {
                arg_expr
            };

            let repeated_fields_error_type_name = syn::Ident::new(
                &format!(
                    "{}_Error_Repeated_field_{}",
                    builder_name,
                    strip_raw_ident_prefix(field_name.to_string())
                ),
                proc_macro2::Span::call_site(),
            );
            let repeated_fields_error_message = format!("Repeated field {}", field_name);

            Ok(quote! {
                #[allow(dead_code, non_camel_case_types, missing_docs)]
                impl #impl_generics #builder_name < #( #ty_generics ),* > #where_clause {
                    #doc
                    pub fn #field_name (self, #field_name: #arg_type) -> #builder_name < #( #target_generics ),* > {
                        let #field_name = (#arg_expr,);
                        let ( #(#descructuring,)* ) = self.fields;
                        #builder_name {
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
                    pub fn #field_name (self, _: #repeated_fields_error_type_name) -> #builder_name < #( #target_generics ),* > {
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
                proc_macro2::Span::call_site(),
            );
            let early_build_error_message = format!("Missing required field {}", field_name);

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
                        panic!();
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

            let (_, ty_generics, where_clause) = self.generics.split_for_impl();

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
                if let Some(ref default) = field.builder_attr.default {
                    if field.builder_attr.skip {
                        quote!(let #name = #default;)
                    } else {
                        quote!(let #name = #helper_trait_name::into_value(#name, || #default);)
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
                            format!("Finalise the builder and create its [`{}`] instance", name);
                        quote!(#[doc = #doc])
                    }
                }
            } else {
                quote!()
            };
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
                if path_to_single_string(&attr.path).as_deref() != Some("builder") {
                    continue;
                }

                if attr.tokens.is_empty() {
                    continue;
                }
                let as_expr: syn::Expr = syn::parse2(attr.tokens.clone())?;

                match as_expr {
                    syn::Expr::Paren(body) => {
                        result.apply_meta(*body.expr)?;
                    }
                    syn::Expr::Tuple(body) => {
                        for expr in body.elems.into_iter() {
                            result.apply_meta(expr)?;
                        }
                    }
                    _ => {
                        return Err(Error::new_spanned(attr.tokens.clone(), "Expected (<...>)"));
                    }
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
                            format!("Unknown parameter {:?}", name),
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
                            format!("Unknown parameter {:?}", name),
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
                            format!("Illegal builder setting group {}", call_func),
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
                            format!("Illegal builder setting group name {}", subsetting_name),
                        )),
                    }
                }
                _ => Err(Error::new_spanned(expr, "Expected (<...>=<...>)")),
            }
        }
    }
}

/// Logic for declaratively injecting properties in custom components
pub mod injection {
    use std::borrow::BorrowMut;
    use std::collections::{HashMap, HashSet};
    use std::fmt;
    #[cfg(debug_assertions)]
    use std::fmt::Debug;
    use std::fmt::{Display, Formatter, Write};
    use std::ops::{Deref, DerefMut, Range, RangeFrom, RangeTo};
    use std::str::FromStr;
    use std::sync::{Mutex, Once};

    use proc_macro2::Span;
    use quote::ToTokens;

    use crate::props::field_info::{FieldBuilderAttr, FieldInfo};

    /// Stores declaratively defined property injection selectors for a custom component
    pub struct InjectedProperties(HashMap<String, PropertySelectors>);

    impl InjectedProperties {
        /// traverses and adds injected fields and their reciprocal selectors to the static cache
        pub fn add_injected_fields(component: &str, fields: &mut [FieldInfo]) -> syn::Result<()> {
            let injected_fields = fields.iter_mut().filter_map(
                |FieldInfo {
                     name,
                     ty,
                     builder_attr:
                         FieldBuilderAttr {
                             inject_as,
                             selectors,
                             strip_option,
                             ..
                         },
                     ..
                 }| {
                    // selectors are checked to be mutually exclusive during parsing
                    if selectors.is_some() {
                        Some((
                            name.clone(),
                            inject_as.take().unwrap_or_else(|| name.to_string()),
                            ty.to_token_stream().to_string().starts_with("EventHandler"),
                            selectors.take().unwrap_or_else(Selectors::new),
                            strip_option,
                        ))
                    } else {
                        None
                    }
                },
            );

            for (name, inject_as, handler, selector, strip_option) in injected_fields {
                InjectedProperties::add_selectors(
                    component,
                    if handler {
                        Property::Handler {
                            name: name.to_string(),
                            inject_as,
                            optional: *strip_option,
                        }
                    } else {
                        Property::Attribute {
                            name: name.to_string(),
                            inject_as,
                            optional: *strip_option,
                        }
                    },
                    selector,
                )
                .map_err(|err| syn::Error::new(proc_macro2::Span::call_site(), err))?;
            }

            /*
                        #[cfg(debug_assertions)]
                        InjectedProperties::debug(component)
                            .map_err(|err| syn::Error::new(proc_macro2::Span::call_site(), err))?;
            */

            Ok(())
        }

        /// Thread safe method for checking if a property applies to
        /// a child element/component branch
        pub fn check_branch(
            component: &str,
            property: &Property,
            branch: &Branch,
        ) -> Result<bool, String> {
            let components = Self::components().lock().map_err(|err| format!("{err}"))?;

            if let Some(property_selectors) = components.0.get(component) {
                if let Some(selectors) = property_selectors.get(property) {
                    return Ok(selectors.matches(branch));
                }
            }

            Ok(false)
        }

        pub fn component_properties<C: ToString>(component: &C) -> syn::Result<Vec<Property>> {
            let components = Self::components()
                .lock()
                .map_err(|err| syn::Error::new(Span::call_site(), format!("{err}")))?;

            if let Some(selector) = &components.0.get(component.to_string().as_str()) {
                Ok(selector
                    .iter()
                    .map(|(property, _)| property.clone())
                    .collect())
            } else {
                Ok(vec![])
            }
        }

        #[allow(dead_code)]
        #[cfg(debug_assertions)]
        pub fn debug(component: &str) -> Result<(), String> {
            let components = Self::components().lock().map_err(|err| format!("{err}"))?;
            let selectors = &components.0.get(component);
            let selectors = match selectors {
                Some(selectors) => *selectors,
                None => return Ok(()),
            };

            println!("{component}:");

            for (property, selector) in &selectors.0 {
                println!("  {property}: {selector:?}")
            }

            println!();

            Ok(())
        }

        /// Thread safe method for adding selectors for a property of a component;
        /// used during the derivation of Props trait for component properties
        fn add_selectors(
            component: &str,
            property: Property,
            selectors: Selectors,
        ) -> Result<(), String> {
            let mut components = Self::components().lock().map_err(|err| format!("{err}"))?;
            let component = components
                .0
                .entry(component.into())
                .or_insert_with(PropertySelectors::new);

            component.entry(property).or_insert(selectors);

            Ok(())
        }

        /// Thread safe static singleton instance of `ComponentProperties`
        fn components<'a>() -> &'a Mutex<Self> {
            static mut INNER: Option<Mutex<InjectedProperties>> = None;
            static INIT: Once = Once::new();

            // Since this access is inside a call_once, before any other accesses, it is safe
            INIT.call_once(|| unsafe {
                *INNER.borrow_mut() = Some(Mutex::new(Self(HashMap::new())))
            });

            // As long as this function is the only place with access to the static variable,
            // giving out a read-only borrow here is safe because it is guaranteed no more mutable
            // references will exist at this point or in the future.
            unsafe { INNER.as_ref().unwrap() }
        }
    }

    /// Actively traces the current branch as the document tree is traversed during parsing
    #[derive(Clone)]
    pub struct Branch {
        /// Segment traces of a document branch
        segments: Vec<SegmentTrace>,
    }

    #[cfg(debug_assertions)]
    impl Debug for Branch {
        fn fmt(&self, fmt: &mut Formatter<'_>) -> fmt::Result {
            for (count, segment) in self.segments.iter().enumerate() {
                if count > 0 {
                    fmt.write_str(" > ")?;
                }
                fmt.write_fmt(format_args!(
                    "{}:[{}][{}]",
                    segment.current,
                    segment.ordinal,
                    segment.counters.get(&segment.current).unwrap()
                ))?;
            }

            Ok(())
        }
    }

    impl Display for Branch {
        fn fmt(&self, fmt: &mut Formatter<'_>) -> fmt::Result {
            for (index, segment) in self.segments.iter().enumerate() {
                if index > 0 {
                    fmt.write_char(' ')?;
                }

                fmt.write_str(&segment.current)?;
            }

            Ok(())
        }
    }

    impl Branch {
        #[inline]
        #[must_use]
        pub const fn new() -> Self {
            Self {
                segments: Vec::new(),
            }
        }

        /// Adds a new child leve;, creates a new segment trace; only used
        /// to start new level, subsequent children need to use `Branch::sibling`
        pub fn child<N: ToString>(&mut self, name: N) {
            let mut counters = HashMap::default();

            counters.insert(name.to_string(), 0);

            self.segments.push(SegmentTrace {
                current: name.to_string(),
                ordinal: 0,
                counters,
            });
        }

        /// Indicates no more siblings; used after last sibling is added
        pub fn last(&mut self) -> Result<(), String> {
            if self.segments.is_empty() {
                Err(String::from(
                    "Branch traversal expected parent segment, ended unexpectedly",
                ))
            } else {
                self.segments.pop();

                Ok(())
            }
        }

        /// Adds next sibling, updates trace info
        pub fn sibling<N: ToString>(&mut self, name: N) -> Result<(), String> {
            if self.segments.is_empty() {
                Err(String::from(
                    "Branch is empty, start with a call to child first",
                ))
            } else {
                let trace = self.segments.last_mut().unwrap();

                trace.ordinal += 1;
                trace.current = name.to_string();

                trace
                    .counters
                    .entry(trace.current.clone())
                    .and_modify(|cnt| *cnt += 1)
                    .or_insert(0);

                Ok(())
            }
        }
    }

    /// Trace information of a segment of a `Branch`
    #[derive(Clone)]
    #[cfg_attr(debug_assertions, derive(Debug))]
    struct SegmentTrace {
        /// Current element/component; this changes as the branch traverses through document
        current: String,
        /// Position in list of siblings; only applies to `SegmentTrace::current`
        ordinal: usize,
        /// Trace of counters by element/component; used to determine the position in the
        /// list of siblings relative only to a specific element/component,
        /// i.e. nth div, instead of the nth sibling which also needs to be a div
        counters: HashMap<String, usize>,
    }

    impl SegmentTrace {
        /// Gets the position of the current element/component relative to siblings of
        /// the same type, i.e. nth div
        #[inline]
        #[must_use]
        #[allow(clippy::missing_panics_doc)]
        fn get_current_position(&self) -> usize {
            // a name always has an entry
            *self.counters.get(self.current.as_str()).unwrap()
        }
    }

    /// Declarative selectors of custom properties
    struct PropertySelectors(HashMap<Property, Selectors>);

    impl Deref for PropertySelectors {
        type Target = HashMap<Property, Selectors>;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    impl DerefMut for PropertySelectors {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.0
        }
    }

    impl PropertySelectors {
        #[inline]
        #[must_use]
        fn new() -> Self {
            Self(HashMap::new())
        }
    }

    #[derive(Debug, Clone, Eq, PartialEq, Hash)]
    pub enum Property {
        Attribute {
            name: String,
            inject_as: String,
            optional: bool,
        },
        Handler {
            name: String,
            inject_as: String,
            optional: bool,
        },
    }

    impl Display for Property {
        fn fmt(&self, fmt: &mut Formatter<'_>) -> fmt::Result {
            fmt.write_str(match self {
                Property::Attribute { name, .. } | Property::Handler { name, .. } => name,
            })
        }
    }

    /// Declarative selectors
    #[derive(Clone)]
    #[cfg_attr(debug_assertions, derive(Debug))]
    pub struct Selectors(HashMap<String, HashSet<Segments>>);

    impl Selectors {
        #[inline]
        #[must_use]
        fn new() -> Self {
            Self(HashMap::new())
        }

        /// Checks if a `Branch` matches any of the selector rules
        fn matches(&self, branch: &Branch) -> bool {
            let name = branch.to_string();

            match self.0.get(&name) {
                Some(segments) => segments
                    .iter()
                    .filter(|segments| segments.len() == branch.segments.len())
                    .any(|segments| {
                        segments
                            .deref()
                            .iter()
                            .zip(branch.segments.iter())
                            .all(|(lfh, rth)| lfh.matches(rth))
                    }),
                None => false,
            }
        }
    }

    /// Parses a declarative selector from a string
    impl FromStr for Selectors {
        type Err = String;

        fn from_str(value: &str) -> Result<Self, Self::Err> {
            // multiple selectors can be defined separated by a semicolon
            let result = value
                .split(';')
                .filter_map(|sel| {
                    if sel.trim().is_empty() {
                        None
                    } else {
                        Some(Selector::from_str(sel.trim()))
                    }
                })
                // an identifier key may represent multiple selectors, group them
                .fold(Ok(Self::new()), |acc, next| match acc {
                    Ok(mut acc) => match next {
                        Ok(next) => {
                            acc.0
                                .entry(next.identifier.clone())
                                .or_insert_with(HashSet::new)
                                .insert(next.segments);

                            Ok(acc)
                        }
                        Err(err) => Err(err),
                    },
                    err => err,
                });

            if let Ok(ok_result) = &result {
                if ok_result.0.is_empty() {
                    Err(String::from(
                        "selectors can not be empty, provide one or remove definition",
                    ))
                } else {
                    result
                }
            } else {
                result
            }
        }
    }

    /// Selector rules
    #[cfg_attr(debug_assertions, derive(Debug))]
    struct Selector {
        /// identifies selector rules; derived by striping out rules
        identifier: String,
        // rule for each segment of selector
        segments: Segments,
    }

    /// Parses a single declarative selector rule from a string
    impl FromStr for Selector {
        type Err = String;

        fn from_str(value: &str) -> Result<Self, Self::Err> {
            // a selector is broken down into segments, each separated by a `>`
            let segments = value
                .split('>')
                .map(Segment::from_str)
                .collect::<Result<Vec<_>, _>>()
                .map_err(|err| format!("'{value}' exception parsing selector; {err}"))?;
            let identifier = segments
                .iter()
                .map(|seg| match seg {
                    Segment::Component { name, .. } | Segment::Element { name, .. } => name,
                })
                .fold(String::new(), |mut acc, next| {
                    if !acc.is_empty() {
                        acc.push(' ');
                    }

                    acc.push_str(next);

                    acc
                });

            Ok(Self {
                identifier,
                segments: Segments(segments),
            })
        }
    }

    /// A collection of selector segment rules
    #[derive(Clone, Eq, PartialEq, Hash)]
    #[cfg_attr(debug_assertions, derive(Debug))]
    struct Segments(Vec<Segment>);

    impl Deref for Segments {
        type Target = Vec<Segment>;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    /// Identifies which position, absolute or relative, to use in a selector rule
    #[derive(Clone, Eq, PartialEq, Hash)]
    #[cfg_attr(debug_assertions, derive(Debug))]
    enum SelectorMode {
        /// Indicates the Nth rule should use the relative position
        /// of an element/component within a list of only like types
        NthElement,
        /// Indicates the Nth rule should use the absolute position
        /// of a sibling, disregarding element/component types
        NthSibling,
    }

    /// Defines the rules for a segment
    #[derive(Clone, Eq, PartialEq, Hash)]
    #[cfg_attr(debug_assertions, derive(Debug))]
    enum Segment {
        /// Identifies a component segment rule
        Component {
            /// name of component
            name: String,
            /// selection mode of segment
            mode: SelectorMode,
            /// selection rule of segment
            nth: Nth,
        },
        /// Identifies an element segment rule
        Element {
            /// name of element
            name: String,
            /// selection mode of segment
            mode: SelectorMode,
            /// selection rule of segment
            nth: Nth,
        },
    }

    /// Parses a segment rule from a string
    impl FromStr for Segment {
        type Err = String;

        fn from_str(value: &str) -> Result<Self, Self::Err> {
            let value = value.trim();

            if value.is_empty() {
                return Err(String::from("Segment can not be empty"));
            }

            // check if it is a component
            let is_component = value.chars().next().unwrap().is_uppercase();

            // determine selection mode
            let (mode, splitter) = if value.contains(':') {
                (SelectorMode::NthSibling, ':')
            } else {
                (SelectorMode::NthElement, '@')
            };

            Ok(match value.split_once(splitter) {
                None if is_component => Self::Component {
                    name: value.to_string(),
                    mode: SelectorMode::NthSibling,
                    nth: Nth::All,
                },
                None => Self::Element {
                    name: value.to_string(),
                    mode: SelectorMode::NthSibling,
                    nth: Nth::All,
                },
                Some((name, nth)) => {
                    let nth = Nth::from_str(
                        nth.strip_prefix('[')
                            .and_then(|v| v.strip_suffix(']'))
                            .ok_or_else(|| format!(
                                "'{nth}' is not a valid nth value, nth values need to be enclosed in brackets, i.e. elm_name:[nth]"
                            ))?
                    )?;

                    if is_component {
                        Self::Component {
                            name: name.to_string(),
                            mode,
                            nth,
                        }
                    } else {
                        Self::Element {
                            name: name.to_string(),
                            mode,
                            nth,
                        }
                    }
                }
            })
        }
    }

    impl Segment {
        /// Checks if a `SegmentTrace` of a `Branch` matches the `Segment`
        fn matches(&self, target: &SegmentTrace) -> bool {
            match self {
                Segment::Component { name, mode, nth } if *name == target.current => match mode {
                    SelectorMode::NthElement => nth.matches(target.get_current_position()),
                    SelectorMode::NthSibling => nth.matches(target.ordinal),
                },
                Segment::Element { name, mode, nth } if *name == target.current => match mode {
                    SelectorMode::NthElement => nth.matches(target.get_current_position()),
                    SelectorMode::NthSibling => nth.matches(target.ordinal),
                },
                _ => false,
            }
        }
    }

    /// Instance filter portion of `Segment` rule
    #[derive(Clone, Eq, PartialEq, Hash)]
    #[cfg_attr(debug_assertions, derive(Debug))]
    enum Nth {
        /// matches all instances
        All,
        /// matches even instances
        Even,
        /// matches odd instances
        Odd,
        /// matches every n instances
        EveryN(usize),
        /// matches a range of instances
        Range(Range<usize>),
        /// matches a range of instances starting from a position
        RangeFrom(RangeFrom<usize>),
        /// matches a range of instances up to a position
        RangeTo(RangeTo<usize>),
        /// matches a list of specific instances
        List(Vec<usize>),
    }

    impl Nth {
        /// Checks if a `SegmentTrace` position of a branches matches `Nth` rule
        fn matches(&self, position: usize) -> bool {
            match self {
                Nth::All => true,
                Nth::Even => position % 2 == 1,
                Nth::Odd => position % 2 == 0,
                Nth::EveryN(n) => position % n == n - 1,
                Nth::Range(range) => range.contains(&position),
                Nth::RangeFrom(range) => range.contains(&position),
                Nth::RangeTo(range) => range.contains(&position),
                Nth::List(list) => list.contains(&position),
            }
        }
    }

    /// Parses the position portion of a `Segment` rule from a string
    impl FromStr for Nth {
        type Err = String;

        #[allow(clippy::items_after_statements)]
        fn from_str(value: &str) -> Result<Self, Self::Err> {
            return Ok(match value.trim() {
                "even" => Self::Even,
                "odd" => Self::Odd,
                ".." => Self::All,
                nth if nth.ends_with('n') => {
                    let value = nth.strip_suffix('n').unwrap();

                    Self::EveryN(
                        usize::from_str(value)
                            .map_err(|_| format!("'{nth}' is not a valid nth value"))?,
                    )
                }
                range_to if range_to.starts_with("..") => {
                    let values = range_values(range_to)?;

                    Self::RangeTo(RangeTo { end: values[1] })
                }
                range_from if range_from.ends_with("..") => {
                    let values = range_values(range_from)?;

                    Self::RangeFrom(RangeFrom { start: values[0] })
                }
                range if range.contains("..") => {
                    let values = range_values(range)?;

                    let end = values[1];
                    let start = values[0];

                    if end < usize::max(start, 1) - 1 {
                        return if range.contains('=') {
                            Err(format!(
                                "Range start cannot be less than inclusive end; {} < {start}",
                                end - 1
                            ))
                        } else {
                            Err(format!(
                                "Range start cannot be less than end; {end} < {start}"
                            ))
                        };
                    }

                    Self::Range(Range { start, end })
                }
                list => Self::List(
                    list.split(',')
                        .map(usize::from_str)
                        .collect::<Result<Vec<_>, _>>()
                        .map_err(|_| format!("'{list}' is not a valid list of values"))?,
                ),
            });

            fn range_values(range: &str) -> Result<Vec<usize>, String> {
                range
                    .split("..")
                    .enumerate()
                    .take(2)
                    .map(|(idx, val)| {
                        let val = val.trim();

                        if val.is_empty() {
                            Ok(0)
                        } else if idx == 0 || !val.starts_with('=') {
                            usize::from_str(val)
                        } else {
                            usize::from_str(val.strip_prefix('=').unwrap()).map(|val| val + 1)
                        }
                    })
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|_| format!("'{range}' is not a valid range value"))
            }
        }
    }
}

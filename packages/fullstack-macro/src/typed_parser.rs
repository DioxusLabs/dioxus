use super::*;
use core::panic;
use proc_macro::TokenStream;
use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::ToTokens;
use quote::{format_ident, quote};
use std::collections::HashMap;
use syn::{
    braced, bracketed,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    token::{Comma, Slash},
    FnArg, GenericArgument, ItemFn, LitStr, Meta, PathArguments, Signature, Token, Type,
};
use syn::{spanned::Spanned, LitBool, LitInt, Pat, PatType};
use syn::{
    token::{Brace, Star},
    Attribute, Expr, ExprClosure, Lit,
};

pub fn route_impl(
    attr: TokenStream,
    item: TokenStream,
    with_aide: bool,
    method_from_macro: Option<Method>,
) -> syn::Result<TokenStream2> {
    let route = syn::parse::<Route>(attr)?;
    route_impl_with_route(route, item, with_aide, method_from_macro)
}

pub fn route_impl_with_route(
    route: Route,
    item: TokenStream,
    with_aide: bool,
    method_from_macro: Option<Method>,
) -> syn::Result<TokenStream2> {
    // Parse the route and function
    let function = syn::parse::<ItemFn>(item)?;

    let server_args = &route.server_args;
    let server_arg_tokens = quote! { #server_args  };

    let mut function_on_server = function.clone();
    function_on_server.sig.inputs.extend(server_args.clone());
    let server_idents = server_args
        .iter()
        .cloned()
        .filter_map(|arg| match arg {
            FnArg::Receiver(_) => None,
            FnArg::Typed(pat_type) => match &*pat_type.pat {
                Pat::Ident(pat_ident) => Some(pat_ident.ident.clone()),
                _ => None,
            },
        })
        .collect::<Vec<_>>();

    // Now we can compile the route
    let original_inputs = &function.sig.inputs;
    let route = CompiledRoute::from_route(route, &function, with_aide, method_from_macro)?;
    let path_extractor = route.path_extractor();
    let query_extractor = route.query_extractor();
    let query_params_struct = route.query_params_struct(with_aide);
    let state_type = &route.state;
    let axum_path = route.to_axum_path_string();
    let method_ident = &route.method;
    let http_method = route.method.to_axum_method_name();
    let remaining_numbered_pats = route.remaining_pattypes_numbered(&function.sig.inputs);
    let body_json_args = route.remaining_pattypes_named(&function.sig.inputs);
    let body_json_names = body_json_args.iter().map(|pat_type| &pat_type.pat);
    let body_json_types = body_json_args.iter().map(|pat_type| &pat_type.ty);
    let mut extracted_idents = route.extracted_idents();
    let remaining_numbered_idents = remaining_numbered_pats.iter().map(|pat_type| &pat_type.pat);
    let route_docs = route.to_doc_comments();

    extracted_idents.extend(body_json_names.clone().map(|pat| match pat.as_ref() {
        Pat::Ident(pat_ident) => pat_ident.ident.clone(),
        _ => panic!("Expected Pat::Ident"),
    }));
    extracted_idents.extend(server_idents);

    // Get the variables we need for code generation
    let fn_name = &function.sig.ident;
    let fn_output = &function.sig.output;
    let vis = &function.vis;
    let asyncness = &function.sig.asyncness;
    let (impl_generics, ty_generics, where_clause) = &function.sig.generics.split_for_impl();
    let ty_generics = ty_generics.as_turbofish();
    let fn_docs = function
        .attrs
        .iter()
        .filter(|attr| attr.path().is_ident("doc"));

    let (aide_ident_docs, inner_fn_call, method_router_ty) = if with_aide {
        let http_method = format_ident!("{}_with", http_method);
        let summary = route
            .get_oapi_summary()
            .map(|summary| quote! { .summary(#summary) });
        let description = route
            .get_oapi_description()
            .map(|description| quote! { .description(#description) });
        let hidden = route
            .get_oapi_hidden()
            .map(|hidden| quote! { .hidden(#hidden) });
        let tags = route.get_oapi_tags();
        let id = route
            .get_oapi_id(&function.sig)
            .map(|id| quote! { .id(#id) });
        let transform = route.get_oapi_transform()?;
        let responses = route.get_oapi_responses();
        let response_code = responses.iter().map(|response| &response.0);
        let response_type = responses.iter().map(|response| &response.1);
        let security = route.get_oapi_security();
        let schemes = security.iter().map(|sec| &sec.0);
        let scopes = security.iter().map(|sec| &sec.1);

        (
            route.ide_documentation_for_aide_methods(),
            quote! {
                ::aide::axum::routing::#http_method(
                    __inner__function__ #ty_generics,
                    |__op__| {
                        let __op__ = __op__
                            #summary
                            #description
                            #hidden
                            #id
                            #(.tag(#tags))*
                            #(.security_requirement_scopes::<Vec<&'static str>, _>(#schemes, vec![#(#scopes),*]))*
                            #(.response::<#response_code, #response_type>())*
                            ;
                        #transform
                        __op__
                    }
                )
            },
            quote! { ::aide::axum::routing::ApiMethodRouter },
        )
    } else {
        (
            quote!(),
            quote! { __axum::routing::#http_method(__inner__function__ #ty_generics) },
            quote! { __axum::routing::MethodRouter },
        )
    };

    let shadow_bind = original_inputs.iter().map(|arg| match arg {
        FnArg::Receiver(receiver) => todo!(),
        FnArg::Typed(pat_type) => {
            let pat = &pat_type.pat;
            quote! {
                let _ = #pat;
            }
        }
    });
    let value_bind = original_inputs.iter().map(|arg| match arg {
        FnArg::Receiver(receiver) => todo!(),
        FnArg::Typed(pat_type) => &pat_type.pat,
    });
    let shadow_bind2 = shadow_bind.clone();

    // #vis fn #fn_name #impl_generics() ->  #method_router_ty<#state_type> #where_clause {

    // let body_json_contents = remaining_numbered_pats.iter().map(|pat_type| [quote! {}]);
    let rest_idents = body_json_types.clone();
    let rest_ident_names2 = body_json_names.clone();
    let rest_ident_names3 = body_json_names.clone();

    let input_types = original_inputs.iter().map(|arg| match arg {
        FnArg::Receiver(_) => parse_quote! { () },
        FnArg::Typed(pat_type) => (*pat_type.ty).clone(),
    });

    let output_type = match &function.sig.output {
        syn::ReturnType::Default => parse_quote! { () },
        syn::ReturnType::Type(_, ty) => (*ty).clone(),
    };

    let query_param_names = route.query_params.iter().map(|(ident, _)| ident);

    let url_without_queries = route
        .route_lit
        .value()
        .split('?')
        .next()
        .unwrap()
        .to_string();

    let path_param_args = route.path_params.iter().map(|(_slash, param)| match param {
        PathParam::Capture(lit, _brace_1, ident, _ty, _brace_2) => {
            Some(quote! { #ident = #ident, })
        }
        PathParam::WildCard(lit, _brace_1, _star, ident, _ty, _brace_2) => {
            Some(quote! { #ident = #ident, })
        }
        PathParam::Static(lit) => None,
    });

    let query_param_names2 = query_param_names.clone();
    let request_url = quote! {
        format!(#url_without_queries, #( #path_param_args)*)
    };

    let out_ty = match output_type.as_ref() {
        Type::Tuple(tuple) if tuple.elems.is_empty() => parse_quote! { () },
        _ => output_type.clone(),
    };

    Ok(quote! {
        #(#fn_docs)*
        #route_docs
        #vis async fn #fn_name #impl_generics(
            #original_inputs
        ) #fn_output #where_clause {
            use dioxus_fullstack::reqwest as __reqwest;
            use dioxus_fullstack::serde as serde;
            use dioxus_fullstack::{
                DeSer,  ClientRequest, ExtractState, ExtractRequest, EncodeState,
                ServerFnSugar, ServerFnRejection, EncodeRequest, get_server_url, EncodedBody,
                ServerFnError,
            };


            #query_params_struct

            // On the client, we make the request to the server
            if cfg!(not(feature = "server")) {
                let __params = __QueryParams__ {
                    #(#query_param_names,)*
                };

                let client = __reqwest::Client::new()
                    .post(format!("{}{}", get_server_url(), #request_url))
                    .query(&__params);

                let encode_state = EncodeState {
                    client
                };

                return (&&&&&&&&&&&&&&ClientRequest::<(#(#rest_idents,)*), #out_ty, _>::new())
                        .fetch(encode_state, (#(#rest_ident_names2,)*))
                        .await;
            }

            // On the server, we expand the tokens and submit the function to inventory
            #[cfg(feature = "server")] {
                use dioxus_fullstack::inventory as __inventory;
                use dioxus_fullstack::axum as __axum;
                use dioxus_fullstack::http as __http;
                use __axum::response::IntoResponse;
                use dioxus_server::ServerFunction;

                #aide_ident_docs
                #asyncness fn __inner__function__ #impl_generics(
                    #path_extractor
                    #query_extractor
                    #server_arg_tokens
                ) -> __axum::response::Response #where_clause {
                    let ( #(#body_json_names,)*) = match (&&&&&&&&&&&&&&DeSer::<(#(#body_json_types,)*), _>::new()).extract(ExtractState::default()).await {
                        Ok(v) => v,
                        Err(rejection) => return rejection.into_response()
                    };

                    #function_on_server

                    #fn_name #ty_generics(#(#extracted_idents,)*).await.desugar_into_response()

                    // #[__axum::debug_handler]
                    // body: Json<__BodyExtract__>,
                    // #remaining_numbered_pats
                    // let __BodyExtract__ { #(#body_json_names,)* } = body.0;
                    // ) #fn_output #where_clause {
                    // let __res = #fn_name #ty_generics(#(#extracted_idents,)* #(#remaining_numbered_idents,)* ).await;
                    // serverfn_sugar()
                    // desugar_into_response will autoref into using the Serialize impl
                    // #fn_name #ty_generics(#(#extracted_idents,)* #(#remaining_numbered_idents,)* ).await.desugar_into_response()
                    // #fn_name #ty_generics(#(#extracted_idents,)*  #(#body_json_names2,)* ).await.desugar_into_response()
                    // #fn_name #ty_generics(#(#extracted_idents,)* Json(__BodyExtract__::new()) ).await.desugar_into_response()
                    // #fn_name #ty_generics(#(#extracted_idents,)* #(#remaining_numbered_idents,)* ).await.desugar_into_response()
                }

                __inventory::submit! {
                    ServerFunction::new(__http::Method::#method_ident, #axum_path, || #inner_fn_call)
                }

                todo!("Calling server_fn on server is not yet supported. todo.");
            }

            #[allow(unreachable_code)]
            {
                unreachable!()
            }
        }
    })
}

pub struct CompiledRoute {
    pub method: Method,
    #[allow(clippy::type_complexity)]
    pub path_params: Vec<(Slash, PathParam)>,
    pub query_params: Vec<(Ident, Box<Type>)>,
    pub state: Type,
    pub route_lit: LitStr,
    pub oapi_options: Option<OapiOptions>,
}

impl CompiledRoute {
    pub fn to_axum_path_string(&self) -> String {
        let mut path = String::new();

        for (_slash, param) in &self.path_params {
            path.push('/');
            match param {
                PathParam::Capture(lit, _brace_1, _, _, _brace_2) => {
                    path.push('{');
                    path.push_str(&lit.value());
                    path.push('}');
                }
                PathParam::WildCard(lit, _brace_1, _, _, _, _brace_2) => {
                    path.push('{');
                    path.push('*');
                    path.push_str(&lit.value());
                    path.push('}');
                }
                PathParam::Static(lit) => path.push_str(&lit.value()),
            }
            // if colon.is_some() {
            //     path.push(':');
            // }
            // path.push_str(&ident.value());
        }

        path
    }

    /// Removes the arguments in `route` from `args`, and merges them in the output.
    pub fn from_route(
        mut route: Route,
        function: &ItemFn,
        with_aide: bool,
        method_from_macro: Option<Method>,
    ) -> syn::Result<Self> {
        if !with_aide && route.oapi_options.is_some() {
            return Err(syn::Error::new(
                Span::call_site(),
                "Use `api_route` instead of `route` to use OpenAPI options",
            ));
        } else if with_aide && route.oapi_options.is_none() {
            route.oapi_options = Some(OapiOptions {
                summary: None,
                description: None,
                id: None,
                hidden: None,
                tags: None,
                security: None,
                responses: None,
                transform: None,
            });
        }

        let sig = &function.sig;
        let mut arg_map = sig
            .inputs
            .iter()
            .filter_map(|item| match item {
                syn::FnArg::Receiver(_) => None,
                syn::FnArg::Typed(pat_type) => Some(pat_type),
            })
            .filter_map(|pat_type| match &*pat_type.pat {
                syn::Pat::Ident(ident) => Some((ident.ident.clone(), pat_type.ty.clone())),
                _ => None,
            })
            .collect::<HashMap<_, _>>();

        for (_slash, path_param) in &mut route.path_params {
            match path_param {
                PathParam::Capture(_lit, _, ident, ty, _) => {
                    let (new_ident, new_ty) = arg_map.remove_entry(ident).ok_or_else(|| {
                        syn::Error::new(
                            ident.span(),
                            format!("path parameter `{}` not found in function arguments", ident),
                        )
                    })?;
                    *ident = new_ident;
                    *ty = new_ty;
                }
                PathParam::WildCard(_lit, _, _star, ident, ty, _) => {
                    let (new_ident, new_ty) = arg_map.remove_entry(ident).ok_or_else(|| {
                        syn::Error::new(
                            ident.span(),
                            format!("path parameter `{}` not found in function arguments", ident),
                        )
                    })?;
                    *ident = new_ident;
                    *ty = new_ty;
                }
                PathParam::Static(_lit) => {}
            }
        }

        let mut query_params = Vec::new();
        for ident in route.query_params {
            let (ident, ty) = arg_map.remove_entry(&ident).ok_or_else(|| {
                syn::Error::new(
                    ident.span(),
                    format!(
                        "query parameter `{}` not found in function arguments",
                        ident
                    ),
                )
            })?;
            query_params.push((ident, ty));
        }

        if let Some(options) = route.oapi_options.as_mut() {
            options.merge_with_fn(function)
        }

        let method = match (method_from_macro, route.method) {
            (Some(method), None) => method,
            (None, Some(method)) => method,
            (Some(_), Some(_)) => {
                return Err(syn::Error::new(
                    Span::call_site(),
                    "HTTP method specified both in macro and in attribute",
                ))
            }
            (None, None) => {
                return Err(syn::Error::new(
                    Span::call_site(),
                    "HTTP method not specified in macro or in attribute",
                ))
            }
        };

        Ok(Self {
            method,
            route_lit: route.route_lit,
            path_params: route.path_params,
            query_params,
            state: route.state.unwrap_or_else(|| guess_state_type(sig)),
            oapi_options: route.oapi_options,
        })
    }

    pub fn path_extractor(&self) -> Option<TokenStream2> {
        if !self.path_params.iter().any(|(_, param)| param.captures()) {
            return None;
        }

        let path_iter = self
            .path_params
            .iter()
            .filter_map(|(_slash, path_param)| path_param.capture());
        let idents = path_iter.clone().map(|item| item.0);
        let types = path_iter.clone().map(|item| item.1);
        Some(quote! {
            __axum::extract::Path((#(#idents,)*)): __axum::extract::Path<(#(#types,)*)>,
        })
    }

    pub fn query_extractor(&self) -> Option<TokenStream2> {
        if self.query_params.is_empty() {
            return None;
        }

        let idents = self.query_params.iter().map(|item| &item.0);
        Some(quote! {
            __axum::extract::Query(__QueryParams__ {
                #(#idents,)*
            }): __axum::extract::Query<__QueryParams__>,
        })
    }

    pub fn query_params_struct(&self, with_aide: bool) -> Option<TokenStream2> {
        // match self.query_params.is_empty() {
        //     true => None,
        //     false => {
        let idents = self.query_params.iter().map(|item| &item.0);
        let types = self.query_params.iter().map(|item| &item.1);
        let derive = match with_aide {
            true => {
                quote! { #[derive(serde::Deserialize, serde::Serialize, ::schemars::JsonSchema)] }
            }
            false => quote! { #[derive(serde::Deserialize, serde::Serialize)] },
        };
        Some(quote! {
            #derive
            struct __QueryParams__ {
                #(#idents: #types,)*
            }
        })
        // }
        // }
    }

    pub fn extracted_idents(&self) -> Vec<Ident> {
        let mut idents = Vec::new();
        for (_slash, path_param) in &self.path_params {
            if let Some((ident, _ty)) = path_param.capture() {
                idents.push(ident.clone());
            }
        }
        for (ident, _ty) in &self.query_params {
            idents.push(ident.clone());
        }
        idents
    }

    fn remaining_pattypes_named(
        &self,
        args: &Punctuated<FnArg, Comma>,
    ) -> Punctuated<PatType, Comma> {
        args.iter()
            .enumerate()
            .filter_map(|(i, item)| {
                if let FnArg::Typed(pat_type) = item {
                    if let syn::Pat::Ident(pat_ident) = &*pat_type.pat {
                        if self.path_params.iter().any(|(_slash, path_param)| {
                            if let Some((path_ident, _ty)) = path_param.capture() {
                                path_ident == &pat_ident.ident
                            } else {
                                false
                            }
                        }) || self
                            .query_params
                            .iter()
                            .any(|(query_ident, _)| query_ident == &pat_ident.ident)
                        {
                            return None;
                        }
                    }

                    Some(pat_type.clone())
                } else {
                    unimplemented!("Self type is not supported")
                }
            })
            .collect()
    }

    /// The arguments not used in the route.
    /// Map the identifier to `___arg___{i}: Type`.
    pub fn remaining_pattypes_numbered(
        &self,
        args: &Punctuated<FnArg, Comma>,
    ) -> Punctuated<PatType, Comma> {
        args.iter()
            .enumerate()
            .filter_map(|(i, item)| {
                if let FnArg::Typed(pat_type) = item {
                    if let syn::Pat::Ident(pat_ident) = &*pat_type.pat {
                        if self.path_params.iter().any(|(_slash, path_param)| {
                            if let Some((path_ident, _ty)) = path_param.capture() {
                                path_ident == &pat_ident.ident
                            } else {
                                false
                            }
                        }) || self
                            .query_params
                            .iter()
                            .any(|(query_ident, _)| query_ident == &pat_ident.ident)
                        {
                            return None;
                        }
                    }

                    let mut new_pat_type = pat_type.clone();
                    let ident = format_ident!("___arg___{}", i);
                    new_pat_type.pat = Box::new(parse_quote!(#ident));
                    Some(new_pat_type)
                } else {
                    unimplemented!("Self type is not supported")
                }
            })
            .collect()
    }

    pub fn ide_documentation_for_aide_methods(&self) -> TokenStream2 {
        let Some(options) = &self.oapi_options else {
            return quote! {};
        };
        let summary = options.summary.as_ref().map(|(ident, _)| {
            let method = Ident::new("summary", ident.span());
            quote!( let x = x.#method(""); )
        });
        let description = options.description.as_ref().map(|(ident, _)| {
            let method = Ident::new("description", ident.span());
            quote!( let x = x.#method(""); )
        });
        let id = options.id.as_ref().map(|(ident, _)| {
            let method = Ident::new("id", ident.span());
            quote!( let x = x.#method(""); )
        });
        let hidden = options.hidden.as_ref().map(|(ident, _)| {
            let method = Ident::new("hidden", ident.span());
            quote!( let x = x.#method(false); )
        });
        let tags = options.tags.as_ref().map(|(ident, _)| {
            let method = Ident::new("tag", ident.span());
            quote!( let x = x.#method(""); )
        });
        let security = options.security.as_ref().map(|(ident, _)| {
            let method = Ident::new("security_requirement_scopes", ident.span());
            quote!( let x = x.#method("", [""]); )
        });
        let responses = options.responses.as_ref().map(|(ident, _)| {
            let method = Ident::new("response", ident.span());
            quote!( let x = x.#method::<0, String>(); )
        });
        let transform = options.transform.as_ref().map(|(ident, _)| {
            let method = Ident::new("with", ident.span());
            quote!( let x = x.#method(|x|x); )
        });

        quote! {
            #[allow(unused)]
            #[allow(clippy::no_effect)]
            fn ____ide_documentation_for_aide____(x: ::aide::transform::TransformOperation) {
                #summary
                #description
                #id
                #hidden
                #tags
                #security
                #responses
                #transform
            }
        }
    }

    pub fn get_oapi_summary(&self) -> Option<LitStr> {
        if let Some(oapi_options) = &self.oapi_options {
            if let Some(summary) = &oapi_options.summary {
                return Some(summary.1.clone());
            }
        }
        None
    }

    pub fn get_oapi_description(&self) -> Option<LitStr> {
        if let Some(oapi_options) = &self.oapi_options {
            if let Some(description) = &oapi_options.description {
                return Some(description.1.clone());
            }
        }
        None
    }

    pub fn get_oapi_hidden(&self) -> Option<LitBool> {
        if let Some(oapi_options) = &self.oapi_options {
            if let Some(hidden) = &oapi_options.hidden {
                return Some(hidden.1.clone());
            }
        }
        None
    }

    pub fn get_oapi_tags(&self) -> Vec<LitStr> {
        if let Some(oapi_options) = &self.oapi_options {
            if let Some(tags) = &oapi_options.tags {
                return tags.1 .0.clone();
            }
        }
        Vec::new()
    }

    pub fn get_oapi_id(&self, sig: &Signature) -> Option<LitStr> {
        if let Some(oapi_options) = &self.oapi_options {
            if let Some(id) = &oapi_options.id {
                return Some(id.1.clone());
            }
        }
        Some(LitStr::new(&sig.ident.to_string(), sig.ident.span()))
    }

    pub fn get_oapi_transform(&self) -> syn::Result<Option<TokenStream2>> {
        if let Some(oapi_options) = &self.oapi_options {
            if let Some(transform) = &oapi_options.transform {
                if transform.1.inputs.len() != 1 {
                    return Err(syn::Error::new(
                        transform.1.span(),
                        "expected a single identifier",
                    ));
                }

                let pat = transform.1.inputs.first().unwrap();
                let body = &transform.1.body;

                if let Pat::Ident(pat_ident) = pat {
                    let ident = &pat_ident.ident;
                    return Ok(Some(quote! {
                        let #ident = __op__;
                        let __op__ = #body;
                    }));
                } else {
                    return Err(syn::Error::new(
                        pat.span(),
                        "expected a single identifier without type",
                    ));
                }
            }
        }
        Ok(None)
    }

    pub fn get_oapi_responses(&self) -> Vec<(LitInt, Type)> {
        if let Some(oapi_options) = &self.oapi_options {
            if let Some((_ident, Responses(responses))) = &oapi_options.responses {
                return responses.clone();
            }
        }
        Default::default()
    }

    pub fn get_oapi_security(&self) -> Vec<(LitStr, Vec<LitStr>)> {
        if let Some(oapi_options) = &self.oapi_options {
            if let Some((_ident, Security(security))) = &oapi_options.security {
                return security
                    .iter()
                    .map(|(scheme, StrArray(scopes))| (scheme.clone(), scopes.clone()))
                    .collect();
            }
        }
        Default::default()
    }

    pub(crate) fn to_doc_comments(&self) -> TokenStream2 {
        let mut doc = format!(
            "# Handler information
- Method: `{}`
- Path: `{}`
- State: `{}`",
            self.method.to_axum_method_name(),
            self.route_lit.value(),
            self.state.to_token_stream(),
        );

        if let Some(options) = &self.oapi_options {
            let summary = options
                .summary
                .as_ref()
                .map(|(_, summary)| format!("\"{}\"", summary.value()))
                .unwrap_or("None".to_string());
            let description = options
                .description
                .as_ref()
                .map(|(_, description)| format!("\"{}\"", description.value()))
                .unwrap_or("None".to_string());
            let id = options
                .id
                .as_ref()
                .map(|(_, id)| format!("\"{}\"", id.value()))
                .unwrap_or("None".to_string());
            let hidden = options
                .hidden
                .as_ref()
                .map(|(_, hidden)| hidden.value().to_string())
                .unwrap_or("None".to_string());
            let tags = options
                .tags
                .as_ref()
                .map(|(_, tags)| tags.to_string())
                .unwrap_or("[]".to_string());
            let security = options
                .security
                .as_ref()
                .map(|(_, security)| security.to_string())
                .unwrap_or("{}".to_string());

            doc = format!(
                "{doc}

## OpenAPI
- Summary: `{summary}`
- Description: `{description}`
- Operation id: `{id}`
- Tags: `{tags}`
- Security: `{security}`
- Hidden: `{hidden}`
"
            );
        }

        quote!(
            #[doc = #doc]
        )
    }
}

fn guess_state_type(sig: &syn::Signature) -> Type {
    for arg in &sig.inputs {
        if let FnArg::Typed(pat_type) = arg {
            // Returns `T` if the type of the last segment is exactly `State<T>`.
            if let Type::Path(ty) = &*pat_type.ty {
                let last_segment = ty.path.segments.last().unwrap();
                if last_segment.ident == "State" {
                    if let PathArguments::AngleBracketed(args) = &last_segment.arguments {
                        if args.args.len() == 1 {
                            if let GenericArgument::Type(ty) = args.args.first().unwrap() {
                                return ty.clone();
                            }
                        }
                    }
                }
            }
        }
    }

    parse_quote! { () }
}

struct RouteParser {
    path_params: Vec<(Slash, PathParam)>,
    query_params: Vec<Ident>,
}

impl RouteParser {
    fn new(lit: LitStr) -> syn::Result<Self> {
        let val = lit.value();
        let span = lit.span();
        let split_route = val.split('?').collect::<Vec<_>>();
        if split_route.len() > 2 {
            return Err(syn::Error::new(span, "expected at most one '?'"));
        }

        let path = split_route[0];
        if !path.starts_with('/') {
            return Err(syn::Error::new(span, "expected path to start with '/'"));
        }
        let path = path.strip_prefix('/').unwrap();

        let mut path_params = Vec::new();

        for path_param in path.split('/') {
            path_params.push((
                Slash(span),
                PathParam::new(path_param, span, Box::new(parse_quote!(())))?,
            ));
        }

        let path_param_len = path_params.len();
        for (i, (_slash, path_param)) in path_params.iter().enumerate() {
            match path_param {
                PathParam::WildCard(_, _, _, _, _, _) => {
                    if i != path_param_len - 1 {
                        return Err(syn::Error::new(
                            span,
                            "wildcard path param must be the last path param",
                        ));
                    }
                }
                PathParam::Capture(_, _, _, _, _) => (),
                PathParam::Static(lit) => {
                    if lit.value() == "*" && i != path_param_len - 1 {
                        return Err(syn::Error::new(
                            span,
                            "wildcard path param must be the last path param",
                        ));
                    }
                }
            }
        }

        let mut query_params = Vec::new();
        if split_route.len() == 2 {
            let query = split_route[1];
            for query_param in query.split('&') {
                query_params.push(Ident::new(query_param, span));
            }
        }

        Ok(Self {
            path_params,
            query_params,
        })
    }
}

pub enum PathParam {
    WildCard(LitStr, Brace, Star, Ident, Box<Type>, Brace),
    Capture(LitStr, Brace, Ident, Box<Type>, Brace),
    Static(LitStr),
}

impl PathParam {
    pub fn captures(&self) -> bool {
        matches!(self, Self::Capture(..) | Self::WildCard(..))
    }

    pub fn capture(&self) -> Option<(&Ident, &Type)> {
        match self {
            Self::Capture(_, _, ident, ty, _) => Some((ident, ty)),
            Self::WildCard(_, _, _, ident, ty, _) => Some((ident, ty)),
            _ => None,
        }
    }

    fn new(str: &str, span: Span, ty: Box<Type>) -> syn::Result<Self> {
        let ok = if str.starts_with('{') {
            let str = str
                .strip_prefix('{')
                .unwrap()
                .strip_suffix('}')
                .ok_or_else(|| {
                    syn::Error::new(span, "expected path param to be wrapped in curly braces")
                })?;
            Self::Capture(
                LitStr::new(str, span),
                Brace(span),
                Ident::new(str, span),
                ty,
                Brace(span),
            )
        } else if str.starts_with('*') && str.len() > 1 {
            let str = str.strip_prefix('*').unwrap();
            Self::WildCard(
                LitStr::new(str, span),
                Brace(span),
                Star(span),
                Ident::new(str, span),
                ty,
                Brace(span),
            )
        } else {
            Self::Static(LitStr::new(str, span))
        };

        Ok(ok)
    }
}

pub struct OapiOptions {
    pub summary: Option<(Ident, LitStr)>,
    pub description: Option<(Ident, LitStr)>,
    pub id: Option<(Ident, LitStr)>,
    pub hidden: Option<(Ident, LitBool)>,
    pub tags: Option<(Ident, StrArray)>,
    pub security: Option<(Ident, Security)>,
    pub responses: Option<(Ident, Responses)>,
    pub transform: Option<(Ident, ExprClosure)>,
}

pub struct Security(pub Vec<(LitStr, StrArray)>);
impl Parse for Security {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let inner;
        braced!(inner in input);

        let mut arr = Vec::new();
        while !inner.is_empty() {
            let scheme = inner.parse::<LitStr>()?;
            let _ = inner.parse::<Token![:]>()?;
            let scopes = inner.parse::<StrArray>()?;
            let _ = inner.parse::<Token![,]>().ok();
            arr.push((scheme, scopes));
        }

        Ok(Self(arr))
    }
}

impl ToString for Security {
    fn to_string(&self) -> String {
        let mut s = String::new();
        s.push('{');
        for (i, (scheme, scopes)) in self.0.iter().enumerate() {
            if i > 0 {
                s.push_str(", ");
            }
            s.push_str(&scheme.value());
            s.push_str(": ");
            s.push_str(&scopes.to_string());
        }
        s.push('}');
        s
    }
}

pub struct Responses(pub Vec<(LitInt, Type)>);
impl Parse for Responses {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let inner;
        braced!(inner in input);

        let mut arr = Vec::new();
        while !inner.is_empty() {
            let status = inner.parse::<LitInt>()?;
            let _ = inner.parse::<Token![:]>()?;
            let ty = inner.parse::<Type>()?;
            let _ = inner.parse::<Token![,]>().ok();
            arr.push((status, ty));
        }

        Ok(Self(arr))
    }
}

impl ToString for Responses {
    fn to_string(&self) -> String {
        let mut s = String::new();
        s.push('{');
        for (i, (status, ty)) in self.0.iter().enumerate() {
            if i > 0 {
                s.push_str(", ");
            }
            s.push_str(&status.to_string());
            s.push_str(": ");
            s.push_str(&ty.to_token_stream().to_string());
        }
        s.push('}');
        s
    }
}

#[derive(Clone)]
pub struct StrArray(pub Vec<LitStr>);
impl Parse for StrArray {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let inner;
        bracketed!(inner in input);
        let mut arr = Vec::new();
        while !inner.is_empty() {
            arr.push(inner.parse::<LitStr>()?);
            inner.parse::<Token![,]>().ok();
        }
        Ok(Self(arr))
    }
}

impl ToString for StrArray {
    fn to_string(&self) -> String {
        let mut s = String::new();
        s.push('[');
        for (i, lit) in self.0.iter().enumerate() {
            if i > 0 {
                s.push_str(", ");
            }
            s.push('"');
            s.push_str(&lit.value());
            s.push('"');
        }
        s.push(']');
        s
    }
}

impl Parse for OapiOptions {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut this = Self {
            summary: None,
            description: None,
            id: None,
            hidden: None,
            tags: None,
            security: None,
            responses: None,
            transform: None,
        };

        while !input.is_empty() {
            let ident = input.parse::<Ident>()?;
            let _ = input.parse::<Token![:]>()?;
            match ident.to_string().as_str() {
                "summary" => this.summary = Some((ident, input.parse()?)),
                "description" => this.description = Some((ident, input.parse()?)),
                "id" => this.id = Some((ident, input.parse()?)),
                "hidden" => this.hidden = Some((ident, input.parse()?)),
                "tags" => this.tags = Some((ident, input.parse()?)),
                "security" => this.security = Some((ident, input.parse()?)),
                "responses" => this.responses = Some((ident, input.parse()?)),
                "transform" => this.transform = Some((ident, input.parse()?)),
                _ => {
                    return Err(syn::Error::new(
                        ident.span(),
                        "unexpected field, expected one of (summary, description, id, hidden, tags, security, responses, transform)",
                    ))
                }
            }
            let _ = input.parse::<Token![,]>().ok();
        }

        Ok(this)
    }
}

impl OapiOptions {
    pub fn merge_with_fn(&mut self, function: &ItemFn) {
        if self.description.is_none() {
            self.description = doc_iter(&function.attrs)
                .skip(2)
                .map(|item| item.value())
                .reduce(|mut acc, item| {
                    acc.push('\n');
                    acc.push_str(&item);
                    acc
                })
                .map(|item| (parse_quote!(description), parse_quote!(#item)))
        }
        if self.summary.is_none() {
            self.summary = doc_iter(&function.attrs)
                .next()
                .map(|item| (parse_quote!(summary), item.clone()))
        }
        if self.id.is_none() {
            let id = &function.sig.ident;
            self.id = Some((parse_quote!(id), LitStr::new(&id.to_string(), id.span())));
        }
    }
}

fn doc_iter(attrs: &[Attribute]) -> impl Iterator<Item = &LitStr> + '_ {
    attrs
        .iter()
        .filter(|attr| attr.path().is_ident("doc"))
        .map(|attr| {
            let Meta::NameValue(meta) = &attr.meta else {
                panic!("doc attribute is not a name-value attribute");
            };
            let Expr::Lit(lit) = &meta.value else {
                panic!("doc attribute is not a string literal");
            };
            let Lit::Str(lit_str) = &lit.lit else {
                panic!("doc attribute is not a string literal");
            };
            lit_str
        })
}

pub struct Route {
    pub method: Option<Method>,
    pub path_params: Vec<(Slash, PathParam)>,
    pub query_params: Vec<Ident>,
    pub state: Option<Type>,
    pub route_lit: LitStr,
    pub oapi_options: Option<OapiOptions>,
    pub server_args: Punctuated<FnArg, Comma>,
}

impl Parse for Route {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let method = if input.peek(Ident) {
            Some(input.parse::<Method>()?)
        } else {
            None
        };

        let route_lit = input.parse::<LitStr>()?;
        let route_parser = RouteParser::new(route_lit.clone())?;
        // let state = match input.parse::<kw::with>() {
        //     Ok(_) => Some(input.parse::<Type>()?),
        //     Err(_) => None,
        // };
        let state = None;
        let oapi_options = input
            .peek(Brace)
            .then(|| {
                let inner;
                braced!(inner in input);
                inner.parse::<OapiOptions>()
            })
            .transpose()?;

        let server_args = if input.peek(Comma) {
            let _ = input.parse::<Comma>()?;
            input.parse_terminated(FnArg::parse, Comma)?
        } else {
            Punctuated::new()
        };

        Ok(Route {
            method,
            path_params: route_parser.path_params,
            query_params: route_parser.query_params,
            state,
            route_lit,
            oapi_options,
            server_args,
        })
    }
}

#[derive(Clone)]
pub enum Method {
    Get(Ident),
    Post(Ident),
    Put(Ident),
    Delete(Ident),
    Head(Ident),
    Connect(Ident),
    Options(Ident),
    Trace(Ident),
}

impl ToTokens for Method {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        match self {
            Self::Get(ident)
            | Self::Post(ident)
            | Self::Put(ident)
            | Self::Delete(ident)
            | Self::Head(ident)
            | Self::Connect(ident)
            | Self::Options(ident)
            | Self::Trace(ident) => {
                ident.to_tokens(tokens);
            }
        }
    }
}

impl Parse for Method {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let ident = input.parse::<Ident>()?;
        match ident.to_string().to_uppercase().as_str() {
            "GET" => Ok(Self::Get(ident)),
            "POST" => Ok(Self::Post(ident)),
            "PUT" => Ok(Self::Put(ident)),
            "DELETE" => Ok(Self::Delete(ident)),
            "HEAD" => Ok(Self::Head(ident)),
            "CONNECT" => Ok(Self::Connect(ident)),
            "OPTIONS" => Ok(Self::Options(ident)),
            "TRACE" => Ok(Self::Trace(ident)),
            _ => Err(input
                .error("expected one of (GET, POST, PUT, DELETE, HEAD, CONNECT, OPTIONS, TRACE)")),
        }
    }
}

impl Method {
    pub fn to_axum_method_name(&self) -> Ident {
        match self {
            Self::Get(span) => Ident::new("get", span.span()),
            Self::Post(span) => Ident::new("post", span.span()),
            Self::Put(span) => Ident::new("put", span.span()),
            Self::Delete(span) => Ident::new("delete", span.span()),
            Self::Head(span) => Ident::new("head", span.span()),
            Self::Connect(span) => Ident::new("connect", span.span()),
            Self::Options(span) => Ident::new("options", span.span()),
            Self::Trace(span) => Ident::new("trace", span.span()),
        }
    }

    pub fn new_from_string(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "GET" => Self::Get(Ident::new("GET", Span::call_site())),
            "POST" => Self::Post(Ident::new("POST", Span::call_site())),
            "PUT" => Self::Put(Ident::new("PUT", Span::call_site())),
            "DELETE" => Self::Delete(Ident::new("DELETE", Span::call_site())),
            "HEAD" => Self::Head(Ident::new("HEAD", Span::call_site())),
            "CONNECT" => Self::Connect(Ident::new("CONNECT", Span::call_site())),
            "OPTIONS" => Self::Options(Ident::new("OPTIONS", Span::call_site())),
            "TRACE" => Self::Trace(Ident::new("TRACE", Span::call_site())),
            _ => panic!("expected one of (GET, POST, PUT, DELETE, HEAD, CONNECT, OPTIONS, TRACE)"),
        }
    }
}

mod kw {
    syn::custom_keyword!(with);
}

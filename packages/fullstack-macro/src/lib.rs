// TODO: Create README, uncomment this: #![doc = include_str!("../README.md")]
#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]

use core::panic;
use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::ToTokens;
use quote::{format_ident, quote};
use std::collections::HashMap;
use syn::{
    braced, bracketed,
    parse::ParseStream,
    punctuated::Punctuated,
    token::{Comma, Slash},
    Error, ExprTuple, FnArg, GenericArgument, Meta, PathArguments, PathSegment, Signature, Token,
    Type, TypePath,
};
use syn::{parse::Parse, parse_quote, Ident, ItemFn, LitStr, Path};
use syn::{spanned::Spanned, LitBool, LitInt, Pat, PatType};
use syn::{
    token::{Brace, Star},
    Attribute, Expr, ExprClosure, Lit, Result,
};

/// ## Usage
///
/// ```rust,ignore
/// # use dioxus::prelude::*;
/// # #[derive(serde::Deserialize, serde::Serialize)]
/// # struct BlogPost;
/// # async fn load_posts(category: &str) -> Result<Vec<BlogPost>> { unimplemented!() }
///
/// #[server]
/// async fn blog_posts(category: String) -> Result<Vec<BlogPost>> {
///     let posts = load_posts(&category).await?;
///     // maybe do some other work
///     Ok(posts)
/// }
/// ```
///
/// ## Named Arguments
///
/// You can use any combination of the following named arguments:
/// - `endpoint`: a prefix at which the server function handler will be mounted (defaults to `/api`).
///   Example: `endpoint = "/my_api/my_serverfn"`.
/// - `input`: the encoding for the arguments, defaults to `Json<T>`
///     - You may customize the encoding of the arguments by specifying a different type for `input`.
///     - Any axum `IntoRequest` extractor can be used here, and dioxus provides
///       - `Json<T>`: The default axum `Json` extractor that decodes JSON-encoded request bodies.
///       - `Cbor<T>`: A custom axum `Cbor` extractor that decodes CBOR-encoded request bodies.
///       - `MessagePack<T>`: A custom axum `MessagePack` extractor that decodes MessagePack-encoded request bodies.
/// - `output`: the encoding for the response (defaults to `Json`).
///     - The `output` argument specifies how the server should encode the response data.
///     - Acceptable values include:
///       - `Json`: A response encoded as JSON (default). This is ideal for most web applications.
///       - `Cbor`: A response encoded in the CBOR format for efficient, binary-encoded data.
/// - `client`: a custom `Client` implementation that will be used for this server function. This allows
///   customization of the client-side behavior if needed.
///
/// ## Advanced Usage of `input` and `output` Fields
///
/// The `input` and `output` fields allow you to customize how arguments and responses are encoded and decoded.
/// These fields impose specific trait bounds on the types you use. Here are detailed examples for different scenarios:
///
/// ## Adding layers to server functions
///
/// Layers allow you to transform the request and response of a server function. You can use layers
/// to add authentication, logging, or other functionality to your server functions. Server functions integrate
/// with the tower ecosystem, so you can use any layer that is compatible with tower.
///
/// Common layers include:
/// - [`tower_http::trace::TraceLayer`](https://docs.rs/tower-http/latest/tower_http/trace/struct.TraceLayer.html) for tracing requests and responses
/// - [`tower_http::compression::CompressionLayer`](https://docs.rs/tower-http/latest/tower_http/compression/struct.CompressionLayer.html) for compressing large responses
/// - [`tower_http::cors::CorsLayer`](https://docs.rs/tower-http/latest/tower_http/cors/struct.CorsLayer.html) for adding CORS headers to responses
/// - [`tower_http::timeout::TimeoutLayer`](https://docs.rs/tower-http/latest/tower_http/timeout/struct.TimeoutLayer.html) for adding timeouts to requests
/// - [`tower_sessions::service::SessionManagerLayer`](https://docs.rs/tower-sessions/0.13.0/tower_sessions/service/struct.SessionManagerLayer.html) for adding session management to requests
///
/// You can add a tower [`Layer`](https://docs.rs/tower/latest/tower/trait.Layer.html) to your server function with the middleware attribute:
///
/// ```rust,ignore
/// # use dioxus::prelude::*;
/// #[server]
/// // The TraceLayer will log all requests to the console
/// #[middleware(tower_http::timeout::TimeoutLayer::new(std::time::Duration::from_secs(5)))]
/// pub async fn my_wacky_server_fn(input: Vec<String>) -> ServerFnResult<usize> {
///     unimplemented!()
/// }
/// ```
#[proc_macro_attribute]
pub fn server(_attr: proc_macro::TokenStream, mut item: TokenStream) -> TokenStream {
    // Parse the attribute list using the old server_fn arg parser.
    let args = match syn::parse::<ServerFnArgs>(_attr) {
        Ok(args) => args,
        Err(err) => {
            let err: TokenStream = err.to_compile_error().into();
            item.extend(err);
            return item;
        }
    };

    let method = Method::Post(Ident::new("POST", proc_macro2::Span::call_site()));
    let route: Route = Route {
        method: None,
        path_params: vec![],
        query_params: vec![],
        state: None,
        route_lit: args.fn_path,
        oapi_options: None,
        server_args: Default::default(),
        prefix: Some(
            args.prefix
                .unwrap_or_else(|| LitStr::new("/api", Span::call_site())),
        ),
        _input_encoding: args.input,
        _output_encoding: args.output,
    };

    match route_impl_with_route(route, item.clone(), Some(method)) {
        Ok(mut tokens) => {
            // Let's add some deprecated warnings to the various fields from `args` if the user is using them...
            // We don't generate structs anymore, don't use various protocols, etc
            if let Some(name) = args.struct_name {
                tokens.extend(quote! {
                    const _: () = {
                        #[deprecated(note = "Dioxus server functions no longer generate a struct for the server function. The function itself is used directly.")]
                        struct #name;
                        fn ___assert_deprecated() {
                            let _ = #name;
                        }

                        ()
                    };
                });
            }

            //
            tokens.into()
        }

        // Retain the original function item and append the error to it. Better for autocomplete.
        Err(err) => {
            let err: TokenStream = err.to_compile_error().into();
            item.extend(err);
            item
        }
    }
}

#[proc_macro_attribute]
pub fn get(args: proc_macro::TokenStream, body: TokenStream) -> TokenStream {
    wrapped_route_impl(args, body, Some(Method::new_from_string("GET")))
}

#[proc_macro_attribute]
pub fn post(args: proc_macro::TokenStream, body: TokenStream) -> TokenStream {
    wrapped_route_impl(args, body, Some(Method::new_from_string("POST")))
}

#[proc_macro_attribute]
pub fn put(args: proc_macro::TokenStream, body: TokenStream) -> TokenStream {
    wrapped_route_impl(args, body, Some(Method::new_from_string("PUT")))
}

#[proc_macro_attribute]
pub fn delete(args: proc_macro::TokenStream, body: TokenStream) -> TokenStream {
    wrapped_route_impl(args, body, Some(Method::new_from_string("DELETE")))
}

#[proc_macro_attribute]
pub fn patch(args: proc_macro::TokenStream, body: TokenStream) -> TokenStream {
    wrapped_route_impl(args, body, Some(Method::new_from_string("PATCH")))
}

fn wrapped_route_impl(
    attr: TokenStream,
    mut item: TokenStream,
    method: Option<Method>,
) -> TokenStream {
    match route_impl(attr, item.clone(), method) {
        Ok(tokens) => tokens.into(),
        Err(err) => {
            let err: TokenStream = err.to_compile_error().into();
            item.extend(err);
            item
        }
    }
}

fn route_impl(
    attr: TokenStream,
    item: TokenStream,
    method_from_macro: Option<Method>,
) -> syn::Result<TokenStream2> {
    let route = syn::parse::<Route>(attr)?;
    route_impl_with_route(route, item, method_from_macro)
}

fn route_impl_with_route(
    route: Route,
    item: TokenStream,
    method_from_macro: Option<Method>,
) -> syn::Result<TokenStream2> {
    // Parse the route and function
    let function = syn::parse::<ItemFn>(item)?;

    let server_args = route.server_args.clone();
    let mut function_on_server = function.clone();
    function_on_server.sig.inputs.extend(server_args.clone());

    // Now we can compile the route
    let original_inputs = function
        .sig
        .inputs
        .iter()
        .map(|arg| match arg {
            FnArg::Receiver(_receiver) => panic!("Self type is not supported"),
            FnArg::Typed(pat_type) => {
                quote! {
                    #[allow(unused_mut)]
                    #pat_type
                }
            }
        })
        .collect::<Punctuated<_, Token![,]>>();

    let route = CompiledRoute::from_route(route, &function, false, method_from_macro)?;
    let path_extractor = route.path_extractor();
    let query_extractor = route.query_extractor();
    let query_params_struct = route.query_params_struct(false);
    let _state_type = &route.state;
    let method_ident = &route.method;
    let http_method = route.method.to_axum_method_name();
    let _remaining_numbered_pats = route.remaining_pattypes_numbered(&function.sig.inputs);
    let body_json_args = route.remaining_pattypes_named(&function.sig.inputs);
    let body_json_names = body_json_args
        .iter()
        .enumerate()
        .map(|(i, pat_type)| match &*pat_type.pat {
            Pat::Ident(ref pat_ident) => pat_ident.ident.clone(),
            _ => format_ident!("___arg{}", i),
        })
        .collect::<Vec<_>>();
    let body_json_types = body_json_args
        .iter()
        .map(|pat_type| &pat_type.ty)
        .collect::<Vec<_>>();
    let extracted_idents = route.extracted_idents();
    let route_docs = route.to_doc_comments();

    // Get the variables we need for code generation
    let fn_name = &function.sig.ident;
    let vis = &function.vis;
    let asyncness = &function.sig.asyncness;
    let (impl_generics, ty_generics, where_clause) = &function.sig.generics.split_for_impl();
    let ty_generics = ty_generics.as_turbofish();
    let fn_docs = function
        .attrs
        .iter()
        .filter(|attr| attr.path().is_ident("doc"));

    let __axum = quote! { dioxus_fullstack::axum };

    let (aide_ident_docs, _inner_fn_call, _method_router_ty) = {
        (
            quote!(),
            quote! { #__axum::routing::#http_method(__inner__function__ #ty_generics) },
            quote! { #__axum::routing::MethodRouter },
        )
    };

    let output_type = match &function.sig.output {
        syn::ReturnType::Default => parse_quote! { () },
        syn::ReturnType::Type(_, ty) => (*ty).clone(),
    };

    let query_param_names = route.query_params.iter().map(|(ident, _)| ident);

    let path_param_args = route.path_params.iter().map(|(_slash, param)| match param {
        PathParam::Capture(_lit, _brace_1, ident, _ty, _brace_2) => {
            Some(quote! { #ident = #ident, })
        }
        PathParam::WildCard(_lit, _brace_1, _star, ident, _ty, _brace_2) => {
            Some(quote! { #ident = #ident, })
        }
        PathParam::Static(_lit) => None,
    });

    let out_ty = match output_type.as_ref() {
        Type::Tuple(tuple) if tuple.elems.is_empty() => parse_quote! { () },
        _ => output_type.clone(),
    };

    let server_names = server_args
        .iter()
        .map(|pat_type| match pat_type {
            FnArg::Receiver(_) => quote! { () },
            FnArg::Typed(pat_type) => match pat_type.pat.as_ref() {
                Pat::Ident(pat_ident) => {
                    let name = &pat_ident.ident;
                    quote! { #name }
                }
                _ => panic!("Expected Pat::Ident"),
            },
        })
        .collect::<Vec<_>>();

    let server_types = server_args
        .iter()
        .map(|pat_type| match pat_type {
            FnArg::Receiver(_) => parse_quote! { () },
            FnArg::Typed(pat_type) => (*pat_type.ty).clone(),
        })
        .collect::<Vec<_>>();

    let body_struct_impl = {
        let tys = body_json_types
            .iter()
            .enumerate()
            .map(|(idx, _)| format_ident!("__Ty{}", idx));

        let names = body_json_names.iter().enumerate().map(|(idx, name)| {
            let ty_name = format_ident!("__Ty{}", idx);
            quote! { #name: #ty_name }
        });

        quote! {
            #[derive(serde::Serialize, serde::Deserialize)]
            #[serde(crate = "serde")]
            struct ___Body_Serialize___< #(#tys,)* > {
                #(#names,)*
            }
        }
    };

    // This unpacks the body struct into the individual variables that get scoped
    let unpack = {
        let unpack_args = body_json_names.iter().map(|name| quote! { data.#name });
        quote! {
            |data| { ( #(#unpack_args,)* ) }
        }
    };

    // there's no active request on the server, so we just create a dummy one
    let server_defaults = if server_args.is_empty() {
        quote! {}
    } else {
        quote! {
            let (#(#server_names,)*) = dioxus_fullstack::StreamingContext::extract::<(#(#server_types,)*), _>().await?;
        }
    };

    let as_axum_path = route.to_axum_path_string();

    let query_endpoint = if let Some(route_lit) = route.route_lit.as_ref() {
        let prefix = route
            .prefix
            .as_ref()
            .cloned()
            .unwrap_or_else(|| LitStr::new("", Span::call_site()))
            .value();
        let url_without_queries = route_lit.value().split('?').next().unwrap().to_string();
        let full_url = format!(
            "{}{}{}",
            prefix,
            if url_without_queries.starts_with("/") {
                ""
            } else {
                "/"
            },
            url_without_queries
        );
        quote! { format!(#full_url, #( #path_param_args)*) }
    } else {
        quote! { __ENDPOINT_PATH.to_string() }
    };

    let endpoint_path = {
        let prefix = route
            .prefix
            .as_ref()
            .cloned()
            .unwrap_or_else(|| LitStr::new("", Span::call_site()));

        let route_lit = if !as_axum_path.is_empty() {
            quote! { #as_axum_path }
        } else {
            quote! {
                concat!(
                    "/",
                    stringify!(#fn_name)
                )
            }
        };

        let hash = match route.route_lit.as_ref() {
            // Explicit route lit, no need to hash
            Some(_) => quote! { "" },

            // Implicit route lit, we need to hash the function signature to avoid collisions
            None => {
                // let enable_hash = option_env!("DISABLE_SERVER_FN_HASH").is_none();
                let key_env_var = match option_env!("SERVER_FN_OVERRIDE_KEY") {
                    Some(_) => "SERVER_FN_OVERRIDE_KEY",
                    None => "CARGO_MANIFEST_DIR",
                };
                quote! {
                    dioxus_fullstack::xxhash_rust::const_xxh64::xxh64(
                        concat!(env!(#key_env_var), ":", module_path!()).as_bytes(),
                        0
                    )
                }
            }
        };

        quote! {
            dioxus_fullstack::const_format::concatcp!(#prefix, #route_lit, #hash)
        }
    };

    Ok(quote! {
        #(#fn_docs)*
        #route_docs
        #vis async fn #fn_name #impl_generics(
            #original_inputs
        ) -> #out_ty #where_clause {
            use dioxus_fullstack::serde as serde;
            use dioxus_fullstack::{
                // concrete types
                ServerFnEncoder, ServerFnDecoder, DioxusServerState,

                // "magic" traits for encoding/decoding on the client
                ExtractRequest, EncodeRequest, RequestDecodeResult, RequestDecodeErr,

                // "magic" traits for encoding/decoding on the server
                MakeAxumResponse, MakeAxumError,
            };

            _ = dioxus_fullstack::assert_is_result::<#out_ty>();

            #query_params_struct

            #body_struct_impl

            const __ENDPOINT_PATH: &str = #endpoint_path;

            // On the client, we make the request to the server
            // We want to support extremely flexible error types and return types, making this more complex than it should
            #[allow(clippy::unused_unit)]
            #[cfg(not(feature = "server"))]
            {
                let client = dioxus_fullstack::ClientRequest::new(
                    dioxus_fullstack::http::Method::#method_ident,
                    #query_endpoint,
                    &__QueryParams__ { #(#query_param_names,)* },
                );

                let verify_token = (&&&&&&&&&&&&&&ServerFnEncoder::<___Body_Serialize___<#(#body_json_types,)*>, (#(#body_json_types,)*)>::new())
                    .verify_can_serialize();

                dioxus_fullstack::assert_can_encode(verify_token);

                let response = (&&&&&&&&&&&&&&ServerFnEncoder::<___Body_Serialize___<#(#body_json_types,)*>, (#(#body_json_types,)*)>::new())
                    .fetch_client(client, ___Body_Serialize___ { #(#body_json_names,)* }, #unpack)
                    .await;

                let decoded = (&&&&&ServerFnDecoder::<#out_ty>::new())
                    .decode_client_response(response)
                    .await;

                let result = (&&&&&ServerFnDecoder::<#out_ty>::new())
                    .decode_client_err(decoded)
                    .await;

                return result;
            }

            // On the server, we expand the tokens and submit the function to inventory
            #[cfg(feature = "server")] {
                use #__axum::response::IntoResponse;
                use dioxus_server::ServerFunction;

                #function_on_server

                #[allow(clippy::unused_unit)]
                #aide_ident_docs
                #asyncness fn __inner__function__ #impl_generics(
                    ___state: #__axum::extract::State<DioxusServerState>,
                    #path_extractor
                    #query_extractor
                    request: #__axum::extract::Request,
                ) -> Result<#__axum::response::Response, #__axum::response::Response> #where_clause {
                    let ((#(#server_names,)*), (  #(#body_json_names,)* )) = (&&&&&&&&&&&&&&ServerFnEncoder::<___Body_Serialize___<#(#body_json_types,)*>, (#(#body_json_types,)*)>::new())
                        .extract_axum(___state.0, request, #unpack).await?;

                    let encoded = (&&&&&&ServerFnDecoder::<#out_ty>::new())
                        .make_axum_response(
                            #fn_name #ty_generics(#(#extracted_idents,)*  #(#body_json_names,)* #(#server_names,)*).await
                        );

                    let response = (&&&&&ServerFnDecoder::<#out_ty>::new())
                        .make_axum_error(encoded);

                    return response;
                }

                dioxus_fullstack::inventory::submit! {
                    ServerFunction::new(
                        dioxus_fullstack::http::Method::#method_ident,
                        __ENDPOINT_PATH,
                        || #__axum::routing::#http_method(__inner__function__ #ty_generics)
                    )
                }

                #server_defaults

                return #fn_name #ty_generics(
                    #(#extracted_idents,)*
                    #(#body_json_names,)*
                    #(#server_names,)*
                ).await;
            }

            #[allow(unreachable_code)]
            {
                unreachable!()
            }
        }
    })
}

struct CompiledRoute {
    method: Method,
    #[allow(clippy::type_complexity)]
    path_params: Vec<(Slash, PathParam)>,
    query_params: Vec<(Ident, Box<Type>)>,
    state: Type,
    route_lit: Option<LitStr>,
    prefix: Option<LitStr>,
    oapi_options: Option<OapiOptions>,
}

impl CompiledRoute {
    fn to_axum_path_string(&self) -> String {
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
            prefix: route.prefix,
        })
    }

    pub fn path_extractor(&self) -> TokenStream2 {
        let path_iter = self
            .path_params
            .iter()
            .filter_map(|(_slash, path_param)| path_param.capture());
        let idents = path_iter.clone().map(|item| item.0);
        let types = path_iter.clone().map(|item| item.1);
        quote! {
            dioxus_fullstack::axum::extract::Path((#(#idents,)*)): dioxus_fullstack::axum::extract::Path<(#(#types,)*)>,
        }
    }

    pub fn query_extractor(&self) -> TokenStream2 {
        let idents = self.query_params.iter().map(|item| &item.0);
        quote! {
            dioxus_fullstack::axum::extract::Query(__QueryParams__ { #(#idents,)* }): dioxus_fullstack::axum::extract::Query<__QueryParams__>,
        }
    }

    pub fn query_params_struct(&self, with_aide: bool) -> TokenStream2 {
        let idents = self.query_params.iter().map(|item| &item.0);
        let types = self.query_params.iter().map(|item| &item.1);
        let derive = match with_aide {
            true => quote! {
                #[derive(serde::Deserialize, serde::Serialize, ::schemars::JsonSchema)]
                #[serde(crate = "serde")]
            },
            false => quote! {
                #[derive(serde::Deserialize, serde::Serialize)]
                #[serde(crate = "serde")]
            },
        };
        quote! {
            #derive
            struct __QueryParams__ {
                #(#idents: #types,)*
            }
        }
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
            .filter_map(|item| {
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

    #[allow(dead_code)]
    fn aide() {
        // let http_method = format_ident!("{}_with", http_method);
        // let summary = route
        //     .get_oapi_summary()
        //     .map(|summary| quote! { .summary(#summary) });
        // let description = route
        //     .get_oapi_description()
        //     .map(|description| quote! { .description(#description) });
        // let hidden = route
        //     .get_oapi_hidden()
        //     .map(|hidden| quote! { .hidden(#hidden) });
        // let tags = route.get_oapi_tags();
        // let id = route
        //     .get_oapi_id(&function.sig)
        //     .map(|id| quote! { .id(#id) });
        // let transform = route.get_oapi_transform()?;
        // let responses = route.get_oapi_responses();
        // let response_code = responses.iter().map(|response| &response.0);
        // let response_type = responses.iter().map(|response| &response.1);
        // let security = route.get_oapi_security();
        // let schemes = security.iter().map(|sec| &sec.0);
        // let scopes = security.iter().map(|sec| &sec.1);

        // (
        //     route.ide_documentation_for_aide_methods(),
        //     quote! {
        //         ::aide::axum::routing::#http_method(
        //             __inner__function__ #ty_generics,
        //             |__op__| {
        //                 let __op__ = __op__
        //                     #summary
        //                     #description
        //                     #hidden
        //                     #id
        //                     #(.tag(#tags))*
        //                     #(.security_requirement_scopes::<Vec<&'static str>, _>(#schemes, vec![#(#scopes),*]))*
        //                     #(.response::<#response_code, #response_type>())*
        //                     ;
        //                 #transform
        //                 __op__
        //             }
        //         )
        //     },
        //     quote! { ::aide::axum::routing::ApiMethodRouter },
        // )
    }

    #[allow(dead_code)]
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

    #[allow(dead_code)]
    pub fn get_oapi_summary(&self) -> Option<LitStr> {
        if let Some(oapi_options) = &self.oapi_options {
            if let Some(summary) = &oapi_options.summary {
                return Some(summary.1.clone());
            }
        }
        None
    }

    #[allow(dead_code)]
    pub fn get_oapi_description(&self) -> Option<LitStr> {
        if let Some(oapi_options) = &self.oapi_options {
            if let Some(description) = &oapi_options.description {
                return Some(description.1.clone());
            }
        }
        None
    }

    #[allow(dead_code)]
    pub fn get_oapi_hidden(&self) -> Option<LitBool> {
        if let Some(oapi_options) = &self.oapi_options {
            if let Some(hidden) = &oapi_options.hidden {
                return Some(hidden.1.clone());
            }
        }
        None
    }

    #[allow(dead_code)]
    pub fn get_oapi_tags(&self) -> Vec<LitStr> {
        if let Some(oapi_options) = &self.oapi_options {
            if let Some(tags) = &oapi_options.tags {
                return tags.1 .0.clone();
            }
        }
        Vec::new()
    }

    #[allow(dead_code)]
    pub fn get_oapi_id(&self, sig: &Signature) -> Option<LitStr> {
        if let Some(oapi_options) = &self.oapi_options {
            if let Some(id) = &oapi_options.id {
                return Some(id.1.clone());
            }
        }
        Some(LitStr::new(&sig.ident.to_string(), sig.ident.span()))
    }

    #[allow(dead_code)]
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

    #[allow(dead_code)]
    pub fn get_oapi_responses(&self) -> Vec<(LitInt, Type)> {
        if let Some(oapi_options) = &self.oapi_options {
            if let Some((_ident, Responses(responses))) = &oapi_options.responses {
                return responses.clone();
            }
        }
        Default::default()
    }

    #[allow(dead_code)]
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
            self.route_lit
                .as_ref()
                .map(|lit| lit.value())
                .unwrap_or_else(|| "<auto>".into()),
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

enum PathParam {
    WildCard(LitStr, Brace, Star, Ident, Box<Type>, Brace),
    Capture(LitStr, Brace, Ident, Box<Type>, Brace),
    Static(LitStr),
}

impl PathParam {
    fn _captures(&self) -> bool {
        matches!(self, Self::Capture(..) | Self::WildCard(..))
    }

    fn capture(&self) -> Option<(&Ident, &Type)> {
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
        } else if str.starts_with(':') && str.len() > 1 {
            let str = str.strip_prefix(':').unwrap();
            Self::Capture(
                LitStr::new(str, span),
                Brace(span),
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

struct OapiOptions {
    summary: Option<(Ident, LitStr)>,
    description: Option<(Ident, LitStr)>,
    id: Option<(Ident, LitStr)>,
    hidden: Option<(Ident, LitBool)>,
    tags: Option<(Ident, StrArray)>,
    security: Option<(Ident, Security)>,
    responses: Option<(Ident, Responses)>,
    transform: Option<(Ident, ExprClosure)>,
}

struct Security(Vec<(LitStr, StrArray)>);
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

impl std::fmt::Display for Security {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{")?;
        for (i, (scheme, scopes)) in self.0.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}: {}", scheme.value(), scopes)?;
        }
        write!(f, "}}")
    }
}

struct Responses(Vec<(LitInt, Type)>);
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

impl std::fmt::Display for Responses {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{")?;
        for (i, (status, ty)) in self.0.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}: {}", status, ty.to_token_stream())?;
        }
        write!(f, "}}")
    }
}

#[derive(Clone)]
struct StrArray(Vec<LitStr>);
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

impl std::fmt::Display for StrArray {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[")?;
        for (i, lit) in self.0.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "\"{}\"", lit.value())?;
        }
        write!(f, "]")
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
    fn merge_with_fn(&mut self, function: &ItemFn) {
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

struct Route {
    method: Option<Method>,
    path_params: Vec<(Slash, PathParam)>,
    query_params: Vec<Ident>,
    state: Option<Type>,
    route_lit: Option<LitStr>,
    prefix: Option<LitStr>,
    oapi_options: Option<OapiOptions>,
    server_args: Punctuated<FnArg, Comma>,

    // todo: support these since `server_fn` had them
    _input_encoding: Option<Type>,
    _output_encoding: Option<Type>,
}

impl Parse for Route {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let method = if input.peek(Ident) {
            Some(input.parse::<Method>()?)
        } else {
            None
        };

        let route_lit = input.parse::<LitStr>()?;
        let RouteParser {
            path_params,
            query_params,
        } = RouteParser::new(route_lit.clone())?;

        // todo: maybe let the user include `State<T>` here, eventually?
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
            path_params,
            query_params,
            state,
            route_lit: Some(route_lit),
            oapi_options,
            server_args,
            prefix: None,
            _input_encoding: None,
            _output_encoding: None,
        })
    }
}

#[derive(Clone)]
enum Method {
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
    fn to_axum_method_name(&self) -> Ident {
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

    fn new_from_string(s: &str) -> Self {
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

/// The arguments to the `server` macro.
///
/// These originally came from the `server_fn` crate, but many no longer apply after the 0.7 fullstack
/// overhaul. We keep the parser here for temporary backwards compatibility with existing code, but
/// these arguments will be removed in a future release.
#[derive(Debug)]
#[non_exhaustive]
#[allow(unused)]
struct ServerFnArgs {
    /// The name of the struct that will implement the server function trait
    /// and be submitted to inventory.
    struct_name: Option<Ident>,
    /// The prefix to use for the server function URL.
    prefix: Option<LitStr>,
    /// The input http encoding to use for the server function.
    input: Option<Type>,
    /// Additional traits to derive on the input struct for the server function.
    input_derive: Option<ExprTuple>,
    /// The output http encoding to use for the server function.
    output: Option<Type>,
    /// The path to the server function crate.
    fn_path: Option<LitStr>,
    /// The server type to use for the server function.
    server: Option<Type>,
    /// The client type to use for the server function.
    client: Option<Type>,
    /// The custom wrapper to use for the server function struct.
    custom_wrapper: Option<syn::Path>,
    /// If the generated input type should implement `From` the only field in the input
    impl_from: Option<LitBool>,
    /// If the generated input type should implement `Deref` to the only field in the input
    impl_deref: Option<LitBool>,
    /// The protocol to use for the server function implementation.
    protocol: Option<Type>,
    builtin_encoding: bool,
}

impl Parse for ServerFnArgs {
    fn parse(stream: ParseStream) -> syn::Result<Self> {
        // legacy 4-part arguments
        let mut struct_name: Option<Ident> = None;
        let mut prefix: Option<LitStr> = None;
        let mut encoding: Option<LitStr> = None;
        let mut fn_path: Option<LitStr> = None;

        // new arguments: can only be keyed by name
        let mut input: Option<Type> = None;
        let mut input_derive: Option<ExprTuple> = None;
        let mut output: Option<Type> = None;
        let mut server: Option<Type> = None;
        let mut client: Option<Type> = None;
        let mut custom_wrapper: Option<syn::Path> = None;
        let mut impl_from: Option<LitBool> = None;
        let mut impl_deref: Option<LitBool> = None;
        let mut protocol: Option<Type> = None;

        let mut use_key_and_value = false;
        let mut arg_pos = 0;

        while !stream.is_empty() {
            arg_pos += 1;
            let lookahead = stream.lookahead1();
            if lookahead.peek(Ident) {
                let key_or_value: Ident = stream.parse()?;

                let lookahead = stream.lookahead1();
                if lookahead.peek(Token![=]) {
                    stream.parse::<Token![=]>()?;
                    let key = key_or_value;
                    use_key_and_value = true;
                    if key == "name" {
                        if struct_name.is_some() {
                            return Err(syn::Error::new(
                                key.span(),
                                "keyword argument repeated: `name`",
                            ));
                        }
                        struct_name = Some(stream.parse()?);
                    } else if key == "prefix" {
                        if prefix.is_some() {
                            return Err(syn::Error::new(
                                key.span(),
                                "keyword argument repeated: `prefix`",
                            ));
                        }
                        prefix = Some(stream.parse()?);
                    } else if key == "encoding" {
                        if encoding.is_some() {
                            return Err(syn::Error::new(
                                key.span(),
                                "keyword argument repeated: `encoding`",
                            ));
                        }
                        encoding = Some(stream.parse()?);
                    } else if key == "endpoint" {
                        if fn_path.is_some() {
                            return Err(syn::Error::new(
                                key.span(),
                                "keyword argument repeated: `endpoint`",
                            ));
                        }
                        fn_path = Some(stream.parse()?);
                    } else if key == "input" {
                        if encoding.is_some() {
                            return Err(syn::Error::new(
                                key.span(),
                                "`encoding` and `input` should not both be \
                                 specified",
                            ));
                        } else if input.is_some() {
                            return Err(syn::Error::new(
                                key.span(),
                                "keyword argument repeated: `input`",
                            ));
                        }
                        input = Some(stream.parse()?);
                    } else if key == "input_derive" {
                        if input_derive.is_some() {
                            return Err(syn::Error::new(
                                key.span(),
                                "keyword argument repeated: `input_derive`",
                            ));
                        }
                        input_derive = Some(stream.parse()?);
                    } else if key == "output" {
                        if encoding.is_some() {
                            return Err(syn::Error::new(
                                key.span(),
                                "`encoding` and `output` should not both be \
                                 specified",
                            ));
                        } else if output.is_some() {
                            return Err(syn::Error::new(
                                key.span(),
                                "keyword argument repeated: `output`",
                            ));
                        }
                        output = Some(stream.parse()?);
                    } else if key == "server" {
                        if server.is_some() {
                            return Err(syn::Error::new(
                                key.span(),
                                "keyword argument repeated: `server`",
                            ));
                        }
                        server = Some(stream.parse()?);
                    } else if key == "client" {
                        if client.is_some() {
                            return Err(syn::Error::new(
                                key.span(),
                                "keyword argument repeated: `client`",
                            ));
                        }
                        client = Some(stream.parse()?);
                    } else if key == "custom" {
                        if custom_wrapper.is_some() {
                            return Err(syn::Error::new(
                                key.span(),
                                "keyword argument repeated: `custom`",
                            ));
                        }
                        custom_wrapper = Some(stream.parse()?);
                    } else if key == "impl_from" {
                        if impl_from.is_some() {
                            return Err(syn::Error::new(
                                key.span(),
                                "keyword argument repeated: `impl_from`",
                            ));
                        }
                        impl_from = Some(stream.parse()?);
                    } else if key == "impl_deref" {
                        if impl_deref.is_some() {
                            return Err(syn::Error::new(
                                key.span(),
                                "keyword argument repeated: `impl_deref`",
                            ));
                        }
                        impl_deref = Some(stream.parse()?);
                    } else if key == "protocol" {
                        if protocol.is_some() {
                            return Err(syn::Error::new(
                                key.span(),
                                "keyword argument repeated: `protocol`",
                            ));
                        }
                        protocol = Some(stream.parse()?);
                    } else {
                        return Err(lookahead.error());
                    }
                } else {
                    let value = key_or_value;
                    if use_key_and_value {
                        return Err(syn::Error::new(
                            value.span(),
                            "positional argument follows keyword argument",
                        ));
                    }
                    if arg_pos == 1 {
                        struct_name = Some(value)
                    } else {
                        return Err(syn::Error::new(value.span(), "expected string literal"));
                    }
                }
            } else if lookahead.peek(LitStr) {
                if use_key_and_value {
                    return Err(syn::Error::new(
                        stream.span(),
                        "If you use keyword arguments (e.g., `name` = \
                         Something), then you can no longer use arguments \
                         without a keyword.",
                    ));
                }
                match arg_pos {
                    1 => return Err(lookahead.error()),
                    2 => prefix = Some(stream.parse()?),
                    3 => encoding = Some(stream.parse()?),
                    4 => fn_path = Some(stream.parse()?),
                    _ => return Err(syn::Error::new(stream.span(), "unexpected extra argument")),
                }
            } else {
                return Err(lookahead.error());
            }

            if !stream.is_empty() {
                stream.parse::<Token![,]>()?;
            }
        }

        // parse legacy encoding into input/output
        let mut builtin_encoding = false;
        if let Some(encoding) = encoding {
            match encoding.value().to_lowercase().as_str() {
                "url" => {
                    input = Some(type_from_ident(syn::parse_quote!(Url)));
                    output = Some(type_from_ident(syn::parse_quote!(Json)));
                    builtin_encoding = true;
                }
                "cbor" => {
                    input = Some(type_from_ident(syn::parse_quote!(Cbor)));
                    output = Some(type_from_ident(syn::parse_quote!(Cbor)));
                    builtin_encoding = true;
                }
                "getcbor" => {
                    input = Some(type_from_ident(syn::parse_quote!(GetUrl)));
                    output = Some(type_from_ident(syn::parse_quote!(Cbor)));
                    builtin_encoding = true;
                }
                "getjson" => {
                    input = Some(type_from_ident(syn::parse_quote!(GetUrl)));
                    output = Some(syn::parse_quote!(Json));
                    builtin_encoding = true;
                }
                _ => return Err(syn::Error::new(encoding.span(), "Encoding not found.")),
            }
        }

        Ok(Self {
            struct_name,
            prefix,
            input,
            input_derive,
            output,
            fn_path,
            builtin_encoding,
            server,
            client,
            custom_wrapper,
            impl_from,
            impl_deref,
            protocol,
        })
    }
}

/// An argument type in a server function.
#[allow(unused)]
// todo - we used to support a number of these attributes and pass them along to serde. bring them back.
#[derive(Debug, Clone)]
struct ServerFnArg {
    /// The attributes on the server function argument.
    server_fn_attributes: Vec<Attribute>,
    /// The type of the server function argument.
    arg: syn::PatType,
}

impl ToTokens for ServerFnArg {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let ServerFnArg { arg, .. } = self;
        tokens.extend(quote! {
            #arg
        });
    }
}

impl Parse for ServerFnArg {
    fn parse(input: ParseStream) -> Result<Self> {
        let arg: syn::FnArg = input.parse()?;
        let mut arg = match arg {
            FnArg::Receiver(_) => {
                return Err(syn::Error::new(
                    arg.span(),
                    "cannot use receiver types in server function macro",
                ))
            }
            FnArg::Typed(t) => t,
        };

        fn rename_path(path: Path, from_ident: Ident, to_ident: Ident) -> Path {
            if path.is_ident(&from_ident) {
                Path {
                    leading_colon: None,
                    segments: Punctuated::from_iter([PathSegment {
                        ident: to_ident,
                        arguments: PathArguments::None,
                    }]),
                }
            } else {
                path
            }
        }

        let server_fn_attributes = arg
            .attrs
            .iter()
            .cloned()
            .map(|attr| {
                if attr.path().is_ident("server") {
                    // Allow the following attributes:
                    // - #[server(default)]
                    // - #[server(rename = "fieldName")]

                    // Rename `server` to `serde`
                    let attr = Attribute {
                        meta: match attr.meta {
                            Meta::Path(path) => Meta::Path(rename_path(
                                path,
                                format_ident!("server"),
                                format_ident!("serde"),
                            )),
                            Meta::List(mut list) => {
                                list.path = rename_path(
                                    list.path,
                                    format_ident!("server"),
                                    format_ident!("serde"),
                                );
                                Meta::List(list)
                            }
                            Meta::NameValue(mut name_value) => {
                                name_value.path = rename_path(
                                    name_value.path,
                                    format_ident!("server"),
                                    format_ident!("serde"),
                                );
                                Meta::NameValue(name_value)
                            }
                        },
                        ..attr
                    };

                    let args = attr.parse_args::<Meta>()?;
                    match args {
                        // #[server(default)]
                        Meta::Path(path) if path.is_ident("default") => Ok(attr.clone()),
                        // #[server(flatten)]
                        Meta::Path(path) if path.is_ident("flatten") => Ok(attr.clone()),
                        // #[server(default = "value")]
                        Meta::NameValue(name_value) if name_value.path.is_ident("default") => {
                            Ok(attr.clone())
                        }
                        // #[server(skip)]
                        Meta::Path(path) if path.is_ident("skip") => Ok(attr.clone()),
                        // #[server(rename = "value")]
                        Meta::NameValue(name_value) if name_value.path.is_ident("rename") => {
                            Ok(attr.clone())
                        }
                        _ => Err(Error::new(
                            attr.span(),
                            "Unrecognized #[server] attribute, expected \
                             #[server(default)] or #[server(rename = \
                             \"fieldName\")]",
                        )),
                    }
                } else if attr.path().is_ident("doc") {
                    // Allow #[doc = "documentation"]
                    Ok(attr.clone())
                } else if attr.path().is_ident("allow") {
                    // Allow #[allow(...)]
                    Ok(attr.clone())
                } else if attr.path().is_ident("deny") {
                    // Allow #[deny(...)]
                    Ok(attr.clone())
                } else if attr.path().is_ident("ignore") {
                    // Allow #[ignore]
                    Ok(attr.clone())
                } else {
                    Err(Error::new(
                        attr.span(),
                        "Unrecognized attribute, expected #[server(...)]",
                    ))
                }
            })
            .collect::<Result<Vec<_>>>()?;
        arg.attrs = vec![];
        Ok(ServerFnArg {
            arg,
            server_fn_attributes,
        })
    }
}

fn type_from_ident(ident: Ident) -> Type {
    let mut segments = Punctuated::new();
    segments.push(PathSegment {
        ident,
        arguments: PathArguments::None,
    });
    Type::Path(TypePath {
        qself: None,
        path: Path {
            leading_colon: None,
            segments,
        },
    })
}

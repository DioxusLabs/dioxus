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
    Error, ExprTuple, FnArg, Meta, PathArguments, PathSegment, Token, Type, TypePath,
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
pub fn server(attr: proc_macro::TokenStream, mut item: TokenStream) -> TokenStream {
    // Parse the attribute list using the old server_fn arg parser.
    let args = match syn::parse::<ServerFnArgs>(attr) {
        Ok(args) => args,
        Err(err) => {
            let err: TokenStream = err.to_compile_error().into();
            item.extend(err);
            return item;
        }
    };

    let method = Method::Post(Ident::new("POST", proc_macro2::Span::call_site()));
    let prefix = args
        .prefix
        .unwrap_or_else(|| LitStr::new("/api", Span::call_site()));

    let route: Route = Route {
        method: None,
        path_params: vec![],
        query_params: vec![],
        route_lit: args.fn_path,
        oapi_options: None,
        server_args: args.server_args,
        prefix: Some(prefix),
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
    let mut function = syn::parse::<ItemFn>(item)?;

    // Collect the middleware initializers
    let middleware_layers = function
        .attrs
        .iter()
        .filter(|attr| attr.path().is_ident("middleware"))
        .map(|f| match &f.meta {
            Meta::List(meta_list) => Ok({
                let tokens = &meta_list.tokens;
                quote! { .layer(#tokens) }
            }),
            _ => Err(Error::new(
                f.span(),
                "Expected middleware attribute to be a list, e.g. #[middleware(MyLayer::new())]",
            )),
        })
        .collect::<Result<Vec<_>>>()?;

    // don't re-emit the middleware attribute on the inner
    function
        .attrs
        .retain(|attr| !attr.path().is_ident("middleware"));

    // Attach `#[allow(unused_mut)]` to all original inputs to avoid warnings
    let outer_inputs = function
        .sig
        .inputs
        .iter()
        .enumerate()
        .map(|(i, arg)| match arg {
            FnArg::Receiver(_receiver) => panic!("Self type is not supported"),
            FnArg::Typed(pat_type) => match pat_type.pat.as_ref() {
                Pat::Ident(_) => {
                    quote! { #[allow(unused_mut)] #pat_type }
                }
                _ => {
                    let ident = format_ident!("___Arg{}", i);
                    let ty = &pat_type.ty;
                    quote! { #[allow(unused_mut)] #ident: #ty }
                }
            },
        })
        .collect::<Punctuated<_, Token![,]>>();
    // .collect::<Punctuated<_, Token![,]>>();

    let route = CompiledRoute::from_route(route, &function, false, method_from_macro)?;
    let query_params_struct = route.query_params_struct(false);
    let method_ident = &route.method;
    let body_json_args = route.remaining_pattypes_named(&function.sig.inputs);
    let body_json_names = body_json_args
        .iter()
        .map(|(i, pat_type)| match &*pat_type.pat {
            Pat::Ident(ref pat_ident) => pat_ident.ident.clone(),
            _ => format_ident!("___Arg{}", i),
        })
        .collect::<Vec<_>>();
    let body_json_types = body_json_args
        .iter()
        .map(|pat_type| &pat_type.1.ty)
        .collect::<Vec<_>>();
    let route_docs = route.to_doc_comments();

    // Get the variables we need for code generation
    let fn_on_server_name = &function.sig.ident;
    let vis = &function.vis;
    let (impl_generics, ty_generics, where_clause) = &function.sig.generics.split_for_impl();
    let ty_generics = ty_generics.as_turbofish();
    let fn_docs = function
        .attrs
        .iter()
        .filter(|attr| attr.path().is_ident("doc"));

    let __axum = quote! { dioxus_server::axum };

    let output_type = match &function.sig.output {
        syn::ReturnType::Default => parse_quote! { () },
        syn::ReturnType::Type(_, ty) => (*ty).clone(),
    };

    let query_param_names = route
        .query_params
        .iter()
        .filter(|c| !c.catch_all)
        .map(|param| &param.binding);

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

    let mut function_on_server = function.clone();
    function_on_server
        .sig
        .inputs
        .extend(route.server_args.clone());

    let server_names = route
        .server_args
        .iter()
        .enumerate()
        .map(|(i, pat_type)| match pat_type {
            FnArg::Typed(_pat_type) => format_ident!("___sarg___{}", i),
            FnArg::Receiver(_) => panic!("Self type is not supported"),
        })
        .collect::<Vec<_>>();

    let server_types = route
        .server_args
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
    let unpack_closure = {
        let unpack_args = body_json_names.iter().map(|name| quote! { data.#name });
        quote! {
            |data| { ( #(#unpack_args,)* ) }
        }
    };

    let as_axum_path = route.to_axum_path_string();

    let query_endpoint = if let Some(full_url) = route.url_without_queries_for_format() {
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

        let route_lit = if let Some(lit) = as_axum_path {
            quote! { #lit }
        } else {
            let name =
                route.route_lit.as_ref().cloned().unwrap_or_else(|| {
                    LitStr::new(&fn_on_server_name.to_string(), Span::call_site())
                });
            quote! {
                concat!(
                    "/",
                    #name
                )
            }
        };

        let hash = match route.prefix.as_ref() {
            // Implicit route lit, we need to hash the function signature to avoid collisions
            Some(_) if route.route_lit.is_none() => {
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

            // Explicit route lit, no need to hash
            _ => quote! { "" },
        };

        quote! {
            dioxus_fullstack::const_format::concatcp!(#prefix, #route_lit, #hash)
        }
    };

    let extracted_idents = route.extracted_idents();

    let query_tokens = if route.query_is_catchall() {
        let query = route
            .query_params
            .iter()
            .find(|param| param.catch_all)
            .unwrap();
        let input = &function.sig.inputs[query.arg_idx];
        let name = match input {
            FnArg::Typed(pat_type) => match pat_type.pat.as_ref() {
                Pat::Ident(ref pat_ident) => pat_ident.ident.clone(),
                _ => format_ident!("___Arg{}", query.arg_idx),
            },
            FnArg::Receiver(_receiver) => panic!(),
        };
        quote! {
            #name
        }
    } else {
        quote! {
            __QueryParams__ { #(#query_param_names,)* }
        }
    };

    let extracted_as_server_headers = route.extracted_as_server_headers(query_tokens.clone());

    Ok(quote! {
        #(#fn_docs)*
        #route_docs
        #[deny(
            unexpected_cfgs,
            reason = "
==========================================================================================
  Using Dioxus Server Functions requires a `server` feature flag in your `Cargo.toml`.
  Please add the following to your `Cargo.toml`:

  ```toml
  [features]
  server = [\"dioxus/server\"]
  ```

  To enable better Rust-Analyzer support, you can make `server` a default feature:
  ```toml
  [features]
  default = [\"web\", \"server\"]
  web = [\"dioxus/web\"]
  server = [\"dioxus/server\"]
  ```
==========================================================================================
        "
        )]
        #vis async fn #fn_on_server_name #impl_generics( #outer_inputs ) -> #out_ty #where_clause {
            use dioxus_fullstack::serde as serde;
            use dioxus_fullstack::{
                // concrete types
                ServerFnEncoder, ServerFnDecoder, FullstackContext,

                // "magic" traits for encoding/decoding on the client
                ExtractRequest, EncodeRequest, RequestDecodeResult, RequestDecodeErr,

                // "magic" traits for encoding/decoding on the server
                MakeAxumResponse, MakeAxumError,
            };

            #query_params_struct

            #body_struct_impl

            const __ENDPOINT_PATH: &str = #endpoint_path;

            {
                _ = dioxus_fullstack::assert_is_result::<#out_ty>();

                let verify_token = (&&&&&&&&&&&&&&ServerFnEncoder::<___Body_Serialize___<#(#body_json_types,)*>, (#(#body_json_types,)*)>::new())
                    .verify_can_serialize();

                dioxus_fullstack::assert_can_encode(verify_token);

                let decode_token = (&&&&&ServerFnDecoder::<#out_ty>::new())
                    .verify_can_deserialize();

                dioxus_fullstack::assert_can_decode(decode_token);
            };


            // On the client, we make the request to the server
            // We want to support extremely flexible error types and return types, making this more complex than it should
            #[allow(clippy::unused_unit)]
            #[cfg(not(feature = "server"))]
            {
                let client = dioxus_fullstack::ClientRequest::new(
                    dioxus_fullstack::http::Method::#method_ident,
                    #query_endpoint,
                    &#query_tokens,
                );

                let response = (&&&&&&&&&&&&&&ServerFnEncoder::<___Body_Serialize___<#(#body_json_types,)*>, (#(#body_json_types,)*)>::new())
                    .fetch_client(client, ___Body_Serialize___ { #(#body_json_names,)* }, #unpack_closure)
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
                #function_on_server

                #[allow(clippy::unused_unit)]
                fn __inner__function__ #impl_generics(
                    ___state: #__axum::extract::State<FullstackContext>,
                    ___request: #__axum::extract::Request,
                ) -> std::pin::Pin<Box<dyn std::future::Future<Output = #__axum::response::Response>>> #where_clause {
                    Box::pin(async move {
                         match (&&&&&&&&&&&&&&ServerFnEncoder::<___Body_Serialize___<#(#body_json_types,)*>, (#(#body_json_types,)*)>::new()).extract_axum(___state.0, ___request, #unpack_closure).await {
                            Ok(((#(#body_json_names,)* ), (#(#extracted_as_server_headers,)* #(#server_names,)*) )) => {
                                // Call the user function
                                let res = #fn_on_server_name #ty_generics(#(#extracted_idents,)* #(#body_json_names,)* #(#server_names,)*).await;

                                // Encode the response Into a `Result<T, E>`
                                let encoded = (&&&&&&ServerFnDecoder::<#out_ty>::new()).make_axum_response(res);

                                // And then encode `Result<T, E>` into `Response`
                                (&&&&&ServerFnDecoder::<#out_ty>::new()).make_axum_error(encoded)
                            },
                            Err(res) => res,
                        }
                    })
                }

                dioxus_server::inventory::submit! {
                    dioxus_server::ServerFunction::new(
                        dioxus_server::http::Method::#method_ident,
                        __ENDPOINT_PATH,
                        || {
                            dioxus_server::ServerFunction::make_handler(dioxus_server::http::Method::#method_ident, __inner__function__ #ty_generics)
                                #(#middleware_layers)*
                        }
                    )
                }

                // Extract the server arguments from the context if needed.
                let (#(#server_names,)*) = dioxus_fullstack::FullstackContext::extract::<(#(#server_types,)*), _>().await?;

                // Call the function directly
                return #fn_on_server_name #ty_generics(
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
    query_params: Vec<QueryParam>,
    route_lit: Option<LitStr>,
    prefix: Option<LitStr>,
    oapi_options: Option<OapiOptions>,
    server_args: Punctuated<FnArg, Comma>,
}

struct QueryParam {
    arg_idx: usize,
    name: String,
    binding: Ident,
    catch_all: bool,
    ty: Box<Type>,
}

impl CompiledRoute {
    fn to_axum_path_string(&self) -> Option<String> {
        if self.prefix.is_some() {
            return None;
        }

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
        }

        Some(path)
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
            .enumerate()
            .filter_map(|(i, item)| match item {
                syn::FnArg::Receiver(_) => None,
                syn::FnArg::Typed(pat_type) => Some((i, pat_type)),
            })
            .filter_map(|(i, pat_type)| match &*pat_type.pat {
                syn::Pat::Ident(ident) => Some((ident.ident.clone(), (pat_type.ty.clone(), i))),
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
                    *ty = new_ty.0;
                }
                PathParam::WildCard(_lit, _, _star, ident, ty, _) => {
                    let (new_ident, new_ty) = arg_map.remove_entry(ident).ok_or_else(|| {
                        syn::Error::new(
                            ident.span(),
                            format!("path parameter `{}` not found in function arguments", ident),
                        )
                    })?;
                    *ident = new_ident;
                    *ty = new_ty.0;
                }
                PathParam::Static(_lit) => {}
            }
        }

        let mut query_params = Vec::new();
        for param in route.query_params {
            let (ident, ty) = arg_map.remove_entry(&param.binding).ok_or_else(|| {
                syn::Error::new(
                    param.binding.span(),
                    format!(
                        "query parameter `{}` not found in function arguments",
                        param.binding
                    ),
                )
            })?;
            query_params.push(QueryParam {
                binding: ident,
                name: param.name,
                catch_all: param.catch_all,
                ty: ty.0,
                arg_idx: ty.1,
            });
        }

        // Disallow multiple query params if one is a catch-all
        if query_params.iter().any(|param| param.catch_all) && query_params.len() > 1 {
            return Err(syn::Error::new(
                Span::call_site(),
                "Cannot have multiple query parameters when one is a catch-all",
            ));
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
            oapi_options: route.oapi_options,
            prefix: route.prefix,
            server_args: route.server_args,
        })
    }

    pub fn query_is_catchall(&self) -> bool {
        self.query_params.iter().any(|param| param.catch_all)
    }

    pub fn extracted_as_server_headers(&self, query_tokens: TokenStream2) -> Vec<Pat> {
        let mut out = vec![];

        // Add the path extractor
        out.push({
            let path_iter = self
                .path_params
                .iter()
                .filter_map(|(_slash, path_param)| path_param.capture());
            let idents = path_iter.clone().map(|item| item.0);
            parse_quote! {
                dioxus_server::axum::extract::Path((#(#idents,)*))
            }
        });

        out.push(parse_quote!(
            dioxus_fullstack::payloads::Query(#query_tokens)
        ));

        out
    }

    pub fn query_params_struct(&self, with_aide: bool) -> TokenStream2 {
        let fields = self.query_params.iter().map(|item| {
            let name = &item.name;
            let binding = &item.binding;
            let ty = &item.ty;
            if item.catch_all {
                quote! {}
            } else if item.binding != item.name {
                quote! {
                    #[serde(rename = #name)]
                    #binding: #ty,
                }
            } else {
                quote! { #binding: #ty, }
            }
        });
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
                #(#fields)*
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
        for param in &self.query_params {
            idents.push(param.binding.clone());
        }
        idents
    }

    fn remaining_pattypes_named(&self, args: &Punctuated<FnArg, Comma>) -> Vec<(usize, PatType)> {
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
                            .any(|query| query.binding == pat_ident.ident)
                        {
                            return None;
                        }
                    }

                    Some((i, pat_type.clone()))
                } else {
                    unimplemented!("Self type is not supported")
                }
            })
            .collect()
    }

    pub(crate) fn to_doc_comments(&self) -> TokenStream2 {
        let mut doc = format!(
            "# Handler information
- Method: `{}`
- Path: `{}`",
            self.method.to_axum_method_name(),
            self.route_lit
                .as_ref()
                .map(|lit| lit.value())
                .unwrap_or_else(|| "<auto>".into()),
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

    fn url_without_queries_for_format(&self) -> Option<String> {
        // If there's a prefix, then it's an old-style route, and we can't generate a format string.
        if self.prefix.is_some() {
            return None;
        }

        // If there's no explicit route, we can't generate a format string this way.
        let _lit = self.route_lit.as_ref()?;

        let url_without_queries =
            self.path_params
                .iter()
                .fold(String::new(), |mut acc, (_slash, param)| {
                    acc.push('/');
                    match param {
                        PathParam::Capture(lit, _brace_1, _, _, _brace_2) => {
                            acc.push_str(&format!("{{{}}}", lit.value()));
                        }
                        PathParam::WildCard(lit, _brace_1, _, _, _, _brace_2) => {
                            // no `*` since we want to use the argument *as the wildcard* when making requests
                            // it's not super applicable to server functions, more for general route generation
                            acc.push_str(&format!("{{{}}}", lit.value()));
                        }
                        PathParam::Static(lit) => {
                            acc.push_str(&lit.value());
                        }
                    }
                    acc
                });

        let prefix = self
            .prefix
            .as_ref()
            .cloned()
            .unwrap_or_else(|| LitStr::new("", Span::call_site()))
            .value();
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

        Some(full_url)
    }
}

struct RouteParser {
    path_params: Vec<(Slash, PathParam)>,
    query_params: Vec<QueryParam>,
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
                if query_param.starts_with(":") {
                    let ident = Ident::new(query_param.strip_prefix(":").unwrap(), span);

                    query_params.push(QueryParam {
                        name: ident.to_string(),
                        binding: ident,
                        catch_all: true,
                        ty: parse_quote!(()),
                        arg_idx: usize::MAX,
                    });
                } else if query_param.starts_with("{") && query_param.ends_with("}") {
                    let ident = Ident::new(
                        query_param
                            .strip_prefix("{")
                            .unwrap()
                            .strip_suffix("}")
                            .unwrap(),
                        span,
                    );

                    query_params.push(QueryParam {
                        name: ident.to_string(),
                        binding: ident,
                        catch_all: true,
                        ty: parse_quote!(()),
                        arg_idx: usize::MAX,
                    });
                } else {
                    // if there's an `=` in the query param, we only take the left side as the name, and the right side is the binding
                    let name;
                    let binding;
                    if let Some((n, b)) = query_param.split_once('=') {
                        name = n;
                        binding = Ident::new(b, span);
                    } else {
                        name = query_param;
                        binding = Ident::new(query_param, span);
                    }

                    query_params.push(QueryParam {
                        name: name.to_string(),
                        binding,
                        catch_all: false,
                        ty: parse_quote!(()),
                        arg_idx: usize::MAX,
                    });
                }
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
    query_params: Vec<QueryParam>,
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
    Patch(Ident),
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
            | Self::Trace(ident)
            | Self::Patch(ident) => {
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
            Self::Patch(span) => Ident::new("patch", span.span()),
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
            "PATCH" => Self::Patch(Ident::new("PATCH", Span::call_site())),
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
    /// Server-only extractors (e.g., headers: HeaderMap, cookies: Cookies).
    /// These are arguments that exist purely on the server side.
    server_args: Punctuated<FnArg, Comma>,
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

        // Server-only extractors (key: Type pattern)
        // These come after config options (key = value pattern)
        // Example: #[server(endpoint = "/api/chat", headers: HeaderMap, cookies: Cookies)]
        let mut server_args: Punctuated<FnArg, Comma> = Punctuated::new();

        while !stream.is_empty() {
            // Check if this looks like an extractor (Ident : Type)
            // If so, break out to parse extractors - they must come last
            if stream.peek(Ident) && stream.peek2(Token![:]) {
                break;
            }

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

        // Now parse any remaining extractors (key: Type pattern)
        while !stream.is_empty() {
            if stream.peek(Ident) && stream.peek2(Token![:]) {
                server_args.push_value(stream.parse::<FnArg>()?);
                if stream.peek(Comma) {
                    server_args.push_punct(stream.parse::<Comma>()?);
                } else {
                    break;
                }
            } else {
                break;
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
            server_args,
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

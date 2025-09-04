// TODO: Create README, uncomment this: #![doc = include_str!("../README.md")]
#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]

//! This crate contains the dioxus implementation of the #[macro@crate::server] macro without additional context from the server.
//! See the [server_fn_macro] crate for more information.

use proc_macro::TokenStream;
use server_fn_macro_dioxus::ServerFnCall;
use syn::{__private::ToTokens, parse_quote};

/// Declares that a function is a [server function](https://docs.rs/server_fn/).
/// This means that its body will only run on the server, i.e., when the `ssr`
/// feature is enabled on this crate.
///
/// ## Usage
/// ```rust,ignore
/// # use dioxus::prelude::*;
/// # #[derive(serde::Deserialize, serde::Serialize)]
/// # pub struct BlogPost;
/// # async fn load_posts(category: &str) -> ServerFnResult<Vec<BlogPost>> { unimplemented!() }
/// #[server]
/// pub async fn blog_posts(
///     category: String,
/// ) -> ServerFnResult<Vec<BlogPost>> {
///     let posts = load_posts(&category).await?;
///     // maybe do some other work
///     Ok(posts)
/// }
/// ```
///
/// ## Named Arguments
///
/// You can use any combination of the following named arguments:
/// - `name`: sets the identifier for the server function’s type, which is a struct created
///   to hold the arguments (defaults to the function identifier in PascalCase).
///   Example: `name = MyServerFunction`.
/// - `prefix`: a prefix at which the server function handler will be mounted (defaults to `/api`).
///   Example: `prefix = "/my_api"`.
/// - `endpoint`: specifies the exact path at which the server function handler will be mounted,
///   relative to the prefix (defaults to the function name followed by unique hash).
///   Example: `endpoint = "my_fn"`.
/// - `input`: the encoding for the arguments (defaults to `PostUrl`).
///     - The `input` argument specifies how the function arguments are encoded for transmission.
///     - Acceptable values include:
///       - `PostUrl`: A `POST` request with URL-encoded arguments, suitable for form-like submissions.
///       - `Json`: A `POST` request where the arguments are encoded as JSON. This is a common choice for modern APIs.
///       - `Cbor`: A `POST` request with CBOR-encoded arguments, useful for binary data transmission with compact encoding.
///       - `GetUrl`: A `GET` request with URL-encoded arguments, suitable for simple queries or when data fits in the URL.
///       - `GetCbor`: A `GET` request with CBOR-encoded arguments, useful for query-style APIs when the payload is binary.
/// - `output`: the encoding for the response (defaults to `Json`).
///     - The `output` argument specifies how the server should encode the response data.
///     - Acceptable values include:
///       - `Json`: A response encoded as JSON (default). This is ideal for most web applications.
///       - `Cbor`: A response encoded in the CBOR format for efficient, binary-encoded data.
/// - `client`: a custom `Client` implementation that will be used for this server function. This allows
///   customization of the client-side behavior if needed.
/// - `encoding`: (legacy, may be deprecated in future) specifies the encoding, which may be one
///   of the following (not case sensitive):
///     - `"Url"`: `POST` request with URL-encoded arguments and JSON response
///     - `"GetUrl"`: `GET` request with URL-encoded arguments and JSON response
///     - `"Cbor"`: `POST` request with CBOR-encoded arguments and response
///     - `"GetCbor"`: `GET` request with URL-encoded arguments and CBOR response
/// - `req` and `res`: specify the HTTP request and response types to be used on the server. These
///   are typically necessary if you are integrating with a custom server framework (other than Actix/Axum).
///   Example: `req = SomeRequestType`, `res = SomeResponseType`.
///
/// ## Advanced Usage of `input` and `output` Fields
///
/// The `input` and `output` fields allow you to customize how arguments and responses are encoded and decoded.
/// These fields impose specific trait bounds on the types you use. Here are detailed examples for different scenarios:
///
/// ### `output = StreamingJson`
///
/// Setting the `output` type to `StreamingJson` requires the return type to implement `From<JsonStream<T>>`,
/// where `T` implements `serde::Serialize` and `serde::de::DeserializeOwned`.
///
/// ```rust,ignore
/// #[server(output = StreamingJson)]
/// pub async fn json_stream_fn() -> Result<JsonStream<String>, ServerFnError> {
///     todo!()
/// }
/// ```
///
/// ### `output = StreamingText`
///
/// Setting the `output` type to `StreamingText` requires the return type to implement `From<TextStream>`.
///
/// ```rust,ignore
/// #[server(output = StreamingText)]
/// pub async fn text_stream_fn() -> Result<TextStream, ServerFnError> {
///     todo!()
/// }
/// ```
///
/// ### `output = PostUrl`
///
/// Setting the `output` type to `PostUrl` requires the return type to implement `Serialize` and `Deserialize`.
/// Note that this uses `serde_qs`, which imposes the following constraints:
/// - The structure must be less than 5 levels deep.
/// - The structure must not contain any `serde(flatten)` attributes.
///
/// ```rust,ignore
/// #[server(output = PostUrl)]
/// pub async fn form_fn() -> Result<TextStream, ServerFnError> {
///     todo!()
/// }
/// ```
///
/// These examples illustrate how the `output` type impacts the bounds and expectations for your server function. Ensure your return types comply with these requirements.
///
///
/// ```rust,ignore
/// #[server(
///   name = SomeStructName,
///   prefix = "/my_api",
///   endpoint = "my_fn",
///   input = Cbor,
///   output = Json
/// )]
/// pub async fn my_wacky_server_fn(input: Vec<String>) -> ServerFnResult<usize> {
///   unimplemented!()
/// }
///
/// // expands to
/// #[derive(Deserialize, Serialize)]
/// struct SomeStructName {
///   input: Vec<String>
/// }
///
/// impl ServerFn for SomeStructName {
///   const PATH: &'static str = "/my_api/my_fn";
///
///   // etc.
/// }
/// ```
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
///
/// ## Extracting additional data from requests
///
/// Server functions automatically handle serialization and deserialization of arguments and responses.
/// However, you may want to extract additional data from the request, such as the user's session or
/// authentication information. You can do this with the `extract` function. This function returns any
/// type that implements the [`FromRequestParts`](https://docs.rs/axum/latest/axum/extract/trait.FromRequestParts.html)
/// trait:
///
/// ```rust,ignore
/// # use dioxus::prelude::*;
/// #[server]
/// pub async fn my_wacky_server_fn(input: Vec<String>) -> ServerFnResult<String> {
///     let headers: axum::http::header::HeaderMap = extract().await?;
///     Ok(format!("The server got a request with headers: {:?}", headers))
/// }
/// ```
///
/// ## Sharing data with server functions
///
/// You may need to share context with your server functions like a database pool. Server
/// functions can access any context provided through the launch builder. You can access
/// this context with the `FromContext` extractor:
///
/// ```rust,ignore
/// # use dioxus::prelude::*;
/// # fn app() -> Element { unimplemented!() }
/// #[derive(Clone, Copy, Debug)]
/// struct DatabasePool;
///
/// fn main() {
///     LaunchBuilder::new()
///         .with_context(server_only! {
///             DatabasePool
///         })
///         .launch(app);
/// }
///
/// #[server]
/// pub async fn my_wacky_server_fn(input: Vec<String>) -> ServerFnResult<String> {
///     let FromContext(pool): FromContext<DatabasePool> = extract().await?;
///     Ok(format!("The server read {:?} from the shared context", pool))
/// }
/// ```
#[proc_macro_attribute]
pub fn server(args: proc_macro::TokenStream, body: TokenStream) -> TokenStream {
    // If there is no input codec, use json as the default
    #[allow(unused_mut)]
    let mut parsed = match ServerFnCall::parse("/api", args.into(), body.into()) {
        Ok(parsed) => parsed,
        Err(e) => return e.to_compile_error().into(),
    };

    parsed
        .default_protocol(Some(
            parse_quote!(server_fn::Http<server_fn::codec::Json, server_fn::codec::Json>),
        ))
        .default_input_encoding(Some(parse_quote!(server_fn::codec::Json)))
        .default_output_encoding(Some(parse_quote!(server_fn::codec::Json)))
        .default_server_fn_path(Some(parse_quote!(server_fn)))
        .to_token_stream()
        .into()
}



//! Implementation of the `server_fn` macro.
//!
//! This crate contains the implementation of the `server_fn` macro. [`server_macro_impl`] can be used to implement custom versions of the macro for different frameworks that allow users to pass a custom context from the server to the server function.

use convert_case::{Case, Converter};
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{format_ident, quote, quote_spanned, ToTokens};
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    spanned::Spanned,
    *,
};

/// A parsed server function call.
pub struct ServerFnCall {
    args: ServerFnArgs,
    body: ServerFnBody,
    default_path: String,
    server_fn_path: Option<Path>,
    preset_server: Option<Type>,
    default_protocol: Option<Type>,
    default_input_encoding: Option<Type>,
    default_output_encoding: Option<Type>,
}

impl ServerFnCall {
    /// Parse the arguments of a server function call.
    ///
    /// ```ignore
    /// #[proc_macro_attribute]
    /// pub fn server(args: proc_macro::TokenStream, s: TokenStream) -> TokenStream {
    ///     match ServerFnCall::parse(
    ///         "/api",
    ///         args.into(),
    ///         s.into(),
    ///     ) {
    ///         Err(e) => e.to_compile_error().into(),
    ///         Ok(s) => s.to_token_stream().into(),
    ///     }
    /// }
    /// ```
    pub fn parse(default_path: &str, args: TokenStream2, body: TokenStream2) -> Result<Self> {
        let args = syn::parse2(args)?;
        let body = syn::parse2(body)?;
        let mut myself = ServerFnCall {
            default_path: default_path.into(),
            args,
            body,
            server_fn_path: None,
            preset_server: None,
            default_protocol: None,
            default_input_encoding: None,
            default_output_encoding: None,
        };

        Ok(myself)
    }

    /// Get a reference to the server function arguments.
    pub fn get_args(&self) -> &ServerFnArgs {
        &self.args
    }

    /// Get a mutable reference to the server function arguments.
    pub fn get_args_mut(&mut self) -> &mut ServerFnArgs {
        &mut self.args
    }

    /// Get a reference to the server function body.
    pub fn get_body(&self) -> &ServerFnBody {
        &self.body
    }

    /// Get a mutable reference to the server function body.
    pub fn get_body_mut(&mut self) -> &mut ServerFnBody {
        &mut self.body
    }

    /// Set the path to the server function crate.
    pub fn default_server_fn_path(mut self, path: Option<Path>) -> Self {
        self.server_fn_path = path;
        self
    }

    /// Set the default server implementation.
    pub fn default_server_type(mut self, server: Option<Type>) -> Self {
        self.preset_server = server;
        self
    }

    /// Set the default protocol.
    pub fn default_protocol(mut self, protocol: Option<Type>) -> Self {
        self.default_protocol = protocol;
        self
    }

    /// Set the default input http encoding. This will be used by [`Self::protocol`]
    /// if no protocol or default protocol is set or if only the output encoding is set
    ///
    /// Defaults to `PostUrl`
    pub fn default_input_encoding(mut self, protocol: Option<Type>) -> Self {
        self.default_input_encoding = protocol;
        self
    }

    /// Set the default output http encoding. This will be used by [`Self::protocol`]
    /// if no protocol or default protocol is set or if only the input encoding is set
    ///
    /// Defaults to `Json`
    pub fn default_output_encoding(mut self, protocol: Option<Type>) -> Self {
        self.default_output_encoding = protocol;
        self
    }

    /// Get the client type to use for the server function.
    pub fn client_type(&self) -> Type {
        let server_fn_path = self.server_fn_path();
        if let Some(client) = self.args.client.clone() {
            client
        } else if cfg!(feature = "reqwest") {
            parse_quote! {
                #server_fn_path::client::reqwest::ReqwestClient
            }
        } else {
            parse_quote! {
                #server_fn_path::client::browser::BrowserClient
            }
        }
    }

    /// Get the server type to use for the server function.
    pub fn server_type(&self) -> Type {
        let server_fn_path = self.server_fn_path();
        if !cfg!(feature = "ssr") {
            parse_quote! {
                #server_fn_path::mock::BrowserMockServer
            }
        } else if cfg!(feature = "axum") {
            parse_quote! {
                #server_fn_path::axum::AxumServerFnBackend
            }
        } else if cfg!(feature = "generic") {
            parse_quote! {
                #server_fn_path::axum::AxumServerFnBackend
            }
        } else if let Some(server) = &self.args.server {
            server.clone()
        } else if let Some(server) = &self.preset_server {
            server.clone()
        } else {
            // fall back to the browser version, to avoid erroring out
            // in things like doctests
            // in reality, one of the above needs to be set
            parse_quote! {
                #server_fn_path::mock::BrowserMockServer
            }
        }
    }

    /// Get the path to the server_fn crate.
    pub fn server_fn_path(&self) -> Path {
        self.server_fn_path
            .clone()
            .unwrap_or_else(|| parse_quote! { server_fn })
    }

    /// Get the input http encoding if no protocol is set
    fn input_http_encoding(&self) -> Type {
        let server_fn_path = self.server_fn_path();
        self.args
            .input
            .as_ref()
            .map(|n| {
                if self.args.builtin_encoding {
                    parse_quote! { #server_fn_path::codec::#n }
                } else {
                    n.clone()
                }
            })
            .unwrap_or_else(|| {
                self.default_input_encoding
                    .clone()
                    .unwrap_or_else(|| parse_quote!(#server_fn_path::codec::PostUrl))
            })
    }

    /// Get the output http encoding if no protocol is set
    fn output_http_encoding(&self) -> Type {
        let server_fn_path = self.server_fn_path();
        self.args
            .output
            .as_ref()
            .map(|n| {
                if self.args.builtin_encoding {
                    parse_quote! { #server_fn_path::codec::#n }
                } else {
                    n.clone()
                }
            })
            .unwrap_or_else(|| {
                self.default_output_encoding
                    .clone()
                    .unwrap_or_else(|| parse_quote!(#server_fn_path::codec::Json))
            })
    }

    /// Get the http input and output encodings for the server function
    /// if no protocol is set
    pub fn http_encodings(&self) -> Option<(Type, Type)> {
        self.args
            .protocol
            .is_none()
            .then(|| (self.input_http_encoding(), self.output_http_encoding()))
    }

    /// Get the protocol to use for the server function.
    pub fn protocol(&self) -> Type {
        let server_fn_path = self.server_fn_path();
        let default_protocol = &self.default_protocol;
        self.args.protocol.clone().unwrap_or_else(|| {
            // If both the input and output encodings are none,
            // use the default protocol
            if self.args.input.is_none() && self.args.output.is_none() {
                default_protocol.clone().unwrap_or_else(|| {
                    parse_quote! {
                        #server_fn_path::Http<#server_fn_path::codec::PostUrl, #server_fn_path::codec::Json>
                    }
                })
            } else {
                // Otherwise use the input and output encodings, filling in
                // defaults if necessary
                let input = self.input_http_encoding();
                let output = self.output_http_encoding();
                parse_quote! {
                    #server_fn_path::Http<#input, #output>
                }
            }
        })
    }

    fn input_ident(&self) -> Option<String> {
        match &self.args.input {
            Some(Type::Path(path)) => path.path.segments.last().map(|seg| seg.ident.to_string()),
            None => Some("PostUrl".to_string()),
            _ => None,
        }
    }

    fn websocket_protocol(&self) -> bool {
        if let Type::Path(path) = self.protocol() {
            path.path
                .segments
                .iter()
                .any(|segment| segment.ident == "Websocket")
        } else {
            false
        }
    }

    fn serde_path(&self) -> String {
        let path = self
            .server_fn_path()
            .segments
            .iter()
            .map(|segment| segment.ident.to_string())
            .collect::<Vec<_>>();
        let path = path.join("::");
        format!("{path}::serde")
    }

    /// Get the docs for the server function.
    pub fn docs(&self) -> TokenStream2 {
        // pass through docs from the function body
        self.body
            .docs
            .iter()
            .map(|(doc, span)| quote_spanned!(*span=> #[doc = #doc]))
            .collect::<TokenStream2>()
    }

    fn fn_name_as_str(&self) -> String {
        self.body.ident.to_string()
    }

    fn struct_tokens(&self) -> TokenStream2 {
        let server_fn_path = self.server_fn_path();
        let fn_name_as_str = self.fn_name_as_str();
        let link_to_server_fn = format!(
            "Serialized arguments for the [`{fn_name_as_str}`] server \
             function.\n\n"
        );
        let args_docs = quote! {
            #[doc = #link_to_server_fn]
        };

        let docs = self.docs();

        let input_ident = self.input_ident();

        enum PathInfo {
            Serde,
            Rkyv,
            None,
        }

        let (path, derives) = match input_ident.as_deref() {
            Some("Rkyv") => (
                PathInfo::Rkyv,
                quote! {
                    Clone, #server_fn_path::rkyv::Archive, #server_fn_path::rkyv::Serialize, #server_fn_path::rkyv::Deserialize
                },
            ),
            Some("MultipartFormData") | Some("Streaming") | Some("StreamingText") => {
                (PathInfo::None, quote! {})
            }
            Some("SerdeLite") => (
                PathInfo::Serde,
                quote! {
                    Clone, #server_fn_path::serde_lite::Serialize, #server_fn_path::serde_lite::Deserialize
                },
            ),
            _ => match &self.args.input_derive {
                Some(derives) => {
                    let d = &derives.elems;
                    (PathInfo::None, quote! { #d })
                }
                None => {
                    if self.websocket_protocol() {
                        (PathInfo::None, quote! {})
                    } else {
                        (
                            PathInfo::Serde,
                            quote! {
                                Clone, #server_fn_path::serde::Serialize, #server_fn_path::serde::Deserialize
                            },
                        )
                    }
                }
            },
        };
        let addl_path = match path {
            PathInfo::Serde => {
                let serde_path = self.serde_path();
                quote! {
                    #[serde(crate = #serde_path)]
                }
            }
            PathInfo::Rkyv => quote! {},
            PathInfo::None => quote! {},
        };

        let vis = &self.body.vis;
        let struct_name = self.struct_name();
        let fields = self
            .body
            .inputs
            .iter()
            .map(|server_fn_arg| {
                let mut typed_arg = server_fn_arg.arg.clone();
                // strip `mut`, which is allowed in fn args but not in struct fields
                if let Pat::Ident(ident) = &mut *typed_arg.pat {
                    ident.mutability = None;
                }
                let attrs = &server_fn_arg.server_fn_attributes;
                quote! { #(#attrs ) * #vis #typed_arg }
            })
            .collect::<Vec<_>>();

        quote! {
            #args_docs
            #docs
            #[derive(Debug, #derives)]
            #addl_path
            #vis struct #struct_name {
                #(#fields),*
            }
        }
    }

    /// Get the name of the server function struct that will be submitted to inventory.
    ///
    /// This will either be the name specified in the macro arguments or the PascalCase
    /// version of the function name.
    pub fn struct_name(&self) -> Ident {
        // default to PascalCase version of function name if no struct name given
        self.args.struct_name.clone().unwrap_or_else(|| {
            let upper_camel_case_name = Converter::new()
                .from_case(Case::Snake)
                .to_case(Case::UpperCamel)
                .convert(self.body.ident.to_string());
            Ident::new(&upper_camel_case_name, self.body.ident.span())
        })
    }

    /// Wrap the struct name in any custom wrapper specified in the macro arguments
    /// and return it as a type
    fn wrapped_struct_name(&self) -> TokenStream2 {
        let struct_name = self.struct_name();
        if let Some(wrapper) = self.args.custom_wrapper.as_ref() {
            quote! { #wrapper<#struct_name> }
        } else {
            quote! { #struct_name }
        }
    }

    /// Wrap the struct name in any custom wrapper specified in the macro arguments
    /// and return it as a type with turbofish
    fn wrapped_struct_name_turbofish(&self) -> TokenStream2 {
        let struct_name = self.struct_name();
        if let Some(wrapper) = self.args.custom_wrapper.as_ref() {
            quote! { #wrapper::<#struct_name> }
        } else {
            quote! { #struct_name }
        }
    }

    /// Generate the code to submit the server function type to inventory.
    pub fn submit_to_inventory(&self) -> TokenStream2 {
        // auto-registration with inventory
        if cfg!(feature = "ssr") {
            let server_fn_path = self.server_fn_path();
            let wrapped_struct_name = self.wrapped_struct_name();
            let wrapped_struct_name_turbofish = self.wrapped_struct_name_turbofish();
            quote! {
                #server_fn_path::inventory::submit! {{
                    use #server_fn_path::{ServerFn, codec::Encoding};
                    #server_fn_path::ServerFnTraitObj::new::<#wrapped_struct_name>(
                        |req| Box::pin(#wrapped_struct_name_turbofish::run_on_server(req)),
                    )
                }}
            }
        } else {
            quote! {}
        }
    }

    /// Generate the server function's URL. This will be the prefix path, then by the
    /// module path if `SERVER_FN_MOD_PATH` is set, then the function name, and finally
    /// a hash of the function name and location in the source code.
    pub fn server_fn_url(&self) -> TokenStream2 {
        let default_path = &self.default_path;
        let prefix = self
            .args
            .prefix
            .clone()
            .unwrap_or_else(|| LitStr::new(default_path, Span::call_site()));
        let server_fn_path = self.server_fn_path();
        let fn_path = self.args.fn_path.clone().map(|fn_path| {
            let fn_path = fn_path.value();
            // Remove any leading slashes, then add one slash back
            let fn_path = "/".to_string() + fn_path.trim_start_matches('/');
            fn_path
        });

        let enable_server_fn_mod_path = option_env!("SERVER_FN_MOD_PATH").is_some();
        let mod_path = if enable_server_fn_mod_path {
            quote! {
                #server_fn_path::const_format::concatcp!(
                    #server_fn_path::const_str::replace!(module_path!(), "::", "/"),
                    "/"
                )
            }
        } else {
            quote! { "" }
        };

        let enable_hash = option_env!("DISABLE_SERVER_FN_HASH").is_none();
        let key_env_var = match option_env!("SERVER_FN_OVERRIDE_KEY") {
            Some(_) => "SERVER_FN_OVERRIDE_KEY",
            None => "CARGO_MANIFEST_DIR",
        };
        let hash = if enable_hash {
            quote! {
                #server_fn_path::xxhash_rust::const_xxh64::xxh64(
                    concat!(env!(#key_env_var), ":", module_path!()).as_bytes(),
                    0
                )
            }
        } else {
            quote! { "" }
        };

        let fn_name_as_str = self.fn_name_as_str();
        if let Some(fn_path) = fn_path {
            quote! {
                #server_fn_path::const_format::concatcp!(
                    #prefix,
                    #mod_path,
                    #fn_path
                )
            }
        } else {
            quote! {
                #server_fn_path::const_format::concatcp!(
                    #prefix,
                    "/",
                    #mod_path,
                    #fn_name_as_str,
                    #hash
                )
            }
        }
    }

    /// Get the names of the fields the server function takes as inputs.
    fn field_names(&self) -> Vec<&std::boxed::Box<syn::Pat>> {
        self.body
            .inputs
            .iter()
            .map(|f| &f.arg.pat)
            .collect::<Vec<_>>()
    }

    /// Generate the implementation for the server function trait.
    fn server_fn_impl(&self) -> TokenStream2 {
        let server_fn_path = self.server_fn_path();
        let struct_name = self.struct_name();

        let protocol = self.protocol();
        let middlewares = &self.body.middlewares;
        let return_ty = &self.body.return_ty;
        let output_ty = self.body.output_ty.as_ref().map_or_else(
            || {
                quote! {
                    <#return_ty as #server_fn_path::error::ServerFnMustReturnResult>::Ok
                }
            },
            ToTokens::to_token_stream,
        );
        let error_ty = &self.body.error_ty;
        let error_ty = error_ty.as_ref().map_or_else(
            || {
                quote! {
                    <#return_ty as #server_fn_path::error::ServerFnMustReturnResult>::Err
                }
            },
            ToTokens::to_token_stream,
        );
        let error_ws_in_ty = if self.websocket_protocol() {
            self.body
                .error_ws_in_ty
                .as_ref()
                .map(ToTokens::to_token_stream)
                .unwrap_or(error_ty.clone())
        } else {
            error_ty.clone()
        };
        let error_ws_out_ty = if self.websocket_protocol() {
            self.body
                .error_ws_out_ty
                .as_ref()
                .map(ToTokens::to_token_stream)
                .unwrap_or(error_ty.clone())
        } else {
            error_ty.clone()
        };
        let field_names = self.field_names();

        // run_body in the trait implementation
        let run_body = if cfg!(feature = "ssr") {
            let destructure = if let Some(wrapper) = self.args.custom_wrapper.as_ref() {
                quote! {
                    let #wrapper(#struct_name { #(#field_names),* }) = self;
                }
            } else {
                quote! {
                    let #struct_name { #(#field_names),* } = self;
                }
            };
            let dummy_name = self.body.to_dummy_ident();

            // using the impl Future syntax here is thanks to Actix
            //
            // if we use Actix types inside the function, here, it becomes !Send
            // so we need to add SendWrapper, because Actix won't actually send it anywhere
            // but if we used SendWrapper in an async fn, the types don't work out because it
            // becomes impl Future<Output = SendWrapper<_>>
            //
            // however, SendWrapper<Future<Output = T>> impls Future<Output = T>
            let body = quote! {
                async move {
                    #destructure
                    #dummy_name(#(#field_names),*).await
                }
            };
            quote! {
                // we need this for Actix, for the SendWrapper to count as impl Future
                // but non-Actix will have a clippy warning otherwise
                #[allow(clippy::manual_async_fn)]
                fn run_body(self) -> impl std::future::Future<Output = #return_ty> + Send {
                    #body
                }
            }
        } else {
            quote! {
                #[allow(unused_variables)]
                async fn run_body(self) -> #return_ty {
                    unreachable!()
                }
            }
        };
        let client = self.client_type();

        let server = self.server_type();

        // generate the url of the server function
        let path = self.server_fn_url();

        let middlewares = if cfg!(feature = "ssr") {
            quote! {
                vec![
                    #(
                        std::sync::Arc::new(#middlewares)
                    ),*
                ]
            }
        } else {
            quote! { vec![] }
        };
        let wrapped_struct_name = self.wrapped_struct_name();

        quote! {
            impl #server_fn_path::ServerFn for #wrapped_struct_name {
                const PATH: &'static str = #path;

                type Client = #client;
                type Server = #server;
                type Protocol = #protocol;
                type Output = #output_ty;
                type Error = #error_ty;
                type InputStreamError = #error_ws_in_ty;
                type OutputStreamError = #error_ws_out_ty;

                fn middlewares() -> Vec<std::sync::Arc<dyn #server_fn_path::middleware::Layer<<Self::Server as #server_fn_path::server::Server<Self::Error>>::Request, <Self::Server as #server_fn_path::server::Server<Self::Error>>::Response>>> {
                    #middlewares
                }

                #run_body
            }
        }
    }

    /// Return the name and type of the first field if there is only one field.
    fn single_field(&self) -> Option<(&Pat, &Type)> {
        self.body
            .inputs
            .first()
            .filter(|_| self.body.inputs.len() == 1)
            .map(|field| (&*field.arg.pat, &*field.arg.ty))
    }

    fn deref_impl(&self) -> TokenStream2 {
        let impl_deref = self
            .args
            .impl_deref
            .as_ref()
            .map(|v| v.value)
            .unwrap_or(true)
            || self.websocket_protocol();
        if !impl_deref {
            return quote! {};
        }
        // if there's exactly one field, impl Deref<T> for the struct
        let Some(single_field) = self.single_field() else {
            return quote! {};
        };
        let struct_name = self.struct_name();
        let (name, ty) = single_field;
        quote! {
            impl std::ops::Deref for #struct_name {
                type Target = #ty;
                fn deref(&self) -> &Self::Target {
                    &self.#name
                }
            }
        }
    }

    fn impl_from(&self) -> TokenStream2 {
        let impl_from = self
            .args
            .impl_from
            .as_ref()
            .map(|v| v.value)
            .unwrap_or(true)
            || self.websocket_protocol();
        if !impl_from {
            return quote! {};
        }
        // if there's exactly one field, impl From<T> for the struct
        let Some(single_field) = self.single_field() else {
            return quote! {};
        };
        let struct_name = self.struct_name();
        let (name, ty) = single_field;
        quote! {
            impl From<#struct_name> for #ty {
                fn from(value: #struct_name) -> Self {
                    let #struct_name { #name } = value;
                    #name
                }
            }

            impl From<#ty> for #struct_name {
                fn from(#name: #ty) -> Self {
                    #struct_name { #name }
                }
            }
        }
    }

    fn func_tokens(&self) -> TokenStream2 {
        let body = &self.body;
        // default values for args
        let struct_name = self.struct_name();

        // build struct for type
        let fn_name = &body.ident;
        let attrs = &body.attrs;

        let fn_args = body.inputs.iter().map(|f| &f.arg).collect::<Vec<_>>();

        let field_names = self.field_names();

        // check output type
        let output_arrow = body.output_arrow;
        let return_ty = &body.return_ty;
        let vis = &body.vis;

        // Forward the docs from the function
        let docs = self.docs();

        // the actual function definition
        if cfg!(feature = "ssr") {
            let dummy_name = body.to_dummy_ident();
            quote! {
                #docs
                #(#attrs)*
                #vis async fn #fn_name(#(#fn_args),*) #output_arrow #return_ty {
                    #dummy_name(#(#field_names),*).await
                }
            }
        } else {
            let restructure = if let Some(custom_wrapper) = self.args.custom_wrapper.as_ref() {
                quote! {
                    let data = #custom_wrapper(#struct_name { #(#field_names),* });
                }
            } else {
                quote! {
                    let data = #struct_name { #(#field_names),* };
                }
            };
            let server_fn_path = self.server_fn_path();
            quote! {
                #docs
                #(#attrs)*
                #[allow(unused_variables)]
                #vis async fn #fn_name(#(#fn_args),*) #output_arrow #return_ty {
                    use #server_fn_path::ServerFn;
                    #restructure
                    data.run_on_client().await
                }
            }
        }
    }
}

impl ToTokens for ServerFnCall {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let body = &self.body;

        // only emit the dummy (unmodified server-only body) for the server build
        let dummy = cfg!(feature = "ssr").then(|| body.to_dummy_output());

        let impl_from = self.impl_from();

        let deref_impl = self.deref_impl();

        let inventory = self.submit_to_inventory();

        let func = self.func_tokens();

        let server_fn_impl = self.server_fn_impl();

        let struct_tokens = self.struct_tokens();

        tokens.extend(quote! {
            #struct_tokens

            #impl_from

            #deref_impl

            #server_fn_impl

            #inventory

            #func

            #dummy
        });
    }
}

/// The implementation of the `server` macro.
/// ```ignore
/// #[proc_macro_attribute]
/// pub fn server(args: proc_macro::TokenStream, s: TokenStream) -> TokenStream {
///     match server_macro_impl(
///         args.into(),
///         s.into(),
///         Some(syn::parse_quote!(my_crate::exports::server_fn)),
///     ) {
///         Err(e) => e.to_compile_error().into(),
///         Ok(s) => s.to_token_stream().into(),
///     }
/// }
/// ```
pub fn server_macro_impl(
    args: TokenStream2,
    body: TokenStream2,
    server_fn_path: Option<Path>,
    default_path: &str,
    preset_server: Option<Type>,
    default_protocol: Option<Type>,
) -> Result<TokenStream2> {
    let body = ServerFnCall::parse(default_path, args, body)?
        .default_server_fn_path(server_fn_path)
        .default_server_type(preset_server)
        .default_protocol(default_protocol);

    Ok(body.to_token_stream())
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

/// Middleware for a server function.
#[derive(Debug, Clone)]
pub struct Middleware {
    expr: syn::Expr,
}

impl ToTokens for Middleware {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let expr = &self.expr;
        tokens.extend(quote::quote! {
            #expr
        });
    }
}

impl Parse for Middleware {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let arg: syn::Expr = input.parse()?;
        Ok(Middleware { expr: arg })
    }
}

fn output_type(return_ty: &Type) -> Option<&Type> {
    if let syn::Type::Path(pat) = &return_ty {
        if pat.path.segments[0].ident == "Result" {
            if pat.path.segments.is_empty() {
                panic!("{:#?}", pat.path);
            } else if let PathArguments::AngleBracketed(args) = &pat.path.segments[0].arguments {
                if let GenericArgument::Type(ty) = &args.args[0] {
                    return Some(ty);
                }
            }
        }
    };

    None
}

fn err_type(return_ty: &Type) -> Option<&Type> {
    if let syn::Type::Path(pat) = &return_ty {
        if pat.path.segments[0].ident == "Result" {
            if let PathArguments::AngleBracketed(args) = &pat.path.segments[0].arguments {
                // Result<T>
                if args.args.len() == 1 {
                    return None;
                }
                // Result<T, _>
                else if let GenericArgument::Type(ty) = &args.args[1] {
                    return Some(ty);
                }
            }
        }
    };

    None
}

fn err_ws_in_type(inputs: &Punctuated<ServerFnArg, syn::token::Comma>) -> Option<Type> {
    inputs.into_iter().find_map(|pat| {
        if let syn::Type::Path(ref pat) = *pat.arg.ty {
            if pat.path.segments[0].ident != "BoxedStream" {
                return None;
            }

            if let PathArguments::AngleBracketed(args) = &pat.path.segments[0].arguments {
                // BoxedStream<T>
                if args.args.len() == 1 {
                    return None;
                }
                // BoxedStream<T, E>
                else if let GenericArgument::Type(ty) = &args.args[1] {
                    return Some(ty.clone());
                }
            };
        };

        None
    })
}

fn err_ws_out_type(output_ty: &Option<Type>) -> Result<Option<Type>> {
    if let Some(syn::Type::Path(ref pat)) = output_ty {
        if pat.path.segments[0].ident == "BoxedStream" {
            if let PathArguments::AngleBracketed(args) = &pat.path.segments[0].arguments {
                // BoxedStream<T>
                if args.args.len() == 1 {
                    return Ok(None);
                }
                // BoxedStream<T, E>
                else if let GenericArgument::Type(ty) = &args.args[1] {
                    return Ok(Some(ty.clone()));
                }

                return Err(syn::Error::new(
                    output_ty.span(),
                    "websocket server functions should return \
                     BoxedStream<Result<T, E>> where E: FromServerFnError",
                ));
            };
        }
    };

    Ok(None)
}

/// The arguments to the `server` macro.
#[derive(Debug)]
#[non_exhaustive]
pub struct ServerFnArgs {
    /// The name of the struct that will implement the server function trait
    /// and be submitted to inventory.
    pub struct_name: Option<Ident>,
    /// The prefix to use for the server function URL.
    pub prefix: Option<LitStr>,
    /// The input http encoding to use for the server function.
    pub input: Option<Type>,
    /// Additional traits to derive on the input struct for the server function.
    pub input_derive: Option<ExprTuple>,
    /// The output http encoding to use for the server function.
    pub output: Option<Type>,
    /// The path to the server function crate.
    pub fn_path: Option<LitStr>,
    /// The server type to use for the server function.
    pub server: Option<Type>,
    /// The client type to use for the server function.
    pub client: Option<Type>,
    /// The custom wrapper to use for the server function struct.
    pub custom_wrapper: Option<Path>,
    /// If the generated input type should implement `From` the only field in the input
    pub impl_from: Option<LitBool>,
    /// If the generated input type should implement `Deref` to the only field in the input
    pub impl_deref: Option<LitBool>,
    /// The protocol to use for the server function implementation.
    pub protocol: Option<Type>,
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
        let mut custom_wrapper: Option<Path> = None;
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
#[derive(Debug, Clone)]
pub struct ServerFnArg {
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

/// The body of a server function.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ServerFnBody {
    /// The attributes on the server function.
    pub attrs: Vec<Attribute>,
    /// The visibility of the server function.
    pub vis: syn::Visibility,
    async_token: Token![async],
    fn_token: Token![fn],
    /// The name of the server function.
    pub ident: Ident,
    /// The generics of the server function.
    pub generics: Generics,
    _paren_token: token::Paren,
    /// The arguments to the server function.
    pub inputs: Punctuated<ServerFnArg, Token![,]>,
    output_arrow: Token![->],
    /// The return type of the server function.
    pub return_ty: syn::Type,
    /// The Ok output type of the server function.
    pub output_ty: Option<syn::Type>,
    /// The error output type of the server function.
    pub error_ty: Option<syn::Type>,
    /// The error type of WebSocket client-sent error
    pub error_ws_in_ty: Option<syn::Type>,
    /// The error type of WebSocket server-sent error
    pub error_ws_out_ty: Option<syn::Type>,
    /// The body of the server function.
    pub block: TokenStream2,
    /// The documentation of the server function.
    pub docs: Vec<(String, Span)>,
    /// The middleware attributes applied to the server function.
    pub middlewares: Vec<Middleware>,
}

impl Parse for ServerFnBody {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut attrs: Vec<Attribute> = input.call(Attribute::parse_outer)?;

        let vis: Visibility = input.parse()?;

        let async_token = input.parse()?;

        let fn_token = input.parse()?;
        let ident = input.parse()?;
        let generics: Generics = input.parse()?;

        let content;
        let _paren_token = syn::parenthesized!(content in input);

        let inputs = syn::punctuated::Punctuated::parse_terminated(&content)?;

        let output_arrow = input.parse()?;
        let return_ty = input.parse()?;
        let output_ty = output_type(&return_ty).cloned();
        let error_ty = err_type(&return_ty).cloned();
        let error_ws_in_ty = err_ws_in_type(&inputs);
        let error_ws_out_ty = err_ws_out_type(&output_ty)?;

        let block = input.parse()?;

        let docs = attrs
            .iter()
            .filter_map(|attr| {
                let Meta::NameValue(attr) = &attr.meta else {
                    return None;
                };
                if !attr.path.is_ident("doc") {
                    return None;
                }

                let value = match &attr.value {
                    syn::Expr::Lit(lit) => match &lit.lit {
                        syn::Lit::Str(s) => Some(s.value()),
                        _ => return None,
                    },
                    _ => return None,
                };

                Some((value.unwrap_or_default(), attr.path.span()))
            })
            .collect();
        attrs.retain(|attr| {
            let Meta::NameValue(attr) = &attr.meta else {
                return true;
            };
            !attr.path.is_ident("doc")
        });
        // extract all #[middleware] attributes, removing them from signature of dummy
        let mut middlewares: Vec<Middleware> = vec![];
        attrs.retain(|attr| {
            if attr.meta.path().is_ident("middleware") {
                if let Ok(middleware) = attr.parse_args() {
                    middlewares.push(middleware);
                    false
                } else {
                    true
                }
            } else {
                // in ssr mode, remove the "lazy" macro
                // the lazy macro doesn't do anything on the server anyway, but it can cause confusion for rust-analyzer
                // when the lazy macro is applied to both the function and the dummy
                !(cfg!(feature = "ssr") && matches!(attr.meta.path().segments.last(), Some(PathSegment { ident, .. }) if ident == "lazy") )
            }
        });

        Ok(Self {
            vis,
            async_token,
            fn_token,
            ident,
            generics,
            _paren_token,
            inputs,
            output_arrow,
            return_ty,
            output_ty,
            error_ty,
            error_ws_in_ty,
            error_ws_out_ty,
            block,
            attrs,
            docs,
            middlewares,
        })
    }
}

impl ServerFnBody {
    fn to_dummy_ident(&self) -> Ident {
        Ident::new(&format!("__server_{}", self.ident), self.ident.span())
    }

    fn to_dummy_output(&self) -> TokenStream2 {
        let ident = self.to_dummy_ident();
        let Self {
            attrs,
            vis,
            async_token,
            fn_token,
            generics,
            inputs,
            output_arrow,
            return_ty,
            block,
            ..
        } = &self;
        quote! {
            #[doc(hidden)]
            #(#attrs)*
            #vis #async_token #fn_token #ident #generics ( #inputs ) #output_arrow #return_ty
            #block
        }
    }
}

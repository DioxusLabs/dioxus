// TODO: Create README, uncomment this: #![doc = include_str!("../README.md")]
#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]

//! This crate contains the dioxus implementation of the #[macro@crate::server] macro without additional context from the server.
//! See the [server_fn_macro] crate for more information.

use proc_macro::TokenStream;
use server_fn_macro::server_macro_impl;
use syn::__private::ToTokens;

/// Declares that a function is a [server function](https://docs.rs/server_fn/).
/// This means that its body will only run on the server, i.e., when the `ssr`
/// feature is enabled on this crate.
///
/// ## Usage
/// ```rust,ignore
/// # use dioxus::prelude::*;
/// # #[derive(serde::Deserialize, serde::Serialize)]
/// # pub struct BlogPost;
/// # async fn load_posts(category: &str) -> Result<Vec<BlogPost>, ServerFnError> { unimplemented!() }
/// #[server]
/// pub async fn blog_posts(
///     category: String,
/// ) -> Result<Vec<BlogPost>, ServerFnError> {
///     let posts = load_posts(&category).await?;
///     // maybe do some other work
///     Ok(posts)
/// }
/// ```
///
/// ## Named Arguments
///
/// You can any combination of the following named arguments:
/// - `name`: sets the identifier for the server function’s type, which is a struct created
///    to hold the arguments (defaults to the function identifier in PascalCase)
/// - `prefix`: a prefix at which the server function handler will be mounted (defaults to `/api`)
/// - `endpoint`: specifies the exact path at which the server function handler will be mounted,
///   relative to the prefix (defaults to the function name followed by unique hash)
/// - `input`: the encoding for the arguments (defaults to `PostUrl`)
/// - `output`: the encoding for the response (defaults to `Json`)
/// - `client`: a custom `Client` implementation that will be used for this server fn
/// - `encoding`: (legacy, may be deprecated in future) specifies the encoding, which may be one
///   of the following (not case sensitive)
///     - `"Url"`: `POST` request with URL-encoded arguments and JSON response
///     - `"GetUrl"`: `GET` request with URL-encoded arguments and JSON response
///     - `"Cbor"`: `POST` request with CBOR-encoded arguments and response
///     - `"GetCbor"`: `GET` request with URL-encoded arguments and CBOR response
/// - `req` and `res` specify the HTTP request and response types to be used on the server (these
///   should usually only be necessary if you are integrating with a server other than Actix/Axum)
/// ```rust,ignore
/// #[server(
///   name = SomeStructName,
///   prefix = "/my_api",
///   endpoint = "my_fn",
///   input = Cbor,
///   output = Json
/// )]
/// pub async fn my_wacky_server_fn(input: Vec<String>) -> Result<usize, ServerFnError> {
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
/// pub async fn my_wacky_server_fn(input: Vec<String>) -> Result<usize, ServerFnError> {
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
/// pub async fn my_wacky_server_fn(input: Vec<String>) -> Result<String, ServerFnError> {
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
/// pub async fn my_wacky_server_fn(input: Vec<String>) -> Result<String, ServerFnError> {
///     let FromContext(pool): FromContext<DatabasePool> = extract().await?;
///     Ok(format!("The server read {:?} from the shared context", pool))
/// }
/// ```
#[proc_macro_attribute]
pub fn server(args: proc_macro::TokenStream, s: TokenStream) -> TokenStream {
    match server_macro_impl(
        args.into(),
        s.into(),
        Some(syn::parse_quote!(server_fn)),
        "/api",
        None,
        None,
    ) {
        Err(e) => e.to_compile_error().into(),
        Ok(s) => s.to_token_stream().into(),
    }
}

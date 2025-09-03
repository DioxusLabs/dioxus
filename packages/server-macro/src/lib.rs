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

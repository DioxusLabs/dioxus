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
///   todo!()
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

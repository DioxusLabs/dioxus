use proc_macro::TokenStream;
use quote::ToTokens;
use server_fn_macro::*;

/// Declares that a function is a [server function](dioxus_fullstack). This means that
/// its body will only run on the server, i.e., when the `ssr` feature is enabled.
///
/// If you call a server function from the client (i.e., when the `csr` or `hydrate` features
/// are enabled), it will instead make a network request to the server.
///
/// You can specify one, two, or three arguments to the server function:
/// 1. **Required**: A type name that will be used to identify and register the server function
///   (e.g., `MyServerFn`).
/// 2. *Optional*: A URL prefix at which the function will be mounted when it’s registered
///   (e.g., `"/api"`). Defaults to `"/"`.
/// 3. *Optional*: either `"Cbor"` (specifying that it should use the binary `cbor` format for
///   serialization), `"Url"` (specifying that it should be use a URL-encoded form-data string).
///   Defaults to `"Url"`. If you want to use this server function
///   using Get instead of Post methods, the encoding must be `"GetCbor"` or `"GetJson"`.
///
/// The server function itself can take any number of arguments, each of which should be serializable
/// and deserializable with `serde`. Optionally, its first argument can be a [DioxusServerContext](https::/docs.rs/dioxus-fullstack/latest/dixous_server/prelude/struct.DioxusServerContext.html),
/// which will be injected *on the server side.* This can be used to inject the raw HTTP request or other
/// server-side context into the server function.
///
/// ```ignore
/// # use dioxus_fullstack::prelude::*; use serde::{Serialize, Deserialize};
/// # #[derive(Serialize, Deserialize)]
/// # pub struct Post { }
/// #[server(ReadPosts, "/api")]
/// pub async fn read_posts(how_many: u8, query: String) -> Result<Vec<Post>, ServerFnError> {
///   // do some work on the server to access the database
///   todo!()
/// }
/// ```
///
/// Note the following:
/// - **Server functions must be `async`.** Even if the work being done inside the function body
///   can run synchronously on the server, from the client’s perspective it involves an asynchronous
///   function call.
/// - **Server functions must return `Result<T, ServerFnError>`.** Even if the work being done
///   inside the function body can’t fail, the processes of serialization/deserialization and the
///   network call are fallible.
/// - **Return types must implement [`Serialize`](https://docs.rs/serde/latest/serde/trait.Serialize.html).**
///   This should be fairly obvious: we have to serialize arguments to send them to the server, and we
///   need to deserialize the result to return it to the client.
/// - **Arguments must be implement [`Serialize`](https://docs.rs/serde/latest/serde/trait.Serialize.html)
///   and [`DeserializeOwned`](https://docs.rs/serde/latest/serde/de/trait.DeserializeOwned.html).**
///   They are serialized as an `application/x-www-form-urlencoded`
///   form data using [`serde_urlencoded`](https://docs.rs/serde_urlencoded/latest/serde_urlencoded/) or as `application/cbor`
///   using [`cbor`](https://docs.rs/cbor/latest/cbor/).
/// - **The [DioxusServerContext](https::/docs.rs/dioxus-fullstack/latest/dixous_server/prelude/struct.DioxusServerContext.html) comes from the server.** Optionally, the first argument of a server function
///   can be a [DioxusServerContext](https::/docs.rs/dioxus-fullstack/latest/dixous_server/prelude/struct.DioxusServerContext.html). This scope can be used to inject dependencies like the HTTP request
///   or response or other server-only dependencies, but it does *not* have access to reactive state that exists in the client.
#[proc_macro_attribute]
pub fn server(args: proc_macro::TokenStream, s: TokenStream) -> TokenStream {
    let context = ServerContext {
        ty: syn::parse_quote!(DioxusServerContext),
        path: syn::parse_quote!(::dioxus_fullstack::prelude::DioxusServerContext),
    };
    match server_macro_impl(
        args.into(),
        s.into(),
        syn::parse_quote!(::dioxus_fullstack::prelude::ServerFnTraitObj),
        Some(context),
        Some(syn::parse_quote!(::dioxus_fullstack::prelude::server_fn)),
    ) {
        Err(e) => e.to_compile_error().into(),
        Ok(s) => s.to_token_stream().into(),
    }
}

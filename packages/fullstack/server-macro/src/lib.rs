use proc_macro::TokenStream;
use quote::{ToTokens, __private::TokenStream as TokenStream2};
use server_fn_macro::*;
use syn::{parse::Parse, spanned::Spanned, ItemFn};

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
    // before we pass this off to the server function macro, we apply extractors and middleware
    let mut function: syn::ItemFn = match syn::parse(s).map_err(|e| e.to_compile_error()) {
        Ok(f) => f,
        Err(e) => return e.into(),
    };

    // find all arguments with the #[extract] attribute
    let mut extractors: Vec<Extractor> = vec![];
    function.sig.inputs = function
        .sig
        .inputs
        .into_iter()
        .filter(|arg| {
            if let Ok(extractor) = syn::parse2(arg.clone().into_token_stream()) {
                extractors.push(extractor);
                false
            } else {
                true
            }
        })
        .collect();

    let ItemFn {
        attrs,
        vis,
        sig,
        block,
    } = function;
    let mapped_body = quote::quote! {
        #(#attrs)*
        #vis #sig {
            #(#extractors)*
            #block
        }
    };

    match server_macro_impl(
        args.into(),
        mapped_body,
        syn::parse_quote!(::dioxus_fullstack::prelude::ServerFnTraitObj),
        None,
        Some(syn::parse_quote!(::dioxus_fullstack::prelude::server_fn)),
    ) {
        Err(e) => e.to_compile_error().into(),
        Ok(s) => s.to_token_stream().into(),
    }
}

struct Extractor {
    pat: syn::PatType,
}

impl ToTokens for Extractor {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let pat = &self.pat;
        tokens.extend(quote::quote! {
            let #pat = ::dioxus_fullstack::prelude::extract_server_context().await?;
        });
    }
}

impl Parse for Extractor {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let arg: syn::FnArg = input.parse()?;
        match arg {
            syn::FnArg::Typed(mut pat_type) => {
                let mut contains_extract = false;
                pat_type.attrs.retain(|attr| {
                    let is_extract = attr.path().is_ident("extract");
                    if is_extract {
                        contains_extract = true;
                    }
                    !is_extract
                });
                if !contains_extract {
                    return Err(syn::Error::new(
                        pat_type.span(),
                        "expected an argument with the #[extract] attribute",
                    ));
                }
                Ok(Extractor { pat: pat_type })
            }
            _ => Err(syn::Error::new(arg.span(), "expected a typed argument")),
        }
    }
}

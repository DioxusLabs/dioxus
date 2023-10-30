// TODO: Create README, uncomment this: #![doc = include_str!("../README.md")]
#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]

use convert_case::{Case, Converter};
use proc_macro::TokenStream;
use proc_macro2::Literal;
use quote::{ToTokens, __private::TokenStream as TokenStream2};
use server_fn_macro::*;
use syn::{
    parse::{Parse, ParseStream},
    Ident, ItemFn, Token,
};

/// Declares that a function is a [server function](https://dioxuslabs.com/learn/0.4/reference/fullstack/server_functions). This means that
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

    // extract all #[middleware] attributes
    let mut middlewares: Vec<Middleware> = vec![];
    function.attrs.retain(|attr| {
        if attr.meta.path().is_ident("middleware") {
            if let Ok(middleware) = attr.parse_args() {
                middlewares.push(middleware);
                false
            } else {
                true
            }
        } else {
            true
        }
    });

    let ItemFn {
        attrs,
        vis,
        sig,
        block,
    } = function;
    let mapped_body = quote::quote! {
        #(#attrs)*
        #vis #sig {
            #block
        }
    };

    let server_fn_path: syn::Path = syn::parse_quote!(::dioxus_fullstack::prelude::server_fn);
    let trait_obj_wrapper: syn::Type =
        syn::parse_quote!(::dioxus_fullstack::prelude::ServerFnTraitObj);
    let mut args: ServerFnArgs = match syn::parse(args) {
        Ok(args) => args,
        Err(e) => return e.to_compile_error().into(),
    };
    if args.struct_name.is_none() {
        let upper_cammel_case_name = Converter::new()
            .from_case(Case::Snake)
            .to_case(Case::UpperCamel)
            .convert(sig.ident.to_string());
        args.struct_name = Some(Ident::new(&upper_cammel_case_name, sig.ident.span()));
    }
    let struct_name = args.struct_name.as_ref().unwrap();
    match server_macro_impl(
        quote::quote!(#args),
        mapped_body,
        trait_obj_wrapper,
        None,
        Some(server_fn_path.clone()),
    ) {
        Err(e) => e.to_compile_error().into(),
        Ok(tokens) => quote::quote! {
            #tokens
            #[cfg(feature = "ssr")]
            #server_fn_path::inventory::submit! {
                ::dioxus_fullstack::prelude::ServerFnMiddleware {
                    prefix: #struct_name::PREFIX,
                    url: #struct_name::URL,
                    middleware: || vec![
                        #(
                            std::sync::Arc::new(#middlewares),
                        ),*
                    ]
                }
            }
        }
        .to_token_stream()
        .into(),
    }
}

#[derive(Debug)]
struct Middleware {
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

struct ServerFnArgs {
    struct_name: Option<Ident>,
    _comma: Option<Token![,]>,
    prefix: Option<Literal>,
    _comma2: Option<Token![,]>,
    encoding: Option<Literal>,
    _comma3: Option<Token![,]>,
    fn_path: Option<Literal>,
}

impl ToTokens for ServerFnArgs {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let struct_name = self.struct_name.as_ref().map(|s| quote::quote! { #s, });
        let prefix = self.prefix.as_ref().map(|p| quote::quote! { #p, });
        let encoding = self.encoding.as_ref().map(|e| quote::quote! { #e, });
        let fn_path = self.fn_path.as_ref().map(|f| quote::quote! { #f, });
        tokens.extend(quote::quote! {
            #struct_name
            #prefix
            #encoding
            #fn_path
        })
    }
}

impl Parse for ServerFnArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let struct_name = input.parse()?;
        let _comma = input.parse()?;
        let prefix = input.parse()?;
        let _comma2 = input.parse()?;
        let encoding = input.parse()?;
        let _comma3 = input.parse()?;
        let fn_path = input.parse()?;

        Ok(Self {
            struct_name,
            _comma,
            prefix,
            _comma2,
            encoding,
            _comma3,
            fn_path,
        })
    }
}

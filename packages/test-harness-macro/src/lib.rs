use proc_macro::TokenStream;
use quote::quote;
use syn::{Ident, ItemFn, LitStr, Token, parse_macro_input, punctuated::Punctuated};

/// `#[dioxus_test_harness::test]` - register a function as a test case with the
/// `dioxus-test-harness` runtime.
///
/// Supported arguments:
/// - `ignore`
/// - `should_panic`
/// - `timeout = "5s"` (also `ms` and `m` suffixes)
/// - `tags = ["slow", "ui"]`
/// - `platforms = [web, desktop, ios, android, server, native]`
#[proc_macro_attribute]
pub fn test(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args as TestArgs);
    let func = parse_macro_input!(input as ItemFn);

    let name = &func.sig.ident;
    let is_async = func.sig.asyncness.is_some();

    let ignore = args.ignore;
    let should_panic = args.should_panic;
    let timeout_ms = match &args.timeout_ms {
        Some(ms) => quote!(Some(#ms)),
        None => quote!(None),
    };
    let tags = args.tags.iter().map(|tag| quote!(#tag)).collect::<Vec<_>>();
    let platforms = args
        .platforms
        .iter()
        .map(|platform| quote!(#platform))
        .collect::<Vec<_>>();
    let platform_cfg = args.platform_cfg();

    // A non-capturing closure coerces to the `run` fn pointer; the cast keeps
    // `Some(#run)` well-typed through the `let` in `run_init`.
    let run = if is_async {
        quote!((|| ::std::boxed::Box::pin(async move { #name().await }))
            as dioxus_test_harness::TestFn)
    } else {
        quote!((|| ::std::boxed::Box::pin(async move { #name() }))
            as dioxus_test_harness::TestFn)
    };

    // The metadata is always registered; only the test body is platform-gated,
    // so `run` is `None` on targets the test isn't declared for.
    let run_init = match args.platform_predicate() {
        Some(pred) => quote!({
            #[cfg(#pred)]
            let __dx_test_run = Some(#run);
            #[cfg(not(#pred))]
            let __dx_test_run = None;
            __dx_test_run
        }),
        None => quote!(Some(#run)),
    };

    quote! {
        #platform_cfg
        #func

        dioxus_test_harness::inventory::submit! {
            dioxus_test_harness::TestCase {
                name: ::core::concat!(::core::module_path!(), "::", ::core::stringify!(#name)),
                file: ::core::file!(),
                line: ::core::line!(),
                ignore: #ignore,
                should_panic: #should_panic,
                timeout_ms: #timeout_ms,
                tags: &[#(#tags),*],
                platforms: &[#(#platforms),*],
                run: #run_init,
            }
        }
    }
    .into()
}

struct TestArgs {
    ignore: bool,
    should_panic: bool,
    timeout_ms: Option<u64>,
    tags: Vec<String>,
    platforms: Vec<String>,
}

impl TestArgs {
    /// The cfg predicate (`any(..)`) matching the declared platforms.
    fn platform_predicate(&self) -> Option<proc_macro2::TokenStream> {
        if self.platforms.is_empty() {
            return None;
        }

        let cfgs = self
            .platforms
            .iter()
            .map(|platform| match platform.as_str() {
                "web" => quote!(target_arch = "wasm32"),
                "ios" => quote!(target_os = "ios"),
                "android" => quote!(target_os = "android"),
                "desktop" | "server" | "native" => {
                    quote!(not(any(
                        target_arch = "wasm32",
                        target_os = "ios",
                        target_os = "android"
                    )))
                }
                _ => quote!(all()),
            });
        Some(quote!(any(#(#cfgs),*)))
    }

    /// The cfg gate for the test function itself.
    fn platform_cfg(&self) -> proc_macro2::TokenStream {
        self.platform_predicate()
            .map(|pred| quote!(#[cfg(#pred)]))
            .unwrap_or_default()
    }
}

impl syn::parse::Parse for TestArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        const ACCEPTED: &str = "ignore, should_panic, timeout, tags, platforms";

        let mut args = TestArgs {
            ignore: false,
            should_panic: false,
            timeout_ms: None,
            tags: vec![],
            platforms: vec![],
        };

        while !input.is_empty() {
            let key: Ident = input.parse()?;
            match key.to_string().as_str() {
                "ignore" => args.ignore = true,
                "should_panic" => args.should_panic = true,
                "timeout" => {
                    input.parse::<Token![=]>()?;
                    let lit: LitStr = input.parse()?;
                    args.timeout_ms = Some(parse_duration_ms(&lit)?);
                }
                "tags" => {
                    input.parse::<Token![=]>()?;
                    let content;
                    syn::bracketed!(content in input);
                    let lits = Punctuated::<LitStr, Token![,]>::parse_terminated(&content)?;
                    args.tags = lits.iter().map(|lit| lit.value()).collect();
                }
                "platforms" => {
                    input.parse::<Token![=]>()?;
                    let content;
                    syn::bracketed!(content in input);
                    let idents = Punctuated::<Ident, Token![,]>::parse_terminated(&content)?;
                    for ident in idents {
                        let platform = ident.to_string();
                        match platform.as_str() {
                            "web" | "ios" | "android" | "desktop" | "server" | "native" => {
                                args.platforms.push(platform);
                            }
                            _ => {
                                return Err(syn::Error::new(
                                    ident.span(),
                                    format!(
                                        "unknown platform `{platform}` (accepted: web, ios, android, desktop, server, native)"
                                    ),
                                ));
                            }
                        }
                    }
                }
                other => {
                    return Err(syn::Error::new(
                        key.span(),
                        format!("unknown argument `{other}` (accepted: {ACCEPTED})"),
                    ));
                }
            }
            _ = input.parse::<Token![,]>();
        }

        Ok(args)
    }
}

/// Parse `"5s"`, `"250ms"`, `"2m"` into milliseconds.
fn parse_duration_ms(lit: &LitStr) -> syn::Result<u64> {
    let text = lit.value();
    let (digits, scale) = match text
        .strip_suffix("ms")
        .map(|d| (d, 1u64))
        .or_else(|| text.strip_suffix('s').map(|d| (d, 1_000)))
        .or_else(|| text.strip_suffix('m').map(|d| (d, 60_000)))
    {
        Some((digits, scale)) => (digits, scale),
        None => (text.as_str(), 1_000),
    };

    digits
        .trim()
        .parse::<u64>()
        .ok()
        .and_then(|n| n.checked_mul(scale))
        .ok_or_else(|| {
            syn::Error::new(
                lit.span(),
                format!("invalid duration `{text}` (expected e.g. \"5s\", \"250ms\", \"2m\")"),
            )
        })
}

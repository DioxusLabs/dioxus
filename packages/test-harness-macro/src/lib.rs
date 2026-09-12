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
    let platform_cfg = args.platform_cfg();

    let run = if is_async {
        quote!(|| ::std::boxed::Box::pin(async move { #name().await }))
    } else {
        quote!(|| ::std::boxed::Box::pin(async move { #name() }))
    };

    quote! {
        #func

        #platform_cfg
        dioxus_test_harness::inventory::submit! {
            dioxus_test_harness::TestCase {
                name: ::core::concat!(::core::module_path!(), "::", ::core::stringify!(#name)),
                file: ::core::file!(),
                line: ::core::line!(),
                ignore: #ignore,
                should_panic: #should_panic,
                timeout_ms: #timeout_ms,
                tags: &[#(#tags),*],
                run: #run,
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
    /// The cfg gate for the inventory submission, or `None` when `platforms` wasn't given.
    fn platform_cfg(&self) -> proc_macro2::TokenStream {
        if self.platforms.is_empty() {
            return quote!();
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
        quote!(#[cfg(any(#(#cfgs),*))])
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

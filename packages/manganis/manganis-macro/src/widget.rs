//! Widget extension macro for embedding Apple Widget Extension metadata
//!
//! This macro embeds metadata about widget extensions (like Live Activity widgets)
//! that should be compiled and bundled with the app.

use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use syn::parse::{Parse, ParseStream};
use syn::{Expr, LitStr, Token};

use crate::resolve_path;

/// Parser for the widget!() macro
///
/// Syntax: `widget!("/path/to/widget", display_name = "Name", bundle_id_suffix = "suffix", module_name = "ModuleName", deployment_target = "17.0")`
pub struct WidgetParser {
    /// Relative path to the widget Swift package (from crate root)
    pub relative_path: String,
    /// Display name for the widget
    pub display_name: String,
    /// Bundle ID suffix (e.g., "location-widget")
    pub bundle_id_suffix: String,
    /// Deployment target (e.g., "17.0")
    pub deployment_target: String,
    /// Swift module name for ActivityKit type matching (must match main app's plugin module)
    pub module_name: String,
}

impl Parse for WidgetParser {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // Parse the path (can be a string literal or macro like concat!)
        let (path_expr, _) = crate::parse_with_tokens::<Expr>(input)?;
        let span = input.span();

        // Evaluate the path expression
        let path_str = match &path_expr {
            Expr::Lit(lit) => {
                if let syn::Lit::Str(s) = &lit.lit {
                    s.value()
                } else {
                    return Err(syn::Error::new_spanned(
                        &path_expr,
                        "Expected string literal for widget path",
                    ));
                }
            }
            Expr::Macro(m) => {
                // Handle concat! and env! macros
                let mac = &m.mac;
                if mac.path.is_ident("concat") || mac.path.is_ident("env") {
                    // For now, just use the raw tokens - we'll need proper evaluation
                    return Err(syn::Error::new_spanned(
                        &path_expr,
                        "concat!/env! macros in widget paths are not yet supported. Use a string literal.",
                    ));
                } else {
                    return Err(syn::Error::new_spanned(
                        &path_expr,
                        "Unsupported macro in widget path",
                    ));
                }
            }
            _ => {
                return Err(syn::Error::new_spanned(
                    &path_expr,
                    "Expected string literal for widget path",
                ));
            }
        };

        // Validate the path exists (but keep the relative path for storage)
        let _ = resolve_path(&path_str, span).map_err(|e| {
            syn::Error::new(span, format!("Failed to resolve widget path: {}", e))
        })?;

        // Keep the relative path (strip leading slash for concat!)
        let relative_path = path_str.trim_start_matches('/').to_string();

        // Parse comma and options
        input.parse::<Token![,]>()?;

        // Parse key-value options
        let mut display_name = None;
        let mut bundle_id_suffix = None;
        let mut deployment_target = String::from("17.0");
        let mut module_name = None;

        while !input.is_empty() {
            let key: syn::Ident = input.parse()?;
            input.parse::<Token![=]>()?;
            let value: LitStr = input.parse()?;

            match key.to_string().as_str() {
                "display_name" => display_name = Some(value.value()),
                "bundle_id_suffix" => bundle_id_suffix = Some(value.value()),
                "deployment_target" => deployment_target = value.value(),
                "module_name" => module_name = Some(value.value()),
                _ => {
                    return Err(syn::Error::new_spanned(
                        key,
                        format!("Unknown widget option. Expected: display_name, bundle_id_suffix, module_name, deployment_target"),
                    ));
                }
            }

            // Optional trailing comma
            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }

        let display_name = display_name.ok_or_else(|| {
            syn::Error::new(span, "Missing required option: display_name")
        })?;

        let bundle_id_suffix = bundle_id_suffix.ok_or_else(|| {
            syn::Error::new(span, "Missing required option: bundle_id_suffix")
        })?;

        let module_name = module_name.ok_or_else(|| {
            syn::Error::new(span, "Missing required option: module_name (must match main app's Swift plugin module name)")
        })?;

        Ok(WidgetParser {
            relative_path,
            display_name,
            bundle_id_suffix,
            deployment_target,
            module_name,
        })
    }
}

impl WidgetParser {
    /// Generate the token stream for embedding widget metadata
    pub fn generate(&self) -> TokenStream2 {
        let relative_path = &self.relative_path;
        let display_name = &self.display_name;
        let bundle_id_suffix = &self.bundle_id_suffix;
        let deployment_target = &self.deployment_target;
        let module_name = &self.module_name;

        // Generate a unique hash for the link section
        let mut hasher = DefaultHasher::new();
        relative_path.hash(&mut hasher);
        display_name.hash(&mut hasher);
        bundle_id_suffix.hash(&mut hasher);
        module_name.hash(&mut hasher);
        let hash = format!("{:016x}", hasher.finish());

        // Use the SymbolData system with generate_link_section_inner
        let link_section = crate::linker::generate_link_section_inner(
            quote! { __WIDGET_METADATA },
            &hash,
            "__ASSETS__",
            quote! { manganis::macro_helpers::serialize_symbol_data },
            quote! { manganis::macro_helpers::copy_bytes },
            quote! { manganis::macro_helpers::ConstVec<u8, 4096> },
        );

        // Generate as a const item so it can be used at module level
        // Use concat! to build the full path at compile time, keeping the embedded path short
        quote! {
            const _: () = {
                const __WIDGET_METADATA: manganis::SymbolData =
                    manganis::SymbolData::AppleWidgetExtension(
                        manganis::AppleWidgetExtensionMetadata::new(
                            concat!(env!("CARGO_MANIFEST_DIR"), "/", #relative_path),
                            #display_name,
                            #bundle_id_suffix,
                            #deployment_target,
                            #module_name,
                        )
                    );

                #link_section
            };
        }
    }
}

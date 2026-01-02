use proc_macro::TokenStream;
use std::hash::{DefaultHasher, Hash, Hasher};
use syn::parse::Parse;
use syn::parse_macro_input;

use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens};

/// Generate a linker section for embedding permission data in the binary
///
/// This function creates a static array containing the serialized permission data
/// wrapped in SymbolData::Permission and exports it with the __ASSETS__ prefix
/// for unified symbol collection with assets.
pub fn generate_link_section(permission: impl ToTokens, permission_hash: &str) -> TokenStream2 {
    dx_macro_helpers::linker::generate_link_section(
        permission,
        permission_hash,
        "__ASSETS__",
        quote! { permissions::macro_helpers::serialize_permission },
        quote! { permissions::macro_helpers::copy_bytes },
        quote! { permissions::macro_helpers::ConstVec<u8, 4096> },
        true, // permissions needs #[used] attribute
    )
}

/// Parser for the `static_permission!()` macro syntax (and `permission!()` alias)
///
/// This parser accepts any expression that evaluates to a `Permission`:
/// - Builder pattern: `PermissionBuilder::location(...).with_description(...).build()`
/// - Direct construction: `Permission::new(PermissionKind::Camera, "...")`
pub struct PermissionParser {
    /// The permission expression (either builder or direct)
    expr: TokenStream2,
}

impl Parse for PermissionParser {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        // Parse the entire expression as a token stream
        // This accepts either:
        // - PermissionBuilder::location(...).with_description(...).build()
        // - Permission::new(PermissionKind::Camera, "...")
        let expr = input.parse::<TokenStream2>()?;
        Ok(Self { expr })
    }
}

impl ToTokens for PermissionParser {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        // Generate a hash for unique symbol naming
        // Hash the expression tokens to create a unique identifier
        let mut hash = DefaultHasher::new();
        self.expr.to_string().hash(&mut hash);
        let permission_hash = format!("{:016x}", hash.finish());

        let expr = &self.expr;
        let link_section =
            crate::linker::generate_link_section(quote!(__PERMISSION), &permission_hash);

        tokens.extend(quote! {
            {
                // Create the permission instance from the expression
                const __PERMISSION: permissions::Permission = #expr;

                #link_section

                // Return the permission
                __PERMISSION
            }
        });
    }
}

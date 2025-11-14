use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens};
use std::hash::{DefaultHasher, Hash, Hasher};
use syn::parse::Parse;

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

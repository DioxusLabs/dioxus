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

        // Check if this is a Custom permission by examining the expression
        // Custom permissions are built via PermissionBuilder::custom() or contain PermissionKind::Custom
        let expr_str = self.expr.to_string();
        let is_custom = expr_str.contains("custom()")
            || expr_str.contains("Custom {")
            || expr_str.contains("PermissionKind::Custom");

        let expr = &self.expr;

        if is_custom {
            // For Custom permissions, skip linker section generation due to buffer size limitations
            // Custom permissions can exceed the 4096 byte buffer limit when serialized
            tokens.extend(quote! {
                {
                    // Create the permission instance directly for Custom permissions
                    // Skip linker section generation due to buffer size limitations
                    const __PERMISSION: permissions_core::Permission = #expr;
                    __PERMISSION
                }
            });
        } else {
            // For regular permissions, use the normal serialization approach with linker sections
            let link_section =
                crate::linker::generate_link_section(quote!(__PERMISSION), &permission_hash);

            tokens.extend(quote! {
                {
                    // Create the permission instance from the expression
                    const __PERMISSION: permissions_core::Permission = #expr;

                    #link_section

                    // Return the permission
                    __PERMISSION
                }
            });
        }
    }
}

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
    generate_link_section_inner(
        permission,
        permission_hash,
        "__ASSETS__",
        quote! { permissions::macro_helpers::serialize_permission },
        quote! { permissions::macro_helpers::copy_bytes },
        quote! { permissions::macro_helpers::ConstVec<u8, 4096> },
        true, // permissions needs #[used] attribute
    )
}

/// Generate a linker section for embedding serialized data in the binary
///
/// This function creates a static array containing serialized data and exports it
/// with a unique symbol name that can be found by build tools. The exported symbol
/// follows the pattern `{prefix}{hash}` and can be extracted from the binary after linking.
///
/// # Parameters
///
/// - `item`: The item to serialize (must implement `ToTokens`)
/// - `hash`: Unique hash string for the export name
/// - `prefix`: Export prefix (e.g., `"__MY_CRATE__"`)
/// - `serialize_fn`: Path to the serialization function (as a `TokenStream`)
/// - `copy_bytes_fn`: Path to the `copy_bytes` function (as a `TokenStream`)
/// - `buffer_type`: The type of the buffer (e.g., `ConstVec<u8>` or `ConstVec<u8, 4096>`)
/// - `add_used_attribute`: Whether to add the `#[used]` attribute (some crates need it)
///
/// # Example
///
/// ```ignore
/// generate_link_section(
///     my_data,
///     "abc123",
///     "__MY_CRATE__",
///     quote! { my_crate::macro_helpers::serialize_data },
///     quote! { my_crate::macro_helpers::copy_bytes },
///     quote! { my_crate::macro_helpers::const_serialize::ConstVec<u8> },
///     false,
/// )
/// ```
pub fn generate_link_section_inner(
    item: impl ToTokens,
    hash: &str,
    prefix: &str,
    serialize_fn: TokenStream2,
    copy_bytes_fn: TokenStream2,
    buffer_type: TokenStream2,
    add_used_attribute: bool,
) -> TokenStream2 {
    let position = proc_macro2::Span::call_site();
    let export_name = syn::LitStr::new(&format!("{}{}", prefix, hash), position);

    let used_attr = if add_used_attribute {
        quote! { #[used] }
    } else {
        quote! {}
    };

    quote! {
        // First serialize the item into a constant sized buffer
        const __BUFFER: #buffer_type = #serialize_fn(&#item);
        // Then pull out the byte slice
        const __BYTES: &[u8] = __BUFFER.as_ref();
        // And the length of the byte slice
        const __LEN: usize = __BYTES.len();

        // Now that we have the size of the item, copy the bytes into a static array
        #used_attr
        #[unsafe(export_name = #export_name)]
        static __LINK_SECTION: [u8; __LEN] = #copy_bytes_fn(__BYTES);
    }
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

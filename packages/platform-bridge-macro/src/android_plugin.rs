use quote::{quote, ToTokens};
use std::hash::{DefaultHasher, Hash, Hasher};
use syn::{
    parse::{Parse, ParseStream},
    ExprArray, ExprLit, Lit, Token,
};

/// Parser for the `android_plugin!()` macro syntax
pub struct AndroidPluginParser {
    /// Java package name (e.g., "dioxus.mobile.geolocation")
    package_name: String,
    /// Plugin identifier (e.g., "geolocation")
    plugin_name: String,
    /// Relative filenames that will be resolved to full paths
    files: Vec<String>,
}

impl Parse for AndroidPluginParser {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut package_name = None;
        let mut plugin_name = None;
        let mut files = None;

        while !input.is_empty() {
            // Parse field name
            let field = input.parse::<syn::Ident>()?;

            match field.to_string().as_str() {
                "package" => {
                    let _equals = input.parse::<Token![=]>()?;
                    let package_lit = input.parse::<syn::LitStr>()?;
                    package_name = Some(package_lit.value());

                    // Check for comma
                    let _ = input.parse::<Option<Token![,]>>()?;
                }
                "plugin" => {
                    let _equals = input.parse::<Token![=]>()?;
                    let plugin_lit = input.parse::<syn::LitStr>()?;
                    plugin_name = Some(plugin_lit.value());

                    // Check for comma
                    let _ = input.parse::<Option<Token![,]>>()?;
                }
                "files" => {
                    let _equals = input.parse::<Token![=]>()?;
                    let array = input.parse::<ExprArray>()?;
                    let mut file_vec = Vec::new();

                    for element in array.elems {
                        if let syn::Expr::Lit(ExprLit {
                            lit: Lit::Str(lit_str),
                            ..
                        }) = element
                        {
                            file_vec.push(lit_str.value());
                        } else {
                            return Err(syn::Error::new(
                                proc_macro2::Span::call_site(),
                                "Expected string literal in files array",
                            ));
                        }
                    }
                    files = Some(file_vec);

                    // Check for comma
                    let _ = input.parse::<Option<Token![,]>>()?;
                }
                _ => {
                    return Err(syn::Error::new(
                        field.span(),
                        "Unknown field, expected 'package', 'plugin', or 'files'",
                    ));
                }
            }
        }

        let package_name = package_name
            .ok_or_else(|| syn::Error::new(input.span(), "Missing required field 'package'"))?;

        let plugin_name = plugin_name
            .ok_or_else(|| syn::Error::new(input.span(), "Missing required field 'plugin'"))?;

        let files =
            files.ok_or_else(|| syn::Error::new(input.span(), "Missing required field 'files'"))?;

        Ok(Self {
            package_name,
            plugin_name,
            files,
        })
    }
}

impl ToTokens for AndroidPluginParser {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let package_name = &self.package_name;
        let plugin_name = &self.plugin_name;

        // Generate a hash for unique symbol naming
        let mut hash = DefaultHasher::new();
        self.package_name.hash(&mut hash);
        self.plugin_name.hash(&mut hash);
        self.files.hash(&mut hash);
        let plugin_hash = format!("{:016x}", hash.finish());

        // Get file literals for code generation (validation happens in generated code)
        let (_, file_path_lits) = self.resolve_file_paths();

        // Generate the export name as a string literal
        let export_name_lit = syn::LitStr::new(
            &format!("__JAVA_SOURCE__{}", plugin_hash),
            proc_macro2::Span::call_site(),
        );

        // Generate the link section - we'll serialize the metadata inline
        // Build file paths dynamically by concatenating
        // Now accepts full relative paths without hard-coding directory structure
        let file_path_consts: Vec<_> = file_path_lits
            .iter()
            .enumerate()
            .map(|(i, file_lit)| {
                let const_name =
                    syn::Ident::new(&format!("__FILE_PATH{}", i), proc_macro2::Span::call_site());
                quote! {
                    const #const_name: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/", #file_lit);
                }
            })
            .collect();

        let file_path_refs: Vec<_> = file_path_lits
            .iter()
            .enumerate()
            .map(|(i, _)| {
                let const_name =
                    syn::Ident::new(&format!("__FILE_PATH{}", i), proc_macro2::Span::call_site());
                quote! { #const_name }
            })
            .collect();

        let link_section = quote! {
            // Build absolute file paths at compile time
            #(#file_path_consts)*

            const __FILE_PATHS: &[&str] = &[#(#file_path_refs),*];

            // Create the Java source metadata with full paths
            const __JAVA_META: dioxus_platform_bridge::android::JavaSourceMetadata =
                dioxus_platform_bridge::android::JavaSourceMetadata::new(
                    #package_name,
                    #plugin_name,
                    __FILE_PATHS,
                );

            // Serialize the metadata
            const __BUFFER: const_serialize::ConstVec<u8, 4096> = {
                const EMPTY: const_serialize::ConstVec<u8, 4096> = const_serialize::ConstVec::new_with_max_size();
                const_serialize::serialize_const(&__JAVA_META, EMPTY)
            };
            const __BYTES: &[u8] = __BUFFER.as_ref();
            const __LEN: usize = __BYTES.len();

            // Embed in linker section
            #[link_section = "__DATA,__java_source"]
            #[used]
            #[unsafe(export_name = #export_name_lit)]
            static __LINK_SECTION: [u8; __LEN] = dioxus_platform_bridge::android::macro_helpers::copy_bytes(__BYTES);

            // Create a module-level static reference to the linker section to ensure
            // it's preserved even if the macro invocation appears unused.
            // This provides additional protection against optimization.
            #[used]
            static __REFERENCE_TO_LINK_SECTION: &'static [u8] = &__LINK_SECTION;
        };

        tokens.extend(link_section);
    }
}

impl AndroidPluginParser {
    /// Resolve file paths to absolute paths at compile time
    ///
    /// Searches for Java files in common locations relative to the crate calling the macro
    fn resolve_file_paths(&self) -> (Vec<String>, Vec<proc_macro2::Literal>) {
        // Use the file position span to get the calling crate's directory
        // Note: We can't get CARGO_MANIFEST_DIR from the calling crate in proc-macro,
        // so we need to generate code that resolves it at compile time
        let mut absolute_paths = Vec::new();
        let mut path_literals = Vec::new();

        for file in &self.files {
            // Generate code that will resolve the path at compile time in the calling crate
            let file_str = file.clone();
            path_literals.push(proc_macro2::Literal::string(file_str.as_str()));
            absolute_paths.push(String::new()); // Will be filled by generated code
        }

        (absolute_paths, path_literals)
    }
}

use dx_macro_helpers::linker;
use quote::{quote, ToTokens};
use std::{
    collections::BTreeSet,
    hash::{DefaultHasher, Hash, Hasher},
};
use syn::{parse::Parse, parse::ParseStream, Token};

pub struct AndroidPluginParser {
    plugin_name: String,
    artifact: ArtifactDeclaration,
    dependencies: BTreeSet<String>,
}

enum ArtifactDeclaration {
    Path(String),
    Env(String),
}

impl Parse for AndroidPluginParser {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut plugin_name = None;
        let mut artifact = None;
        let mut dependencies: BTreeSet<String> = BTreeSet::new();

        while !input.is_empty() {
            let field = input.parse::<syn::Ident>()?;
            match field.to_string().as_str() {
                "plugin" => {
                    let _equals = input.parse::<Token![=]>()?;
                    let plugin_lit = input.parse::<syn::LitStr>()?;
                    plugin_name = Some(plugin_lit.value());
                    let _ = input.parse::<Option<Token![,]>>()?;
                }
                "deps" => {
                    let _equals = input.parse::<Token![=]>()?;
                    let content;
                    syn::bracketed!(content in input);

                    while !content.is_empty() {
                        let value = content.parse::<syn::LitStr>()?;
                        dependencies.insert(value.value());
                        let _ = content.parse::<Option<Token![,]>>()?;
                    }

                    let _ = input.parse::<Option<Token![,]>>()?;
                }
                "aar" => {
                    let _equals = input.parse::<Token![=]>()?;
                    let content;
                    syn::braced!(content in input);

                    let mut path = None;
                    let mut env = None;

                    while !content.is_empty() {
                        let key = content.parse::<syn::Ident>()?;
                        let key_str = key.to_string();
                        let _eq = content.parse::<Token![=]>()?;
                        let value = content.parse::<syn::LitStr>()?;
                        match key_str.as_str() {
                            "path" => path = Some(value.value()),
                            "env" => env = Some(value.value()),
                            _ => {
                                return Err(syn::Error::new(
                                    key.span(),
                                    "Unknown field in aar declaration (expected 'path' or 'env')",
                                ))
                            }
                        }
                        let _ = content.parse::<Option<Token![,]>>()?;
                    }

                    artifact = Some(match (path, env) {
                        (Some(p), None) => ArtifactDeclaration::Path(p),
                        (None, Some(e)) => ArtifactDeclaration::Env(e),
                        (Some(_), Some(_)) => {
                            return Err(syn::Error::new(
                                field.span(),
                                "Specify only one of 'path' or 'env' in aar block",
                            ))
                        }
                        (None, None) => {
                            return Err(syn::Error::new(
                                field.span(),
                                "Missing 'path' or 'env' in aar block",
                            ))
                        }
                    });

                    let _ = input.parse::<Option<Token![,]>>()?;
                }
                _ => {
                    return Err(syn::Error::new(
                        field.span(),
                        "Unknown field, expected 'plugin' or 'aar'",
                    ));
                }
            }
        }

        Ok(Self {
            plugin_name: plugin_name
                .ok_or_else(|| syn::Error::new(input.span(), "Missing required field 'plugin'"))?,
            artifact: artifact
                .ok_or_else(|| syn::Error::new(input.span(), "Missing required field 'aar'"))?,
            dependencies,
        })
    }
}

impl ToTokens for AndroidPluginParser {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let plugin_name = &self.plugin_name;

        let mut hash = DefaultHasher::new();
        self.plugin_name.hash(&mut hash);
        match &self.artifact {
            ArtifactDeclaration::Path(path) => path.hash(&mut hash),
            ArtifactDeclaration::Env(env) => env.hash(&mut hash),
        }
        let plugin_hash = format!("{:016x}", hash.finish());

        let artifact_expr = match &self.artifact {
            ArtifactDeclaration::Path(path) => {
                let path_lit = syn::LitStr::new(path, proc_macro2::Span::call_site());
                quote! { concat!(env!("CARGO_MANIFEST_DIR"), "/", #path_lit) }
            }
            ArtifactDeclaration::Env(var) => {
                let env_lit = syn::LitStr::new(var, proc_macro2::Span::call_site());
                quote! { env!(#env_lit) }
            }
        };
        let deps_joined = self
            .dependencies
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        let deps_lit = syn::LitStr::new(&deps_joined, proc_macro2::Span::call_site());

        let metadata_expr = quote! {
            dioxus_platform_bridge::android::AndroidArtifactMetadata::new(
                #plugin_name,
                #artifact_expr,
                #deps_lit,
            )
        };

        let link_section = linker::generate_link_section(
            metadata_expr,
            &plugin_hash,
            "__ASSETS__",
            quote! { dioxus_platform_bridge::android::metadata::serialize_android_metadata },
            quote! { dioxus_platform_bridge::android::macro_helpers::copy_bytes },
            quote! { dioxus_platform_bridge::android::metadata::AndroidMetadataBuffer },
            true,
        );

        tokens.extend(link_section);
    }
}

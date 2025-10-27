use quote::{quote, ToTokens};
use syn::{
    parse::{Parse, ParseStream},
    ExprArray, ExprLit, Lit, Token,
};

/// Parser for Darwin (iOS/macOS) plugin macro syntax
pub struct DarwinPluginParser {
    /// Plugin identifier (e.g., "geolocation")
    plugin_name: String,
    /// List of framework names (e.g., ["CoreLocation", "Foundation"])
    frameworks: Vec<String>,
}

impl Parse for DarwinPluginParser {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut plugin_name = None;
        let mut frameworks = None;

        while !input.is_empty() {
            // Parse field name
            let field = input.parse::<syn::Ident>()?;

            match field.to_string().as_str() {
                "plugin" => {
                    let _equals = input.parse::<Token![=]>()?;
                    let plugin_lit = input.parse::<syn::LitStr>()?;
                    plugin_name = Some(plugin_lit.value());

                    // Check for comma
                    let _ = input.parse::<Option<Token![,]>>()?;
                }
                "frameworks" => {
                    let _equals = input.parse::<Token![=]>()?;
                    let array = input.parse::<ExprArray>()?;
                    let mut framework_vec = Vec::new();

                    for element in array.elems {
                        if let syn::Expr::Lit(ExprLit {
                            lit: Lit::Str(lit_str),
                            ..
                        }) = element
                        {
                            framework_vec.push(lit_str.value());
                        } else {
                            return Err(syn::Error::new(
                                proc_macro2::Span::call_site(),
                                "Expected string literal in frameworks array",
                            ));
                        }
                    }
                    frameworks = Some(framework_vec);

                    // Check for comma
                    let _ = input.parse::<Option<Token![,]>>()?;
                }
                _ => {
                    return Err(syn::Error::new(
                        field.span(),
                        "Unknown field, expected 'plugin' or 'frameworks'",
                    ));
                }
            }
        }

        let plugin_name = plugin_name
            .ok_or_else(|| syn::Error::new(input.span(), "Missing required field 'plugin'"))?;

        let frameworks = frameworks
            .ok_or_else(|| syn::Error::new(input.span(), "Missing required field 'frameworks'"))?;

        if frameworks.is_empty() {
            return Err(syn::Error::new(
                input.span(),
                "frameworks array cannot be empty",
            ));
        }

        Ok(Self {
            plugin_name,
            frameworks,
        })
    }
}

impl ToTokens for DarwinPluginParser {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let plugin_name = &self.plugin_name;

        // Generate string literals for each framework
        let framework_literals: Vec<proc_macro2::TokenStream> = self
            .frameworks
            .iter()
            .map(|f| {
                let lit = syn::LitStr::new(f, proc_macro2::Span::call_site());
                quote! { #lit }
            })
            .collect();

        // Generate the export name using __DARWIN_FRAMEWORK__ prefix
        let export_name = format!("__DARWIN_FRAMEWORK__{}", plugin_name);

        // Generate the linker section attributes
        // Use __DATA,__darwin_framework for unified extraction
        let link_section = quote! {
            #[link_section = "__DATA,__darwin_framework"]
            #[used]
            #[export_name = #export_name]
            static DARWIN_FRAMEWORK_METADATA: (&str, &[&str]) = (#plugin_name, &[#(#framework_literals),*]);
        };

        tokens.extend(link_section);
    }
}

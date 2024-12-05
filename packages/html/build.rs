use proc_macro2::TokenStream;
use quote::{quote, format_ident, TokenStreamExt};
use serde::Deserialize;
use std::fs;
use std::path::Path;
use std::collections::HashMap;
use convert_case::{Case, Casing};


#[derive(Debug, Deserialize)]
struct Elements(HashMap<String, Element>);

#[derive(Debug, Deserialize)]
struct Element {
    #[serde(default)]
    namespace: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    attributes: HashMap<String, AttributeDefinition>,
}

#[derive(Debug, Deserialize)]
struct AttributeDefinition {
    #[serde(rename = "type")]
    attr_type: Option<String>,
    name: Option<String>,
    #[serde(default)]
    volatile: bool,
}


fn impl_extension_attributes(name: &str, element: &Element) -> TokenStream {
    let mut tokens = TokenStream::new();

    let camel_name = name.to_case(Case::UpperCamel);
    let rust_name = safe_ident(name);

    let extension_name = format_ident!("{}Extension", &camel_name);


    let impls = element.attributes.iter().map(|(ident, _)| {
        let rust_attr_name = safe_ident(ident);

        quote! {
            fn #rust_attr_name(self, value: impl IntoAttributeValue) -> Self {
                let d = #rust_name::#rust_attr_name;
                self.push_attribute(d.0, d.1, value, d.2)
            }
        }
    });
    tokens.append_all(quote! {
        pub trait #extension_name: HasAttributes + Sized {
            #(#impls)*
        }
    });
    tokens
}


fn main() {
    // Read the TOML file
    let toml_content = fs::read_to_string("src/elements.toml")
        .expect("Failed to read elements.toml");
    
    let elements: Elements = toml::from_str(&toml_content)
        .expect("Failed to parse TOML");


    let file = generate_file(&elements);

    let out_dir = std::env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("elements.rs");
    fs::write(dest_path, prettyplease::unparse(&file))
        .expect("Failed to write generated code");
}

fn generate_file(elements: &Elements) -> syn::File{
    // Generate the Rust code
    let module_tokens = elements.0
        .iter()
        .map(|(k, v)| generate_element_module(&k, &v));

    let extensions = generate_extensions(elements);
    let completions = generate_completions(elements);


    let final_tokens = quote! {
        use dioxus_core::prelude::IntoAttributeValue;
        use dioxus_core::HasAttributes;
        pub type AttributeDescription = (&'static str, Option<&'static str>, bool);
        
        #(#module_tokens)*

        #extensions
        #completions
    };


    syn::parse2(final_tokens).unwrap()
}

fn safe_ident(name: &str) -> syn::Ident {
    match syn::parse_str(name) {
        Ok(x) => x,
        Err(_) => syn::Ident::new_raw(name, proc_macro2::Span::call_site())
    }
}

fn generate_element_module(name: &str, element: &Element) -> TokenStream {
    let element_name = 
        element.name.clone()
        .unwrap_or_else(|| name.to_string());

    let namespace = match element.namespace.as_ref() {
        Some(x) => quote!{Some(#x)},
        None => quote!{None},
    };

    let rust_name = safe_ident(name);

    let attr_consts = element.attributes.iter().map(|(ident, def)| {
        let attr_name = def.name.as_ref().unwrap_or_else(|| &ident);
        let rust_attr_name = safe_ident(ident);
        let volatile = def.volatile;
        let attr_type = match &def.attr_type {
            Some(x) => quote!{Some(#x)},
            None => quote!{None},
        };

        let comment = format!(r#" ```
let {rust_attr_name} = "value";
rsx! {{
   // Attributes need to be under the element they modify
   {rust_name} {{
       // Attributes are followed by a colon and then the value of the attribute
       {rust_attr_name}: "value"
   }}
   {rust_name} {{
       // Or you can use the shorthand syntax if you have a variable in scope that has the same name as the attribute
       {rust_attr_name},
   }}
}}
```
"#);
        let comment_lines = comment.lines();

        quote! { 
            #(
                #[doc = #comment_lines]
            )*
            
            pub const #rust_attr_name: super::AttributeDescription = (#attr_name, #attr_type, #volatile);
        }
    });

    let tag_name = element_name.to_string();

    quote! {
        pub mod #rust_name {
            pub const TAG_NAME: &'static str = #tag_name;
            pub const NAME_SPACE: Option<&'static str> = #namespace;

            #(
                #attr_consts
            )*
        }
    }.into()
}

fn generate_element_documentation(name: &str) -> TokenStream {
    let rust_name = safe_ident(name);

    let comment = format!(r#" ```rust, no_run
# use dioxus::prelude::*;
# let attributes = vec![];
# fn ChildComponent() -> Element {{ unimplemented!() }}
# let raw_expression: Element = rsx! {{}};
rsx! {{
    // Elements are followed by braces that surround any attributes and children for that element
    {rust_name} {{
        // Add any attributes first
        class: "my-class",
        "custom-attribute-name": "value",
        // Then add any attributes you are spreading into this element
        ..attributes,
        // Then add any children elements, components, text nodes, or raw expressions
        div {{}}
        ChildComponent {{}}
        "child text"
        {{raw_expression}}
    }}
}};
```"#);

    let comment_lines = comment.lines();
    
    quote! {
        #(
            #[doc = #comment_lines]
        )*
            #rust_name {}
    }.into()
}

fn generate_completions(elements: &Elements) -> TokenStream {
    let docs = elements.0
        .iter()
        .map(|(name, _)| generate_element_documentation(name));


    quote !{
        #[doc(hidden)]
        pub mod completions {
            /// This helper tells rust analyzer that it should autocomplete the element name with braces.
            #[allow(non_camel_case_types)]
            pub enum CompleteWithBraces {
                #(
                    #docs
                ),*
            }
        }
    }.into()
}

fn generate_extensions(elements: &Elements) -> TokenStream {
    let extensions = elements.0
        .iter()
        .map(|(name, def)| impl_extension_attributes(name, def));

    quote!{
        pub(crate) mod extensions {
            use super::*;
            #(
                #extensions
            )*
        }
    }.into()
}

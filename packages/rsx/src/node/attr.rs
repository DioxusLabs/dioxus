
/// A list of fields in the form of
///
/// `name: value`
///
///
pub struct Fields {}

// #[derive(PartialEq, Eq, Clone, Debug, Hash)]
// pub enum AttributeType {
//     /// An attribute that is known
//     Named(ElementAttrNamed),

//     /// An attribute that's being spread in via the `..` syntax
//     Spread(Expr),
// }

// impl ToTokens for AttributeType {
//     fn to_tokens(&self, tokens: &mut TokenStream2) {
//         todo!()
//         // match self {
//         //     AttributeType::Named(named) => named.to_tokens(tokens),
//         //     AttributeType::Spread(expr) => tokens.append_all(quote! { #expr }),
//         // }
//     }
// }

// impl AttributeType {
//     pub fn start(&self) -> Span {
//         match self {
//             AttributeType::Named(n) => n.attr.start(),
//             AttributeType::Spread(e) => e.span(),
//         }
//     }

//     pub fn matches_attr_name(&self, other: &Self) -> bool {
//         match (self, other) {
//             (Self::Named(a), Self::Named(b)) => a.attr.name == b.attr.name,
//             _ => false,
//         }
//     }

//     pub(crate) fn try_combine(&self, other: &Self) -> Option<Self> {
//         match (self, other) {
//             (Self::Named(a), Self::Named(b)) => a.try_combine(b).map(Self::Named),
//             _ => None,
//         }
//     }

//     pub(crate) fn ifmt(&self) -> Option<&IfmtInput> {
//         match self {
//             AttributeType::Named(named) => match &named.attr.value {
//                 ElementAttrValue::AttrLiteral(lit) => Some(lit),
//                 _ => None,
//             },
//             AttributeType::Spread(_) => None,
//         }
//     }

//     pub(crate) fn merge_quote(vec: &[&Self]) -> TokenStream2 {
//         // split into spread and single attributes
//         let mut spread = vec![];
//         let mut single = vec![];
//         for attr in vec.iter() {
//             match attr {
//                 AttributeType::Named(named) => single.push(named),
//                 AttributeType::Spread(expr) => spread.push(expr),
//             }
//         }

//         // If all of them are single attributes, create a static slice
//         if spread.is_empty() {
//             quote! {
//                 Box::new([
//                     #(#single),*
//                 ])
//             }
//         } else {
//             // Otherwise start with the single attributes and append the spread attributes
//             quote! {
//                 {
//                     let mut __attributes = vec![
//                         #(#single),*
//                     ];
//                     #(
//                         let mut __spread = #spread;
//                         __attributes.append(&mut __spread);
//                     )*
//                     __attributes.into_boxed_slice()
//                 }
//             }
//         }
//     }

//     pub fn as_static_str_literal(&self) -> Option<(&ElementAttrName, &IfmtInput)> {
//         match self {
//             AttributeType::Named(ElementAttrNamed {
//                 attr:
//                     ElementAttr {
//                         value: ElementAttrValue::AttrLiteral(value),
//                         name,
//                     },
//                 ..
//             }) if value.is_static() => Some((name, value)),
//             _ => None,
//         }
//     }

//     pub fn is_static_str_literal(&self) -> bool {
//         self.as_static_str_literal().is_some()
//     }
// }

#[derive(Clone, Debug)]
pub struct ElementAttrNamed {
    pub el_name: ElementName,
    pub attr: ElementAttr,
}

impl Hash for ElementAttrNamed {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.attr.name.hash(state);
    }
}

impl PartialEq for ElementAttrNamed {
    fn eq(&self, other: &Self) -> bool {
        self.attr == other.attr
    }
}

impl Eq for ElementAttrNamed {}

impl ElementAttrNamed {
    pub(crate) fn try_combine(&self, other: &Self) -> Option<Self> {
        if self.el_name == other.el_name && self.attr.name == other.attr.name {
            if let Some(separator) = self.attr.name.multi_attribute_separator() {
                return Some(ElementAttrNamed {
                    el_name: self.el_name.clone(),
                    attr: ElementAttr {
                        name: self.attr.name.clone(),
                        value: self.attr.value.combine(separator, &other.attr.value),
                    },
                });
            }
        }
        None
    }
}

impl ToTokens for ElementAttrNamed {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let ElementAttrNamed { el_name, attr } = self;

        let ns = |name: &ElementAttrName| match (el_name, name) {
            (ElementName::Ident(i), ElementAttrName::BuiltIn(_)) => {
                quote! { dioxus_elements::#i::#name.1 }
            }
            _ => quote! { None },
        };
        let volitile = |name: &ElementAttrName| match (el_name, name) {
            (ElementName::Ident(i), ElementAttrName::BuiltIn(_)) => {
                quote! { dioxus_elements::#i::#name.2 }
            }
            _ => quote! { false },
        };
        let attribute = |name: &ElementAttrName| match name {
            ElementAttrName::BuiltIn(name) => match el_name {
                ElementName::Ident(_) => quote! { #el_name::#name.0 },
                ElementName::Custom(_) => {
                    let as_string = name.to_string();
                    quote!(#as_string)
                }
            },
            ElementAttrName::Custom(s) => quote! { #s },
        };

        let attribute = {
            let value = &self.attr.value;
            let is_shorthand_event = match &attr.value {
                ElementAttrValue::Shorthand(s) => s.to_string().starts_with("on"),
                _ => false,
            };

            match &attr.value {
                ElementAttrValue::AttrLiteral(_)
                | ElementAttrValue::AttrExpr(_)
                | ElementAttrValue::Shorthand(_)
                | ElementAttrValue::AttrOptionalExpr { .. }
                    if !is_shorthand_event =>
                {
                    let name = &self.attr.name;
                    let ns = ns(name);
                    let volitile = volitile(name);
                    let attribute = attribute(name);
                    let value = quote! { #value };

                    quote! {
                        dioxus_core::Attribute::new(
                            #attribute,
                            #value,
                            #ns,
                            #volitile
                        )
                    }
                }
                ElementAttrValue::EventTokens(tokens) => match &self.attr.name {
                    ElementAttrName::BuiltIn(name) => {
                        quote! {
                            dioxus_elements::events::#name(#tokens)
                        }
                    }
                    ElementAttrName::Custom(_) => unreachable!("Handled elsewhere in the macro"),
                },
                _ => {
                    quote! { dioxus_elements::events::#value(#value) }
                }
            }
        };

        tokens.append_all(attribute);
    }
}

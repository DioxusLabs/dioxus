
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


        let mut attributes: Vec<AttributeType> = vec![];
        let mut children: Vec<BodyNode> = vec![];
        let mut key = None;

        // parse fields with commas
        // break when we don't get this pattern anymore
        // start parsing bodynodes
        // "def": 456,
        // abc: 123,
        loop {
            if content.peek(Token![..]) {
                content.parse::<Token![..]>()?;
                let expr = content.parse::<Expr>()?;
                let span = expr.span();
                attributes.push(attribute::AttributeType::Spread(expr));

                if content.is_empty() {
                    break;
                }

                if content.parse::<Token![,]>().is_err() {
                    missing_trailing_comma!(span);
                }
                continue;
            }

            // Parse the raw literal fields
            // "def": 456,
            if content.peek(LitStr) && content.peek2(Token![:]) && !content.peek3(Token![:]) {
                let name = content.parse::<LitStr>()?;
                let ident = name.clone();

                content.parse::<Token![:]>()?;

                let value = content.parse::<ElementAttrValue>()?;
                attributes.push(attribute::AttributeType::Named(ElementAttrNamed {
                    el_name: el_name.clone(),
                    attr: ElementAttr {
                        name: ElementAttrName::Custom(name),
                        value,
                    },
                }));

                if content.is_empty() {
                    break;
                }

                if content.parse::<Token![,]>().is_err() {
                    missing_trailing_comma!(ident.span());
                }
                continue;
            }

            // Parse
            // abc: 123,
            if content.peek(Ident) && content.peek2(Token![:]) && !content.peek3(Token![:]) {
                let name = content.parse::<Ident>()?;

                let name_str = name.to_string();
                content.parse::<Token![:]>()?;

                // The span of the content to be parsed,
                // for example the `hi` part of `class: "hi"`.
                let span = content.span();

                if name_str.starts_with("on") {
                    // check for any duplicate event listeners
                    if attributes.iter().any(|f| {
                        if let AttributeType::Named(ElementAttrNamed {
                            attr:
                                ElementAttr {
                                    name: ElementAttrName::BuiltIn(n),
                                    value: ElementAttrValue::EventTokens(_),
                                },
                            ..
                        }) = f
                        {
                            n == &name_str
                        } else {
                            false
                        }
                    }) {
                        return Err(syn::Error::new(
                            name.span(),
                            format!("Duplicate event listener `{}`", name),
                        ));
                    }
                    attributes.push(attribute::AttributeType::Named(ElementAttrNamed {
                        el_name: el_name.clone(),
                        attr: ElementAttr {
                            name: ElementAttrName::BuiltIn(name),
                            value: ElementAttrValue::EventTokens(content.parse()?),
                        },
                    }));
                } else if name_str == "key" {
                    let _key: IfmtInput = content.parse()?;

                    if _key.is_static() {
                        invalid_key!(_key);
                    }

                    key = Some(_key);
                } else {
                    let value = content.parse::<ElementAttrValue>()?;
                    attributes.push(attribute::AttributeType::Named(ElementAttrNamed {
                        el_name: el_name.clone(),
                        attr: ElementAttr {
                            name: ElementAttrName::BuiltIn(name),
                            value,
                        },
                    }));
                }

                if content.is_empty() {
                    break;
                }

                if content.parse::<Token![,]>().is_err() {
                    missing_trailing_comma!(span);
                }
                continue;
            }

            // Parse shorthand fields
            if content.peek(Ident)
                && !content.peek2(Brace)
                && !content.peek2(Token![:])
                && !content.peek2(Token![-])
            {
                let name = content.parse::<Ident>()?;
                let name_ = name.clone();

                // If the shorthand field is children, these are actually children!
                if name == "children" {
                    return Err(syn::Error::new(
                        name.span(),
                        r#"Shorthand element children are not supported.
To pass children into elements, wrap them in curly braces.
Like so:
    div {{ {{children}} }}

"#,
                    ));
                };

                let value = ElementAttrValue::Shorthand(name.clone());
                attributes.push(attribute::AttributeType::Named(ElementAttrNamed {
                    el_name: el_name.clone(),
                    attr: ElementAttr {
                        name: ElementAttrName::BuiltIn(name),
                        value,
                    },
                }));

                if content.is_empty() {
                    break;
                }

                if content.parse::<Token![,]>().is_err() {
                    missing_trailing_comma!(name_.span());
                }
                continue;
            }

            break;
        }

        while !content.is_empty() {
            if (content.peek(LitStr) && content.peek2(Token![:])) && !content.peek3(Token![:]) {
                attr_after_element!(content.span());
            }

            if (content.peek(Ident) && content.peek2(Token![:])) && !content.peek3(Token![:]) {
                attr_after_element!(content.span());
            }

            children.push(content.parse::<BodyNode>()?);
            // consume comma if it exists
            // we don't actually care if there *are* commas after elements/text
            if content.peek(Token![,]) {
                let _ = content.parse::<Token![,]>();
            }
        }

use super::*;

#[derive(PartialEq, Eq, Clone, Debug, Hash)]
pub struct TextNode {
    pub input: IfmtInput,
    pub dyn_idx: CallerLocation,
}

impl TextNode {
    pub fn is_static(&self) -> bool {
        self.input.is_static()
    }

    pub fn to_template_node(&self) -> TemplateNode {
        match self.is_static() {
            true => {
                let text = self.input.source.as_ref().unwrap();
                let text = intern(text.value().as_str());
                TemplateNode::Text { text }
            }
            false => TemplateNode::DynamicText {
                id: self.dyn_idx.get(),
            },
        }
    }
}

impl Parse for TextNode {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Self {
            input: input.parse()?,
            dyn_idx: CallerLocation::default(),
        })
    }
}

impl ToTokens for TextNode {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let txt = &self.input;

        if txt.is_static() {
            tokens.append_all(quote! {
                dioxus_core::DynamicNode::Text(dioxus_core::VText::new(#txt.to_string()))
            })
        } else {
            // If the text is dynamic, we actually create a signal of the formatted segments
            // Crazy, right?
            let segments = txt.as_htotreloaded();
            let idx = txt.hr_idx.idx.get() + 1;

            let rendered_segments = txt.segments.iter().filter_map(|s| match s {
                Segment::Literal(lit) => None,
                Segment::Formatted(fmt) => {
                    // just render as a format_args! call
                    Some(quote! {
                        format!("{}", #fmt)
                    })
                }
            });

            tokens.append_all(quote! {
                dioxus_core::DynamicNode::Text(dioxus_core::VText::new({
                    // Create a signal of the formatted segments
                    // htotreloading will find this via its location and then update the signal
                    static __SIGNAL: GlobalSignal<FmtedSegments> = GlobalSignal::with_key(|| #segments, {
                        concat!(
                            file!(),
                            ":",
                            line!(),
                            ":",
                            column!(),
                            ":",
                            #idx
                        )
                    });

                    // render the signal and subscribe the component to its changes
                    __SIGNAL.with(|s| s.render_with(
                        vec![ #(#rendered_segments),* ]
                    ))
                }))
            })
        }
    }
}

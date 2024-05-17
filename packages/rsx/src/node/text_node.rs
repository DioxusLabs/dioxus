use super::*;

#[derive(PartialEq, Eq, Clone, Debug, Hash)]
pub struct TextNode {
    pub input: IfmtInput,
}

impl TextNode {
    pub fn is_static(&self) -> bool {
        self.input.is_static()
    }
}

impl Parse for TextNode {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Self {
            input: input.parse()?,
        })
    }
}

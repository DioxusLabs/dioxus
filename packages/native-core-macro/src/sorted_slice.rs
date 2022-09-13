extern crate proc_macro;

use std::collections::BTreeMap;

use syn::{
    self, bracketed,
    parse::{Parse, ParseStream, Result},
    LitStr, Token,
};

pub struct StrSlice {
    pub map: BTreeMap<String, LitStr>,
}

impl Parse for StrSlice {
    fn parse(input: ParseStream) -> Result<Self> {
        let content;
        bracketed!(content in input);
        let mut map = BTreeMap::new();
        while let Ok(s) = content.parse::<LitStr>() {
            map.insert(s.value(), s);
            #[allow(unused_must_use)]
            {
                content.parse::<Token![,]>();
            }
        }
        Ok(StrSlice { map })
    }
}

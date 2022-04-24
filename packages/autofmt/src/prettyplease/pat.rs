use super::algorithm::Printer;
use super::iter::IterDelimited;
use super::INDENT;
use proc_macro2::TokenStream;
use syn::{
    FieldPat, Pat, PatBox, PatIdent, PatLit, PatMacro, PatOr, PatPath, PatRange, PatReference,
    PatRest, PatSlice, PatStruct, PatTuple, PatTupleStruct, PatType, PatWild, RangeLimits,
};

impl Printer {
    pub fn pat(&mut self, pat: &Pat) {
        match pat {
            Pat::Box(pat) => self.pat_box(pat),
            Pat::Ident(pat) => self.pat_ident(pat),
            Pat::Lit(pat) => self.pat_lit(pat),
            Pat::Macro(pat) => self.pat_macro(pat),
            Pat::Or(pat) => self.pat_or(pat),
            Pat::Path(pat) => self.pat_path(pat),
            Pat::Range(pat) => self.pat_range(pat),
            Pat::Reference(pat) => self.pat_reference(pat),
            Pat::Rest(pat) => self.pat_rest(pat),
            Pat::Slice(pat) => self.pat_slice(pat),
            Pat::Struct(pat) => self.pat_struct(pat),
            Pat::Tuple(pat) => self.pat_tuple(pat),
            Pat::TupleStruct(pat) => self.pat_tuple_struct(pat),
            Pat::Type(pat) => self.pat_type(pat),
            Pat::Verbatim(pat) => self.pat_verbatim(pat),
            Pat::Wild(pat) => self.pat_wild(pat),
            #[cfg_attr(all(test, exhaustive), deny(non_exhaustive_omitted_patterns))]
            _ => unimplemented!("unknown Pat"),
        }
    }

    fn pat_box(&mut self, pat: &PatBox) {
        self.outer_attrs(&pat.attrs);
        self.word("box ");
        self.pat(&pat.pat);
    }

    fn pat_ident(&mut self, pat: &PatIdent) {
        self.outer_attrs(&pat.attrs);
        if pat.by_ref.is_some() {
            self.word("ref ");
        }
        if pat.mutability.is_some() {
            self.word("mut ");
        }
        self.ident(&pat.ident);
        if let Some((_at_token, subpat)) = &pat.subpat {
            self.word(" @ ");
            self.pat(subpat);
        }
    }

    fn pat_lit(&mut self, pat: &PatLit) {
        self.outer_attrs(&pat.attrs);
        self.expr(&pat.expr);
    }

    fn pat_macro(&mut self, pat: &PatMacro) {
        self.outer_attrs(&pat.attrs);
        self.mac(&pat.mac, None);
    }

    fn pat_or(&mut self, pat: &PatOr) {
        self.outer_attrs(&pat.attrs);
        let mut consistent_break = false;
        for case in &pat.cases {
            match case {
                Pat::Lit(_) | Pat::Wild(_) => {}
                _ => {
                    consistent_break = true;
                    break;
                }
            }
        }
        if consistent_break {
            self.cbox(0);
        } else {
            self.ibox(0);
        }
        for case in pat.cases.iter().delimited() {
            if !case.is_first {
                self.space();
                self.word("| ");
            }
            self.pat(&case);
        }
        self.end();
    }

    fn pat_path(&mut self, pat: &PatPath) {
        self.outer_attrs(&pat.attrs);
        self.qpath(&pat.qself, &pat.path);
    }

    fn pat_range(&mut self, pat: &PatRange) {
        self.outer_attrs(&pat.attrs);
        self.expr(&pat.lo);
        match &pat.limits {
            RangeLimits::HalfOpen(_) => self.word(".."),
            RangeLimits::Closed(_) => self.word("..="),
        }
        self.expr(&pat.hi);
    }

    fn pat_reference(&mut self, pat: &PatReference) {
        self.outer_attrs(&pat.attrs);
        self.word("&");
        if pat.mutability.is_some() {
            self.word("mut ");
        }
        self.pat(&pat.pat);
    }

    fn pat_rest(&mut self, pat: &PatRest) {
        self.outer_attrs(&pat.attrs);
        self.word("..");
    }

    fn pat_slice(&mut self, pat: &PatSlice) {
        self.outer_attrs(&pat.attrs);
        self.word("[");
        for elem in pat.elems.iter().delimited() {
            self.pat(&elem);
            self.trailing_comma(elem.is_last);
        }
        self.word("]");
    }

    fn pat_struct(&mut self, pat: &PatStruct) {
        self.outer_attrs(&pat.attrs);
        self.cbox(INDENT);
        self.path(&pat.path);
        self.word(" {");
        self.space_if_nonempty();
        for field in pat.fields.iter().delimited() {
            self.field_pat(&field);
            self.trailing_comma_or_space(field.is_last && pat.dot2_token.is_none());
        }
        if pat.dot2_token.is_some() {
            self.word("..");
            self.space();
        }
        self.offset(-INDENT);
        self.end();
        self.word("}");
    }

    fn pat_tuple(&mut self, pat: &PatTuple) {
        self.outer_attrs(&pat.attrs);
        self.word("(");
        self.cbox(INDENT);
        self.zerobreak();
        for elem in pat.elems.iter().delimited() {
            self.pat(&elem);
            if pat.elems.len() == 1 {
                if pat.elems.trailing_punct() {
                    self.word(",");
                }
                self.zerobreak();
            } else {
                self.trailing_comma(elem.is_last);
            }
        }
        self.offset(-INDENT);
        self.end();
        self.word(")");
    }

    fn pat_tuple_struct(&mut self, pat: &PatTupleStruct) {
        self.outer_attrs(&pat.attrs);
        self.path(&pat.path);
        self.word("(");
        self.cbox(INDENT);
        self.zerobreak();
        for elem in pat.pat.elems.iter().delimited() {
            self.pat(&elem);
            self.trailing_comma(elem.is_last);
        }
        self.offset(-INDENT);
        self.end();
        self.word(")");
    }

    pub fn pat_type(&mut self, pat: &PatType) {
        self.outer_attrs(&pat.attrs);
        self.pat(&pat.pat);
        self.word(": ");
        self.ty(&pat.ty);
    }

    fn pat_verbatim(&mut self, pat: &TokenStream) {
        unimplemented!("Pat::Verbatim `{}`", pat);
    }

    fn pat_wild(&mut self, pat: &PatWild) {
        self.outer_attrs(&pat.attrs);
        self.word("_");
    }

    fn field_pat(&mut self, field_pat: &FieldPat) {
        self.outer_attrs(&field_pat.attrs);
        if field_pat.colon_token.is_some() {
            self.member(&field_pat.member);
            self.word(": ");
        }
        self.pat(&field_pat.pat);
    }
}

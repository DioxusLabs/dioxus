use super::algorithm::Printer;
use super::iter::IterDelimited;
use super::INDENT;
use syn::{
    BoundLifetimes, ConstParam, GenericParam, Generics, LifetimeDef, PredicateEq,
    PredicateLifetime, PredicateType, TraitBound, TraitBoundModifier, TypeParam, TypeParamBound,
    WhereClause, WherePredicate,
};

impl Printer {
    pub fn generics(&mut self, generics: &Generics) {
        if generics.params.is_empty() {
            return;
        }

        self.word("<");
        self.cbox(0);
        self.zerobreak();

        // Print lifetimes before types and consts, regardless of their
        // order in self.params.
        //
        // TODO: ordering rules for const parameters vs type parameters have
        // not been settled yet. https://github.com/rust-lang/rust/issues/44580
        for param in generics.params.iter().delimited() {
            if let GenericParam::Lifetime(_) = *param {
                self.generic_param(&param);
                self.trailing_comma(param.is_last);
            }
        }
        for param in generics.params.iter().delimited() {
            match *param {
                GenericParam::Type(_) | GenericParam::Const(_) => {
                    self.generic_param(&param);
                    self.trailing_comma(param.is_last);
                }
                GenericParam::Lifetime(_) => {}
            }
        }

        self.offset(-INDENT);
        self.end();
        self.word(">");
    }

    fn generic_param(&mut self, generic_param: &GenericParam) {
        match generic_param {
            GenericParam::Type(type_param) => self.type_param(type_param),
            GenericParam::Lifetime(lifetime_def) => self.lifetime_def(lifetime_def),
            GenericParam::Const(const_param) => self.const_param(const_param),
        }
    }

    pub fn bound_lifetimes(&mut self, bound_lifetimes: &BoundLifetimes) {
        self.word("for<");
        for lifetime_def in bound_lifetimes.lifetimes.iter().delimited() {
            self.lifetime_def(&lifetime_def);
            if !lifetime_def.is_last {
                self.word(", ");
            }
        }
        self.word("> ");
    }

    fn lifetime_def(&mut self, lifetime_def: &LifetimeDef) {
        self.outer_attrs(&lifetime_def.attrs);
        self.lifetime(&lifetime_def.lifetime);
        for lifetime in lifetime_def.bounds.iter().delimited() {
            if lifetime.is_first {
                self.word(": ");
            } else {
                self.word(" + ");
            }
            self.lifetime(&lifetime);
        }
    }

    fn type_param(&mut self, type_param: &TypeParam) {
        self.outer_attrs(&type_param.attrs);
        self.ident(&type_param.ident);
        self.ibox(INDENT);
        for type_param_bound in type_param.bounds.iter().delimited() {
            if type_param_bound.is_first {
                self.word(": ");
            } else {
                self.space();
                self.word("+ ");
            }
            self.type_param_bound(&type_param_bound);
        }
        if let Some(default) = &type_param.default {
            self.space();
            self.word("= ");
            self.ty(default);
        }
        self.end();
    }

    pub fn type_param_bound(&mut self, type_param_bound: &TypeParamBound) {
        match type_param_bound {
            TypeParamBound::Trait(trait_bound) => self.trait_bound(trait_bound),
            TypeParamBound::Lifetime(lifetime) => self.lifetime(lifetime),
        }
    }

    fn trait_bound(&mut self, trait_bound: &TraitBound) {
        if trait_bound.paren_token.is_some() {
            self.word("(");
        }
        let skip = match trait_bound.path.segments.first() {
            Some(segment) if segment.ident == "const" => {
                self.word("~const ");
                1
            }
            _ => 0,
        };
        self.trait_bound_modifier(&trait_bound.modifier);
        if let Some(bound_lifetimes) = &trait_bound.lifetimes {
            self.bound_lifetimes(bound_lifetimes);
        }
        for segment in trait_bound.path.segments.iter().skip(skip).delimited() {
            if !segment.is_first || trait_bound.path.leading_colon.is_some() {
                self.word("::");
            }
            self.path_segment(&segment);
        }
        if trait_bound.paren_token.is_some() {
            self.word(")");
        }
    }

    fn trait_bound_modifier(&mut self, trait_bound_modifier: &TraitBoundModifier) {
        match trait_bound_modifier {
            TraitBoundModifier::None => {}
            TraitBoundModifier::Maybe(_question_mark) => self.word("?"),
        }
    }

    fn const_param(&mut self, const_param: &ConstParam) {
        self.outer_attrs(&const_param.attrs);
        self.word("const ");
        self.ident(&const_param.ident);
        self.word(": ");
        self.ty(&const_param.ty);
        if let Some(default) = &const_param.default {
            self.word(" = ");
            self.expr(default);
        }
    }

    pub fn where_clause_for_body(&mut self, where_clause: &Option<WhereClause>) {
        let hardbreaks = true;
        let semi = false;
        self.where_clause_impl(where_clause, hardbreaks, semi);
    }

    pub fn where_clause_semi(&mut self, where_clause: &Option<WhereClause>) {
        let hardbreaks = true;
        let semi = true;
        self.where_clause_impl(where_clause, hardbreaks, semi);
    }

    pub fn where_clause_oneline(&mut self, where_clause: &Option<WhereClause>) {
        let hardbreaks = false;
        let semi = false;
        self.where_clause_impl(where_clause, hardbreaks, semi);
    }

    pub fn where_clause_oneline_semi(&mut self, where_clause: &Option<WhereClause>) {
        let hardbreaks = false;
        let semi = true;
        self.where_clause_impl(where_clause, hardbreaks, semi);
    }

    fn where_clause_impl(
        &mut self,
        where_clause: &Option<WhereClause>,
        hardbreaks: bool,
        semi: bool,
    ) {
        let where_clause = match where_clause {
            Some(where_clause) if !where_clause.predicates.is_empty() => where_clause,
            _ => {
                if semi {
                    self.word(";");
                } else {
                    self.nbsp();
                }
                return;
            }
        };
        if hardbreaks {
            self.hardbreak();
            self.offset(-INDENT);
            self.word("where");
            self.hardbreak();
            for predicate in where_clause.predicates.iter().delimited() {
                self.where_predicate(&predicate);
                if predicate.is_last && semi {
                    self.word(";");
                } else {
                    self.word(",");
                    self.hardbreak();
                }
            }
            if !semi {
                self.offset(-INDENT);
            }
        } else {
            self.space();
            self.offset(-INDENT);
            self.word("where");
            self.space();
            for predicate in where_clause.predicates.iter().delimited() {
                self.where_predicate(&predicate);
                if predicate.is_last && semi {
                    self.word(";");
                } else {
                    self.trailing_comma_or_space(predicate.is_last);
                }
            }
            if !semi {
                self.offset(-INDENT);
            }
        }
    }

    fn where_predicate(&mut self, predicate: &WherePredicate) {
        match predicate {
            WherePredicate::Type(predicate) => self.predicate_type(predicate),
            WherePredicate::Lifetime(predicate) => self.predicate_lifetime(predicate),
            WherePredicate::Eq(predicate) => self.predicate_eq(predicate),
        }
    }

    fn predicate_type(&mut self, predicate: &PredicateType) {
        if let Some(bound_lifetimes) = &predicate.lifetimes {
            self.bound_lifetimes(bound_lifetimes);
        }
        self.ty(&predicate.bounded_ty);
        self.word(":");
        if predicate.bounds.len() == 1 {
            self.ibox(0);
        } else {
            self.ibox(INDENT);
        }
        for type_param_bound in predicate.bounds.iter().delimited() {
            if type_param_bound.is_first {
                self.nbsp();
            } else {
                self.space();
                self.word("+ ");
            }
            self.type_param_bound(&type_param_bound);
        }
        self.end();
    }

    fn predicate_lifetime(&mut self, predicate: &PredicateLifetime) {
        self.lifetime(&predicate.lifetime);
        self.word(":");
        self.ibox(INDENT);
        for lifetime in predicate.bounds.iter().delimited() {
            if lifetime.is_first {
                self.nbsp();
            } else {
                self.space();
                self.word("+ ");
            }
            self.lifetime(&lifetime);
        }
        self.end();
    }

    fn predicate_eq(&mut self, predicate: &PredicateEq) {
        self.ty(&predicate.lhs_ty);
        self.word(" = ");
        self.ty(&predicate.rhs_ty);
    }
}

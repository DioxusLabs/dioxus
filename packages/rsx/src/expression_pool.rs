//! A pool of the dynamic expressions in a single template body.
//!
//! Every dynamic expression in a template is pulled out into a `let __rsx_expr_n = ...;` binding
//! and the dynamic node/attribute arrays just reference those bindings. That gives us complete
//! control over the order the user's expressions are *evaluated* in, which is what the borrow
//! checker actually cares about.
//!
//! The order is:
//!
//! 1. [`Tier::Borrowing`] - expressions we can prove only ever borrow: formatted strings whose
//!    interpolations are all place expressions (`{x}`, `{x.y}`, `{*x}`). `format_args!` takes its
//!    arguments by reference, so these cannot move anything, and evaluating them first can only
//!    ever make more code compile.
//! 2. [`Tier::Unknown`] - everything else, depth-first, in the order the user wrote it. We can't
//!    tell whether these move or borrow, so we use the order that makes borrow errors readable.
//! 3. [`Tier::Moving`] - expressions we know always move their captures: event handlers, which take
//!    an `impl FnMut(..) + 'static`. Evaluating them last can only ever make more code compile.
//!
//! The sort is stable, so within a tier expressions keep their depth-first source order.

use quote::{ToTokens, quote};
use syn::{Expr, Ident};

use crate::{
    FormattedSegmentType, HotLiteral, IfmtInput, PartialExpr, Segment,
    innerlude::{Attribute, AttributeValue},
};

/// When an expression is evaluated relative to the other expressions in the same template
#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Debug, Default, Hash)]
pub(crate) enum Tier {
    /// Provably borrow-only: formatted strings made only of place expressions
    Borrowing = 0,
    /// Anything we can't reason about: evaluated in the order the user wrote it
    #[default]
    Unknown = 1,
    /// Provably moving: event handlers
    Moving = 2,
}

/// Is this formatted string guaranteed to only *borrow* the values it interpolates?
///
/// `format_args!` takes all of its arguments by reference, so a formatted string can only move
/// something if one of its segments is an expression that moves - a call, a method call, a macro.
/// Place expressions (`x`, `x.y`, `*x`, `x[0]`) never move anything.
pub(crate) fn ifmt_is_borrow_only(input: &IfmtInput) -> bool {
    input.segments.iter().all(|segment| match segment {
        Segment::Literal(_) => true,
        Segment::Formatted(f) => match &f.segment {
            FormattedSegmentType::Ident(_) => true,
            FormattedSegmentType::Expr(expr) => expr_is_place(expr),
        },
    })
}

/// The tier for a formatted string used as a text node, attribute value or component prop
pub(crate) fn fmt_tier(input: &IfmtInput) -> Tier {
    match ifmt_is_borrow_only(input) {
        true => Tier::Borrowing,
        false => Tier::Unknown,
    }
}

/// A conservative "this expression cannot move anything" check
fn expr_is_place(expr: &Expr) -> bool {
    match expr {
        Expr::Path(path) => path.qself.is_none(),
        Expr::Field(field) => expr_is_place(&field.base),
        Expr::Paren(paren) => expr_is_place(&paren.expr),
        Expr::Group(group) => expr_is_place(&group.expr),
        Expr::Reference(reference) => expr_is_place(&reference.expr),
        Expr::Unary(unary) => matches!(unary.op, syn::UnOp::Deref(_)) && expr_is_place(&unary.expr),
        Expr::Index(index) => expr_is_place(&index.expr) && matches!(&*index.index, Expr::Lit(_)),
        Expr::Lit(_) => true,
        _ => false,
    }
}

impl Attribute {
    /// Is this attribute an event handler? Event handlers take an `impl FnMut(..) + 'static`, so
    /// they always move whatever they capture.
    pub(crate) fn is_event_handler(&self) -> bool {
        matches!(self.value, AttributeValue::EventTokens(_)) || self.name.is_likely_event()
    }

    /// When this attribute's value should be evaluated relative to the rest of the template
    pub(crate) fn tier(&self) -> Tier {
        if self.is_event_handler() {
            return Tier::Moving;
        }

        match &self.value {
            AttributeValue::AttrLiteral(HotLiteral::Fmted(fmted)) => fmt_tier(fmted),
            _ => Tier::Unknown,
        }
    }
}

#[derive(PartialEq, Eq, Clone, Debug, Default, Hash)]
pub(crate) struct ExpressionPool {
    expressions: Vec<(Tier, PartialExpr)>,
}

impl ExpressionPool {
    /// Add an expression to the pool, returning the ident it will be bound to
    pub(crate) fn add(&mut self, tier: Tier, expr: PartialExpr) -> Ident {
        let idx = self.add_indexed(tier, expr);
        binding_ident(idx, self.expressions[idx].1.span())
    }

    /// Add an expression to the pool, returning its index
    pub(crate) fn add_indexed(&mut self, tier: Tier, expr: PartialExpr) -> usize {
        let idx = self.expressions.len();
        self.expressions.push((tier, expr));
        idx
    }

    /// The order the bindings are emitted in - by tier, then by depth-first source order
    fn emission_order(&self) -> Vec<usize> {
        let mut order: Vec<usize> = (0..self.expressions.len()).collect();
        order.sort_by_key(|idx| self.expressions[*idx].0);
        order
    }
}

impl ToTokens for ExpressionPool {
    fn to_tokens(&self, out: &mut proc_macro2::TokenStream) {
        let assignments = self.emission_order().into_iter().map(|idx| {
            let (_, expr) = &self.expressions[idx];
            let ident = binding_ident(idx, expr.span());
            quote! { let #ident = #expr; }
        });
        quote! { #(#assignments)* }.to_tokens(out);
    }
}

pub(crate) fn binding_ident(idx: usize, span: proc_macro2::Span) -> Ident {
    Ident::new(&format!("__rsx_expr_{idx}"), span)
}

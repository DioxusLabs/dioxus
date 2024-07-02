use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{Expr, File, Item, Macro, Stmt, TraitItem};

#[derive(Debug)]
pub enum DiffResult {
    /// Non-rsx was changed in the file
    CodeChanged(NotreloadableReason),

    /// Rsx was changed in the file
    ///
    /// Contains a list of macro invocations that were changed
    RsxChanged { rsx_calls: Vec<ChangedRsx> },
}

#[derive(Debug)]
pub enum NotreloadableReason {
    RootMismatch,

    RsxMismatch,
}

#[derive(Debug)]
pub struct ChangedRsx {
    /// The macro that was changed
    pub old: Macro,

    /// The new tokens for the macro
    pub new: TokenStream,
}

/// Find any rsx calls in the given file and return a list of all the rsx calls that have changed.
pub fn diff_rsx(new: &File, old: &File) -> DiffResult {
    let mut rsx_calls = Vec::new();

    if new.items.len() != old.items.len() {
        tracing::trace!(
            "found not hot reload-able change {:#?} != {:#?}",
            new.items
                .iter()
                .map(|i| i.to_token_stream().to_string())
                .collect::<Vec<_>>(),
            old.items
                .iter()
                .map(|i| i.to_token_stream().to_string())
                .collect::<Vec<_>>()
        );
        return DiffResult::CodeChanged(NotreloadableReason::RootMismatch);
    }

    for (new, old) in new.items.iter().zip(old.items.iter()) {
        if find_rsx_item(new, old, &mut rsx_calls) {
            tracing::trace!(
                "found not hot reload-able change {:#?} != {:#?}",
                new.to_token_stream().to_string(),
                old.to_token_stream().to_string()
            );

            return DiffResult::CodeChanged(NotreloadableReason::RsxMismatch);
        }
    }

    tracing::trace!("found hot reload-able changes {:#?}", rsx_calls);
    DiffResult::RsxChanged { rsx_calls }
}

fn find_rsx_item(new: &Item, old: &Item, rsx_calls: &mut Vec<ChangedRsx>) -> bool {
    match (new, old) {
        (Item::Const(new_item), Item::Const(old_item)) => {
            find_rsx_expr(&new_item.expr, &old_item.expr, rsx_calls)
                || new_item.attrs != old_item.attrs
                || new_item.vis != old_item.vis
                || new_item.const_token != old_item.const_token
                || new_item.ident != old_item.ident
                || new_item.colon_token != old_item.colon_token
                || new_item.ty != old_item.ty
                || new_item.eq_token != old_item.eq_token
                || new_item.semi_token != old_item.semi_token
        }
        (Item::Enum(new_item), Item::Enum(old_item)) => {
            if new_item.variants.len() != old_item.variants.len() {
                return true;
            }
            for (new_variant, old_variant) in new_item.variants.iter().zip(old_item.variants.iter())
            {
                match (&new_variant.discriminant, &old_variant.discriminant) {
                    (Some((new_eq, new_expr)), Some((old_eq, old_expr))) => {
                        if find_rsx_expr(new_expr, old_expr, rsx_calls) || new_eq != old_eq {
                            return true;
                        }
                    }
                    (None, None) => (),
                    _ => return true,
                }
                if new_variant.attrs != old_variant.attrs
                    || new_variant.ident != old_variant.ident
                    || new_variant.fields != old_variant.fields
                {
                    return true;
                }
            }
            new_item.attrs != old_item.attrs
                || new_item.vis != old_item.vis
                || new_item.enum_token != old_item.enum_token
                || new_item.ident != old_item.ident
                || new_item.generics != old_item.generics
                || new_item.brace_token != old_item.brace_token
        }
        (Item::ExternCrate(new_item), Item::ExternCrate(old_item)) => old_item != new_item,
        (Item::Fn(new_item), Item::Fn(old_item)) => {
            find_rsx_block(&new_item.block, &old_item.block, rsx_calls)
                || new_item.attrs != old_item.attrs
                || new_item.vis != old_item.vis
                || new_item.sig != old_item.sig
        }
        (Item::ForeignMod(new_item), Item::ForeignMod(old_item)) => old_item != new_item,
        (Item::Impl(new_item), Item::Impl(old_item)) => {
            if new_item.items.len() != old_item.items.len() {
                return true;
            }
            for (new_item, old_item) in new_item.items.iter().zip(old_item.items.iter()) {
                if match (new_item, old_item) {
                    (syn::ImplItem::Const(new_item), syn::ImplItem::Const(old_item)) => {
                        find_rsx_expr(&new_item.expr, &old_item.expr, rsx_calls)
                    }
                    (syn::ImplItem::Fn(new_item), syn::ImplItem::Fn(old_item)) => {
                        find_rsx_block(&new_item.block, &old_item.block, rsx_calls)
                    }
                    (syn::ImplItem::Type(new_item), syn::ImplItem::Type(old_item)) => {
                        old_item != new_item
                    }
                    (syn::ImplItem::Macro(new_item), syn::ImplItem::Macro(old_item)) => {
                        old_item != new_item
                    }
                    (syn::ImplItem::Verbatim(stream), syn::ImplItem::Verbatim(stream2)) => {
                        stream.to_string() != stream2.to_string()
                    }
                    _ => true,
                } {
                    return true;
                }
            }
            new_item.attrs != old_item.attrs
                || new_item.defaultness != old_item.defaultness
                || new_item.unsafety != old_item.unsafety
                || new_item.impl_token != old_item.impl_token
                || new_item.generics != old_item.generics
                || new_item.trait_ != old_item.trait_
                || new_item.self_ty != old_item.self_ty
                || new_item.brace_token != old_item.brace_token
        }
        (Item::Macro(new_item), Item::Macro(old_item)) => {
            find_rsx_macro(&new_item.mac, &old_item.mac, rsx_calls)
                || new_item.attrs != old_item.attrs
                || new_item.semi_token != old_item.semi_token
                || new_item.ident != old_item.ident
        }
        (Item::Mod(new_item), Item::Mod(old_item)) => {
            match (&new_item.content, &old_item.content) {
                (Some((_, new_items)), Some((_, old_items))) => {
                    if new_items.len() != old_items.len() {
                        return true;
                    }
                    for (new_item, old_item) in new_items.iter().zip(old_items.iter()) {
                        if find_rsx_item(new_item, old_item, rsx_calls) {
                            return true;
                        }
                    }
                    new_item.attrs != old_item.attrs
                        || new_item.vis != old_item.vis
                        || new_item.mod_token != old_item.mod_token
                        || new_item.ident != old_item.ident
                        || new_item.semi != old_item.semi
                }
                (None, None) => {
                    new_item.attrs != old_item.attrs
                        || new_item.vis != old_item.vis
                        || new_item.mod_token != old_item.mod_token
                        || new_item.ident != old_item.ident
                        || new_item.semi != old_item.semi
                }
                _ => true,
            }
        }
        (Item::Static(new_item), Item::Static(old_item)) => {
            find_rsx_expr(&new_item.expr, &old_item.expr, rsx_calls)
                || new_item.attrs != old_item.attrs
                || new_item.vis != old_item.vis
                || new_item.static_token != old_item.static_token
                || new_item.mutability != old_item.mutability
                || new_item.ident != old_item.ident
                || new_item.colon_token != old_item.colon_token
                || new_item.ty != old_item.ty
                || new_item.eq_token != old_item.eq_token
                || new_item.semi_token != old_item.semi_token
        }
        (Item::Struct(new_item), Item::Struct(old_item)) => old_item != new_item,
        (Item::Trait(new_item), Item::Trait(old_item)) => {
            find_rsx_trait(new_item, old_item, rsx_calls)
        }
        (Item::TraitAlias(new_item), Item::TraitAlias(old_item)) => old_item != new_item,
        (Item::Type(new_item), Item::Type(old_item)) => old_item != new_item,
        (Item::Union(new_item), Item::Union(old_item)) => old_item != new_item,
        (Item::Use(new_item), Item::Use(old_item)) => old_item != new_item,
        (Item::Verbatim(_), Item::Verbatim(_)) => false,

        _ => true,
    }
}

fn find_rsx_trait(
    new_item: &syn::ItemTrait,
    old_item: &syn::ItemTrait,
    rsx_calls: &mut Vec<ChangedRsx>,
) -> bool {
    if new_item.items.len() != old_item.items.len() {
        return true;
    }
    for (new_item, old_item) in new_item.items.iter().zip(old_item.items.iter()) {
        if match (new_item, old_item) {
            (TraitItem::Const(new_item), TraitItem::Const(old_item)) => {
                if let (Some((_, new_expr)), Some((_, old_expr))) =
                    (&new_item.default, &old_item.default)
                {
                    find_rsx_expr(new_expr, old_expr, rsx_calls)
                } else {
                    true
                }
            }
            (TraitItem::Fn(new_item), TraitItem::Fn(old_item)) => {
                match (&new_item.default, &old_item.default) {
                    (Some(new_block), Some(old_block)) => {
                        find_rsx_block(new_block, old_block, rsx_calls)
                    }
                    (None, None) => false,
                    _ => true,
                }
            }
            (TraitItem::Type(new_item), TraitItem::Type(old_item)) => old_item != new_item,
            (TraitItem::Macro(new_item), TraitItem::Macro(old_item)) => old_item != new_item,
            (TraitItem::Verbatim(stream), TraitItem::Verbatim(stream2)) => {
                stream.to_string() != stream2.to_string()
            }
            _ => true,
        } {
            return true;
        }
    }

    new_item.attrs != old_item.attrs
        || new_item.vis != old_item.vis
        || new_item.unsafety != old_item.unsafety
        || new_item.auto_token != old_item.auto_token
        || new_item.ident != old_item.ident
        || new_item.generics != old_item.generics
        || new_item.colon_token != old_item.colon_token
        || new_item.supertraits != old_item.supertraits
        || new_item.brace_token != old_item.brace_token
}

fn find_rsx_block(
    new_block: &syn::Block,
    old_block: &syn::Block,
    rsx_calls: &mut Vec<ChangedRsx>,
) -> bool {
    if new_block.stmts.len() != old_block.stmts.len() {
        return true;
    }

    for (new_stmt, old_stmt) in new_block.stmts.iter().zip(old_block.stmts.iter()) {
        if find_rsx_stmt(new_stmt, old_stmt, rsx_calls) {
            return true;
        }
    }

    new_block.brace_token != old_block.brace_token
}

fn find_rsx_stmt(new_stmt: &Stmt, old_stmt: &Stmt, rsx_calls: &mut Vec<ChangedRsx>) -> bool {
    match (new_stmt, old_stmt) {
        (Stmt::Local(new_local), Stmt::Local(old_local)) => {
            (match (&new_local.init, &old_local.init) {
                (Some(new_local), Some(old_local)) => {
                    find_rsx_expr(&new_local.expr, &old_local.expr, rsx_calls)
                        || new_local.diverge != old_local.diverge
                }
                (None, None) => false,
                _ => true,
            } || new_local.attrs != old_local.attrs
                || new_local.let_token != old_local.let_token
                || new_local.pat != old_local.pat
                || new_local.semi_token != old_local.semi_token)
        }
        (Stmt::Item(new_item), Stmt::Item(old_item)) => {
            find_rsx_item(new_item, old_item, rsx_calls)
        }
        (Stmt::Expr(new_expr, _), Stmt::Expr(old_expr, _)) => {
            find_rsx_expr(new_expr, old_expr, rsx_calls)
        }
        (Stmt::Macro(new_macro), Stmt::Macro(old_macro)) => {
            find_rsx_macro(&new_macro.mac, &old_macro.mac, rsx_calls)
                || new_macro.attrs != old_macro.attrs
                || new_macro.semi_token != old_macro.semi_token
        }
        _ => true,
    }
}

fn find_rsx_expr(new_expr: &Expr, old_expr: &Expr, rsx_calls: &mut Vec<ChangedRsx>) -> bool {
    match (new_expr, old_expr) {
        (Expr::Array(new_expr), Expr::Array(old_expr)) => {
            if new_expr.elems.len() != old_expr.elems.len() {
                return true;
            }
            for (new_el, old_el) in new_expr.elems.iter().zip(old_expr.elems.iter()) {
                if find_rsx_expr(new_el, old_el, rsx_calls) {
                    return true;
                }
            }
            new_expr.attrs != old_expr.attrs || new_expr.bracket_token != old_expr.bracket_token
        }
        (Expr::Assign(new_expr), Expr::Assign(old_expr)) => {
            find_rsx_expr(&new_expr.left, &old_expr.left, rsx_calls)
                || find_rsx_expr(&new_expr.right, &old_expr.right, rsx_calls)
                || new_expr.attrs != old_expr.attrs
                || new_expr.eq_token != old_expr.eq_token
        }
        (Expr::Async(new_expr), Expr::Async(old_expr)) => {
            find_rsx_block(&new_expr.block, &old_expr.block, rsx_calls)
                || new_expr.attrs != old_expr.attrs
                || new_expr.async_token != old_expr.async_token
                || new_expr.capture != old_expr.capture
        }
        (Expr::Await(new_expr), Expr::Await(old_expr)) => {
            find_rsx_expr(&new_expr.base, &old_expr.base, rsx_calls)
                || new_expr.attrs != old_expr.attrs
                || new_expr.dot_token != old_expr.dot_token
                || new_expr.await_token != old_expr.await_token
        }
        (Expr::Binary(new_expr), Expr::Binary(old_expr)) => {
            find_rsx_expr(&new_expr.left, &old_expr.left, rsx_calls)
                || find_rsx_expr(&new_expr.right, &old_expr.right, rsx_calls)
                || new_expr.attrs != old_expr.attrs
                || new_expr.op != old_expr.op
        }
        (Expr::Block(new_expr), Expr::Block(old_expr)) => {
            find_rsx_block(&new_expr.block, &old_expr.block, rsx_calls)
                || new_expr.attrs != old_expr.attrs
                || new_expr.label != old_expr.label
        }
        (Expr::Break(new_expr), Expr::Break(old_expr)) => match (&new_expr.expr, &old_expr.expr) {
            (Some(new_inner), Some(old_inner)) => {
                find_rsx_expr(new_inner, old_inner, rsx_calls)
                    || new_expr.attrs != old_expr.attrs
                    || new_expr.break_token != old_expr.break_token
                    || new_expr.label != old_expr.label
            }
            (None, None) => {
                new_expr.attrs != old_expr.attrs
                    || new_expr.break_token != old_expr.break_token
                    || new_expr.label != old_expr.label
            }
            _ => true,
        },
        (Expr::Call(new_expr), Expr::Call(old_expr)) => {
            find_rsx_expr(&new_expr.func, &old_expr.func, rsx_calls);
            if new_expr.args.len() != old_expr.args.len() {
                return true;
            }
            for (new_arg, old_arg) in new_expr.args.iter().zip(old_expr.args.iter()) {
                if find_rsx_expr(new_arg, old_arg, rsx_calls) {
                    return true;
                }
            }
            new_expr.attrs != old_expr.attrs || new_expr.paren_token != old_expr.paren_token
        }
        (Expr::Cast(new_expr), Expr::Cast(old_expr)) => {
            find_rsx_expr(&new_expr.expr, &old_expr.expr, rsx_calls)
                || new_expr.attrs != old_expr.attrs
                || new_expr.as_token != old_expr.as_token
                || new_expr.ty != old_expr.ty
        }
        (Expr::Closure(new_expr), Expr::Closure(old_expr)) => {
            find_rsx_expr(&new_expr.body, &old_expr.body, rsx_calls)
                || new_expr.attrs != old_expr.attrs
                || new_expr.movability != old_expr.movability
                || new_expr.asyncness != old_expr.asyncness
                || new_expr.capture != old_expr.capture
                || new_expr.or1_token != old_expr.or1_token
                || new_expr.inputs != old_expr.inputs
                || new_expr.or2_token != old_expr.or2_token
                || new_expr.output != old_expr.output
        }
        (Expr::Const(new_expr), Expr::Const(old_expr)) => {
            find_rsx_block(&new_expr.block, &old_expr.block, rsx_calls)
                || new_expr.attrs != old_expr.attrs
                || new_expr.const_token != old_expr.const_token
        }
        (Expr::Continue(new_expr), Expr::Continue(old_expr)) => old_expr != new_expr,
        (Expr::Field(new_expr), Expr::Field(old_expr)) => {
            find_rsx_expr(&new_expr.base, &old_expr.base, rsx_calls)
                || new_expr.attrs != old_expr.attrs
                || new_expr.dot_token != old_expr.dot_token
                || new_expr.member != old_expr.member
        }
        (Expr::ForLoop(new_expr), Expr::ForLoop(old_expr)) => {
            find_rsx_block(&new_expr.body, &old_expr.body, rsx_calls)
                || find_rsx_expr(&new_expr.expr, &old_expr.expr, rsx_calls)
                || new_expr.attrs != old_expr.attrs
                || new_expr.label != old_expr.label
                || new_expr.for_token != old_expr.for_token
                || new_expr.pat != old_expr.pat
                || new_expr.in_token != old_expr.in_token
        }
        (Expr::Group(new_expr), Expr::Group(old_expr)) => {
            find_rsx_expr(&new_expr.expr, &old_expr.expr, rsx_calls)
        }
        (Expr::If(new_expr), Expr::If(old_expr)) => {
            if find_rsx_expr(&new_expr.cond, &old_expr.cond, rsx_calls)
                || find_rsx_block(&new_expr.then_branch, &old_expr.then_branch, rsx_calls)
            {
                return true;
            }
            match (&new_expr.else_branch, &old_expr.else_branch) {
                (Some((new_tok, new_else)), Some((old_tok, old_else))) => {
                    find_rsx_expr(new_else, old_else, rsx_calls)
                        || new_expr.attrs != old_expr.attrs
                        || new_expr.if_token != old_expr.if_token
                        || new_expr.cond != old_expr.cond
                        || new_tok != old_tok
                }
                (None, None) => {
                    new_expr.attrs != old_expr.attrs
                        || new_expr.if_token != old_expr.if_token
                        || new_expr.cond != old_expr.cond
                }
                _ => true,
            }
        }
        (Expr::Index(new_expr), Expr::Index(old_expr)) => {
            find_rsx_expr(&new_expr.expr, &old_expr.expr, rsx_calls)
                || find_rsx_expr(&new_expr.index, &old_expr.index, rsx_calls)
                || new_expr.attrs != old_expr.attrs
                || new_expr.bracket_token != old_expr.bracket_token
        }
        (Expr::Infer(new_expr), Expr::Infer(old_expr)) => new_expr != old_expr,
        (Expr::Let(new_expr), Expr::Let(old_expr)) => {
            find_rsx_expr(&new_expr.expr, &old_expr.expr, rsx_calls)
                || new_expr.attrs != old_expr.attrs
                || new_expr.let_token != old_expr.let_token
                || new_expr.pat != old_expr.pat
                || new_expr.eq_token != old_expr.eq_token
        }
        (Expr::Lit(new_expr), Expr::Lit(old_expr)) => old_expr != new_expr,
        (Expr::Loop(new_expr), Expr::Loop(old_expr)) => {
            find_rsx_block(&new_expr.body, &old_expr.body, rsx_calls)
                || new_expr.attrs != old_expr.attrs
                || new_expr.label != old_expr.label
                || new_expr.loop_token != old_expr.loop_token
        }
        (Expr::Macro(new_expr), Expr::Macro(old_expr)) => {
            find_rsx_macro(&new_expr.mac, &old_expr.mac, rsx_calls)
                || new_expr.attrs != old_expr.attrs
        }
        (Expr::Match(new_expr), Expr::Match(old_expr)) => {
            if find_rsx_expr(&new_expr.expr, &old_expr.expr, rsx_calls) {
                return true;
            }
            for (new_arm, old_arm) in new_expr.arms.iter().zip(old_expr.arms.iter()) {
                match (&new_arm.guard, &old_arm.guard) {
                    (Some((new_tok, new_expr)), Some((old_tok, old_expr))) => {
                        if find_rsx_expr(new_expr, old_expr, rsx_calls) || new_tok != old_tok {
                            return true;
                        }
                    }
                    (None, None) => (),
                    _ => return true,
                }
                if find_rsx_expr(&new_arm.body, &old_arm.body, rsx_calls)
                    || new_arm.attrs != old_arm.attrs
                    || new_arm.pat != old_arm.pat
                    || new_arm.fat_arrow_token != old_arm.fat_arrow_token
                    || new_arm.comma != old_arm.comma
                {
                    return true;
                }
            }
            new_expr.attrs != old_expr.attrs
                || new_expr.match_token != old_expr.match_token
                || new_expr.brace_token != old_expr.brace_token
        }
        (Expr::MethodCall(new_expr), Expr::MethodCall(old_expr)) => {
            if find_rsx_expr(&new_expr.receiver, &old_expr.receiver, rsx_calls) {
                return true;
            }
            for (new_arg, old_arg) in new_expr.args.iter().zip(old_expr.args.iter()) {
                if find_rsx_expr(new_arg, old_arg, rsx_calls) {
                    return true;
                }
            }
            new_expr.attrs != old_expr.attrs
                || new_expr.dot_token != old_expr.dot_token
                || new_expr.method != old_expr.method
                || new_expr.turbofish != old_expr.turbofish
                || new_expr.paren_token != old_expr.paren_token
        }
        (Expr::Paren(new_expr), Expr::Paren(old_expr)) => {
            find_rsx_expr(&new_expr.expr, &old_expr.expr, rsx_calls)
                || new_expr.attrs != old_expr.attrs
                || new_expr.paren_token != old_expr.paren_token
        }
        (Expr::Path(new_expr), Expr::Path(old_expr)) => old_expr != new_expr,
        (Expr::Range(new_expr), Expr::Range(old_expr)) => {
            match (&new_expr.start, &old_expr.start) {
                (Some(new_expr), Some(old_expr)) => {
                    if find_rsx_expr(new_expr, old_expr, rsx_calls) {
                        return true;
                    }
                }
                (None, None) => (),
                _ => return true,
            }
            match (&new_expr.end, &old_expr.end) {
                (Some(new_inner), Some(old_inner)) => {
                    find_rsx_expr(new_inner, old_inner, rsx_calls)
                        || new_expr.attrs != old_expr.attrs
                        || new_expr.limits != old_expr.limits
                }
                (None, None) => {
                    new_expr.attrs != old_expr.attrs || new_expr.limits != old_expr.limits
                }
                _ => true,
            }
        }
        (Expr::Reference(new_expr), Expr::Reference(old_expr)) => {
            find_rsx_expr(&new_expr.expr, &old_expr.expr, rsx_calls)
                || new_expr.attrs != old_expr.attrs
                || new_expr.and_token != old_expr.and_token
                || new_expr.mutability != old_expr.mutability
        }
        (Expr::Repeat(new_expr), Expr::Repeat(old_expr)) => {
            find_rsx_expr(&new_expr.expr, &old_expr.expr, rsx_calls)
                || find_rsx_expr(&new_expr.len, &old_expr.len, rsx_calls)
                || new_expr.attrs != old_expr.attrs
                || new_expr.bracket_token != old_expr.bracket_token
                || new_expr.semi_token != old_expr.semi_token
        }
        (Expr::Return(new_expr), Expr::Return(old_expr)) => {
            match (&new_expr.expr, &old_expr.expr) {
                (Some(new_inner), Some(old_inner)) => {
                    find_rsx_expr(new_inner, old_inner, rsx_calls)
                        || new_expr.attrs != old_expr.attrs
                        || new_expr.return_token != old_expr.return_token
                }
                (None, None) => {
                    new_expr.attrs != old_expr.attrs
                        || new_expr.return_token != old_expr.return_token
                }
                _ => true,
            }
        }
        (Expr::Struct(new_expr), Expr::Struct(old_expr)) => {
            match (&new_expr.rest, &old_expr.rest) {
                (Some(new_expr), Some(old_expr)) => {
                    if find_rsx_expr(new_expr, old_expr, rsx_calls) {
                        return true;
                    }
                }
                (None, None) => (),
                _ => return true,
            }
            for (new_field, old_field) in new_expr.fields.iter().zip(old_expr.fields.iter()) {
                if find_rsx_expr(&new_field.expr, &old_field.expr, rsx_calls)
                    || new_field.attrs != old_field.attrs
                    || new_field.member != old_field.member
                    || new_field.colon_token != old_field.colon_token
                {
                    return true;
                }
            }
            new_expr.attrs != old_expr.attrs
                || new_expr.path != old_expr.path
                || new_expr.brace_token != old_expr.brace_token
                || new_expr.dot2_token != old_expr.dot2_token
        }
        (Expr::Try(new_expr), Expr::Try(old_expr)) => {
            find_rsx_expr(&new_expr.expr, &old_expr.expr, rsx_calls)
                || new_expr.attrs != old_expr.attrs
                || new_expr.question_token != old_expr.question_token
        }
        (Expr::TryBlock(new_expr), Expr::TryBlock(old_expr)) => {
            find_rsx_block(&new_expr.block, &old_expr.block, rsx_calls)
                || new_expr.attrs != old_expr.attrs
                || new_expr.try_token != old_expr.try_token
        }
        (Expr::Tuple(new_expr), Expr::Tuple(old_expr)) => {
            for (new_el, old_el) in new_expr.elems.iter().zip(old_expr.elems.iter()) {
                if find_rsx_expr(new_el, old_el, rsx_calls) {
                    return true;
                }
            }
            new_expr.attrs != old_expr.attrs || new_expr.paren_token != old_expr.paren_token
        }
        (Expr::Unary(new_expr), Expr::Unary(old_expr)) => {
            find_rsx_expr(&new_expr.expr, &old_expr.expr, rsx_calls)
                || new_expr.attrs != old_expr.attrs
                || new_expr.op != old_expr.op
        }
        (Expr::Unsafe(new_expr), Expr::Unsafe(old_expr)) => {
            find_rsx_block(&new_expr.block, &old_expr.block, rsx_calls)
                || new_expr.attrs != old_expr.attrs
                || new_expr.unsafe_token != old_expr.unsafe_token
        }
        (Expr::While(new_expr), Expr::While(old_expr)) => {
            find_rsx_expr(&new_expr.cond, &old_expr.cond, rsx_calls)
                || find_rsx_block(&new_expr.body, &old_expr.body, rsx_calls)
                || new_expr.attrs != old_expr.attrs
                || new_expr.label != old_expr.label
                || new_expr.while_token != old_expr.while_token
        }
        (Expr::Yield(new_expr), Expr::Yield(old_expr)) => match (&new_expr.expr, &old_expr.expr) {
            (Some(new_inner), Some(old_inner)) => {
                find_rsx_expr(new_inner, old_inner, rsx_calls)
                    || new_expr.attrs != old_expr.attrs
                    || new_expr.yield_token != old_expr.yield_token
            }
            (None, None) => {
                new_expr.attrs != old_expr.attrs || new_expr.yield_token != old_expr.yield_token
            }
            _ => true,
        },
        (Expr::Verbatim(stream), Expr::Verbatim(stream2)) => {
            stream.to_string() != stream2.to_string()
        }
        _ => true,
    }
}

fn find_rsx_macro(
    new_mac: &syn::Macro,
    old_mac: &syn::Macro,
    rsx_calls: &mut Vec<ChangedRsx>,
) -> bool {
    if matches!(
        new_mac
            .path
            .get_ident()
            .map(|ident| ident.to_string())
            .as_deref(),
        Some("rsx" | "render")
    ) && matches!(
        old_mac
            .path
            .get_ident()
            .map(|ident| ident.to_string())
            .as_deref(),
        Some("rsx" | "render")
    ) {
        rsx_calls.push(ChangedRsx {
            old: old_mac.clone(),
            new: new_mac.tokens.clone(),
        });
        false
    } else {
        new_mac != old_mac
    }
}

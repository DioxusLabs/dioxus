use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{File, Macro};

pub enum DiffResult {
    CodeChanged,
    RsxChanged(Vec<(Macro, TokenStream)>),
}

/// Find any rsx calls in the given file and return a list of all the rsx calls that have changed.
pub fn find_rsx(new: &File, old: &File) -> DiffResult {
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
        return DiffResult::CodeChanged;
    }
    for (new, old) in new.items.iter().zip(old.items.iter()) {
        if find_rsx_item(new, old, &mut rsx_calls) {
            tracing::trace!(
                "found not hot reload-able change {:#?} != {:#?}",
                new.to_token_stream().to_string(),
                old.to_token_stream().to_string()
            );
            return DiffResult::CodeChanged;
        }
    }
    tracing::trace!("found hot reload-able changes {:#?}", rsx_calls);
    DiffResult::RsxChanged(rsx_calls)
}

fn find_rsx_item(
    new: &syn::Item,
    old: &syn::Item,
    rsx_calls: &mut Vec<(Macro, TokenStream)>,
) -> bool {
    match (new, old) {
        (syn::Item::Const(new_item), syn::Item::Const(old_item)) => {
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
        (syn::Item::Enum(new_item), syn::Item::Enum(old_item)) => {
            if new_item.variants.len() != old_item.variants.len() {
                return true;
            }
            for (new_varient, old_varient) in new_item.variants.iter().zip(old_item.variants.iter())
            {
                match (&new_varient.discriminant, &old_varient.discriminant) {
                    (Some((new_eq, new_expr)), Some((old_eq, old_expr))) => {
                        if find_rsx_expr(new_expr, old_expr, rsx_calls) || new_eq != old_eq {
                            return true;
                        }
                    }
                    (None, None) => (),
                    _ => return true,
                }
                if new_varient.attrs != old_varient.attrs
                    || new_varient.ident != old_varient.ident
                    || new_varient.fields != old_varient.fields
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
        (syn::Item::ExternCrate(new_item), syn::Item::ExternCrate(old_item)) => {
            old_item != new_item
        }
        (syn::Item::Fn(new_item), syn::Item::Fn(old_item)) => {
            find_rsx_block(&new_item.block, &old_item.block, rsx_calls)
                || new_item.attrs != old_item.attrs
                || new_item.vis != old_item.vis
                || new_item.sig != old_item.sig
        }
        (syn::Item::ForeignMod(new_item), syn::Item::ForeignMod(old_item)) => old_item != new_item,
        (syn::Item::Impl(new_item), syn::Item::Impl(old_item)) => {
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
        (syn::Item::Macro(new_item), syn::Item::Macro(old_item)) => {
            find_rsx_macro(&new_item.mac, &old_item.mac, rsx_calls)
                || new_item.attrs != old_item.attrs
                || new_item.semi_token != old_item.semi_token
                || new_item.ident != old_item.ident
        }
        (syn::Item::Mod(new_item), syn::Item::Mod(old_item)) => {
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
        (syn::Item::Static(new_item), syn::Item::Static(old_item)) => {
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
        (syn::Item::Struct(new_item), syn::Item::Struct(old_item)) => old_item != new_item,
        (syn::Item::Trait(new_item), syn::Item::Trait(old_item)) => {
            find_rsx_trait(new_item, old_item, rsx_calls)
        }
        (syn::Item::TraitAlias(new_item), syn::Item::TraitAlias(old_item)) => old_item != new_item,
        (syn::Item::Type(new_item), syn::Item::Type(old_item)) => old_item != new_item,
        (syn::Item::Union(new_item), syn::Item::Union(old_item)) => old_item != new_item,
        (syn::Item::Use(new_item), syn::Item::Use(old_item)) => old_item != new_item,
        (syn::Item::Verbatim(_), syn::Item::Verbatim(_)) => false,
        _ => true,
    }
}

fn find_rsx_trait(
    new_item: &syn::ItemTrait,
    old_item: &syn::ItemTrait,
    rsx_calls: &mut Vec<(Macro, TokenStream)>,
) -> bool {
    if new_item.items.len() != old_item.items.len() {
        return true;
    }
    for (new_item, old_item) in new_item.items.iter().zip(old_item.items.iter()) {
        if match (new_item, old_item) {
            (syn::TraitItem::Const(new_item), syn::TraitItem::Const(old_item)) => {
                if let (Some((_, new_expr)), Some((_, old_expr))) =
                    (&new_item.default, &old_item.default)
                {
                    find_rsx_expr(new_expr, old_expr, rsx_calls)
                } else {
                    true
                }
            }
            (syn::TraitItem::Fn(new_item), syn::TraitItem::Fn(old_item)) => {
                match (&new_item.default, &old_item.default) {
                    (Some(new_block), Some(old_block)) => {
                        find_rsx_block(new_block, old_block, rsx_calls)
                    }
                    (None, None) => false,
                    _ => true,
                }
            }
            (syn::TraitItem::Type(new_item), syn::TraitItem::Type(old_item)) => {
                old_item != new_item
            }
            (syn::TraitItem::Macro(new_item), syn::TraitItem::Macro(old_item)) => {
                old_item != new_item
            }
            (syn::TraitItem::Verbatim(stream), syn::TraitItem::Verbatim(stream2)) => {
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
    rsx_calls: &mut Vec<(Macro, TokenStream)>,
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

fn find_rsx_stmt(
    new_stmt: &syn::Stmt,
    old_stmt: &syn::Stmt,
    rsx_calls: &mut Vec<(Macro, TokenStream)>,
) -> bool {
    match (new_stmt, old_stmt) {
        (syn::Stmt::Local(new_local), syn::Stmt::Local(old_local)) => {
            (match (&new_local.init, &old_local.init) {
                (Some(new_local), Some(old_local)) => {
                    find_rsx_expr(&new_local.expr, &old_local.expr, rsx_calls)
                        || new_local != old_local
                }
                (None, None) => false,
                _ => true,
            } || new_local.attrs != old_local.attrs
                || new_local.let_token != old_local.let_token
                || new_local.pat != old_local.pat
                || new_local.semi_token != old_local.semi_token)
        }
        (syn::Stmt::Item(new_item), syn::Stmt::Item(old_item)) => {
            find_rsx_item(new_item, old_item, rsx_calls)
        }
        (syn::Stmt::Expr(new_expr, _), syn::Stmt::Expr(old_expr, _)) => {
            find_rsx_expr(new_expr, old_expr, rsx_calls)
        }
        (syn::Stmt::Macro(new_macro), syn::Stmt::Macro(old_macro)) => {
            find_rsx_macro(&new_macro.mac, &old_macro.mac, rsx_calls)
                || new_macro.attrs != old_macro.attrs
                || new_macro.semi_token != old_macro.semi_token
        }
        _ => true,
    }
}

fn find_rsx_expr(
    new_expr: &syn::Expr,
    old_expr: &syn::Expr,
    rsx_calls: &mut Vec<(Macro, TokenStream)>,
) -> bool {
    match (new_expr, old_expr) {
        (syn::Expr::Array(new_expr), syn::Expr::Array(old_expr)) => {
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
        (syn::Expr::Assign(new_expr), syn::Expr::Assign(old_expr)) => {
            find_rsx_expr(&new_expr.left, &old_expr.left, rsx_calls)
                || find_rsx_expr(&new_expr.right, &old_expr.right, rsx_calls)
                || new_expr.attrs != old_expr.attrs
                || new_expr.eq_token != old_expr.eq_token
        }
        (syn::Expr::Async(new_expr), syn::Expr::Async(old_expr)) => {
            find_rsx_block(&new_expr.block, &old_expr.block, rsx_calls)
                || new_expr.attrs != old_expr.attrs
                || new_expr.async_token != old_expr.async_token
                || new_expr.capture != old_expr.capture
        }
        (syn::Expr::Await(new_expr), syn::Expr::Await(old_expr)) => {
            find_rsx_expr(&new_expr.base, &old_expr.base, rsx_calls)
                || new_expr.attrs != old_expr.attrs
                || new_expr.dot_token != old_expr.dot_token
                || new_expr.await_token != old_expr.await_token
        }
        (syn::Expr::Binary(new_expr), syn::Expr::Binary(old_expr)) => {
            find_rsx_expr(&new_expr.left, &old_expr.left, rsx_calls)
                || find_rsx_expr(&new_expr.right, &old_expr.right, rsx_calls)
                || new_expr.attrs != old_expr.attrs
                || new_expr.op != old_expr.op
        }
        (syn::Expr::Block(new_expr), syn::Expr::Block(old_expr)) => {
            find_rsx_block(&new_expr.block, &old_expr.block, rsx_calls)
                || new_expr.attrs != old_expr.attrs
                || new_expr.label != old_expr.label
        }
        (syn::Expr::Break(new_expr), syn::Expr::Break(old_expr)) => {
            match (&new_expr.expr, &old_expr.expr) {
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
            }
        }
        (syn::Expr::Call(new_expr), syn::Expr::Call(old_expr)) => {
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
        (syn::Expr::Cast(new_expr), syn::Expr::Cast(old_expr)) => {
            find_rsx_expr(&new_expr.expr, &old_expr.expr, rsx_calls)
                || new_expr.attrs != old_expr.attrs
                || new_expr.as_token != old_expr.as_token
                || new_expr.ty != old_expr.ty
        }
        (syn::Expr::Closure(new_expr), syn::Expr::Closure(old_expr)) => {
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
        (syn::Expr::Const(new_expr), syn::Expr::Const(old_expr)) => {
            find_rsx_block(&new_expr.block, &old_expr.block, rsx_calls)
                || new_expr.attrs != old_expr.attrs
                || new_expr.const_token != old_expr.const_token
        }
        (syn::Expr::Continue(new_expr), syn::Expr::Continue(old_expr)) => old_expr != new_expr,
        (syn::Expr::Field(new_expr), syn::Expr::Field(old_expr)) => {
            find_rsx_expr(&new_expr.base, &old_expr.base, rsx_calls)
                || new_expr.attrs != old_expr.attrs
                || new_expr.dot_token != old_expr.dot_token
                || new_expr.member != old_expr.member
        }
        (syn::Expr::ForLoop(new_expr), syn::Expr::ForLoop(old_expr)) => {
            find_rsx_block(&new_expr.body, &old_expr.body, rsx_calls)
                || find_rsx_expr(&new_expr.expr, &old_expr.expr, rsx_calls)
                || new_expr.attrs != old_expr.attrs
                || new_expr.label != old_expr.label
                || new_expr.for_token != old_expr.for_token
                || new_expr.pat != old_expr.pat
                || new_expr.in_token != old_expr.in_token
        }
        (syn::Expr::Group(new_expr), syn::Expr::Group(old_expr)) => {
            find_rsx_expr(&new_expr.expr, &old_expr.expr, rsx_calls)
        }
        (syn::Expr::If(new_expr), syn::Expr::If(old_expr)) => {
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
        (syn::Expr::Index(new_expr), syn::Expr::Index(old_expr)) => {
            find_rsx_expr(&new_expr.expr, &old_expr.expr, rsx_calls)
                || find_rsx_expr(&new_expr.index, &old_expr.index, rsx_calls)
                || new_expr.attrs != old_expr.attrs
                || new_expr.bracket_token != old_expr.bracket_token
        }
        (syn::Expr::Infer(new_expr), syn::Expr::Infer(old_expr)) => new_expr != old_expr,
        (syn::Expr::Let(new_expr), syn::Expr::Let(old_expr)) => {
            find_rsx_expr(&new_expr.expr, &old_expr.expr, rsx_calls)
                || new_expr.attrs != old_expr.attrs
                || new_expr.let_token != old_expr.let_token
                || new_expr.pat != old_expr.pat
                || new_expr.eq_token != old_expr.eq_token
        }
        (syn::Expr::Lit(new_expr), syn::Expr::Lit(old_expr)) => old_expr != new_expr,
        (syn::Expr::Loop(new_expr), syn::Expr::Loop(old_expr)) => {
            find_rsx_block(&new_expr.body, &old_expr.body, rsx_calls)
                || new_expr.attrs != old_expr.attrs
                || new_expr.label != old_expr.label
                || new_expr.loop_token != old_expr.loop_token
        }
        (syn::Expr::Macro(new_expr), syn::Expr::Macro(old_expr)) => {
            find_rsx_macro(&new_expr.mac, &old_expr.mac, rsx_calls)
                || new_expr.attrs != old_expr.attrs
        }
        (syn::Expr::Match(new_expr), syn::Expr::Match(old_expr)) => {
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
        (syn::Expr::MethodCall(new_expr), syn::Expr::MethodCall(old_expr)) => {
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
        (syn::Expr::Paren(new_expr), syn::Expr::Paren(old_expr)) => {
            find_rsx_expr(&new_expr.expr, &old_expr.expr, rsx_calls)
                || new_expr.attrs != old_expr.attrs
                || new_expr.paren_token != old_expr.paren_token
        }
        (syn::Expr::Path(new_expr), syn::Expr::Path(old_expr)) => old_expr != new_expr,
        (syn::Expr::Range(new_expr), syn::Expr::Range(old_expr)) => {
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
        (syn::Expr::Reference(new_expr), syn::Expr::Reference(old_expr)) => {
            find_rsx_expr(&new_expr.expr, &old_expr.expr, rsx_calls)
                || new_expr.attrs != old_expr.attrs
                || new_expr.and_token != old_expr.and_token
                || new_expr.mutability != old_expr.mutability
        }
        (syn::Expr::Repeat(new_expr), syn::Expr::Repeat(old_expr)) => {
            find_rsx_expr(&new_expr.expr, &old_expr.expr, rsx_calls)
                || find_rsx_expr(&new_expr.len, &old_expr.len, rsx_calls)
                || new_expr.attrs != old_expr.attrs
                || new_expr.bracket_token != old_expr.bracket_token
                || new_expr.semi_token != old_expr.semi_token
        }
        (syn::Expr::Return(new_expr), syn::Expr::Return(old_expr)) => {
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
        (syn::Expr::Struct(new_expr), syn::Expr::Struct(old_expr)) => {
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
        (syn::Expr::Try(new_expr), syn::Expr::Try(old_expr)) => {
            find_rsx_expr(&new_expr.expr, &old_expr.expr, rsx_calls)
                || new_expr.attrs != old_expr.attrs
                || new_expr.question_token != old_expr.question_token
        }
        (syn::Expr::TryBlock(new_expr), syn::Expr::TryBlock(old_expr)) => {
            find_rsx_block(&new_expr.block, &old_expr.block, rsx_calls)
                || new_expr.attrs != old_expr.attrs
                || new_expr.try_token != old_expr.try_token
        }
        (syn::Expr::Tuple(new_expr), syn::Expr::Tuple(old_expr)) => {
            for (new_el, old_el) in new_expr.elems.iter().zip(old_expr.elems.iter()) {
                if find_rsx_expr(new_el, old_el, rsx_calls) {
                    return true;
                }
            }
            new_expr.attrs != old_expr.attrs || new_expr.paren_token != old_expr.paren_token
        }
        (syn::Expr::Unary(new_expr), syn::Expr::Unary(old_expr)) => {
            find_rsx_expr(&new_expr.expr, &old_expr.expr, rsx_calls)
                || new_expr.attrs != old_expr.attrs
                || new_expr.op != old_expr.op
        }
        (syn::Expr::Unsafe(new_expr), syn::Expr::Unsafe(old_expr)) => {
            find_rsx_block(&new_expr.block, &old_expr.block, rsx_calls)
                || new_expr.attrs != old_expr.attrs
                || new_expr.unsafe_token != old_expr.unsafe_token
        }
        (syn::Expr::While(new_expr), syn::Expr::While(old_expr)) => {
            find_rsx_expr(&new_expr.cond, &old_expr.cond, rsx_calls)
                || find_rsx_block(&new_expr.body, &old_expr.body, rsx_calls)
                || new_expr.attrs != old_expr.attrs
                || new_expr.label != old_expr.label
                || new_expr.while_token != old_expr.while_token
        }
        (syn::Expr::Yield(new_expr), syn::Expr::Yield(old_expr)) => {
            match (&new_expr.expr, &old_expr.expr) {
                (Some(new_inner), Some(old_inner)) => {
                    find_rsx_expr(new_inner, old_inner, rsx_calls)
                        || new_expr.attrs != old_expr.attrs
                        || new_expr.yield_token != old_expr.yield_token
                }
                (None, None) => {
                    new_expr.attrs != old_expr.attrs || new_expr.yield_token != old_expr.yield_token
                }
                _ => true,
            }
        }
        (syn::Expr::Verbatim(stream), syn::Expr::Verbatim(stream2)) => {
            stream.to_string() != stream2.to_string()
        }
        _ => true,
    }
}

fn find_rsx_macro(
    new_mac: &syn::Macro,
    old_mac: &syn::Macro,
    rsx_calls: &mut Vec<(Macro, TokenStream)>,
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
        rsx_calls.push((old_mac.clone(), new_mac.tokens.clone()));
        false
    } else {
        new_mac != old_mac
    }
}

//! Collect macros from a file
//!
//! Returns all macros that match a pattern. You can use this information to autoformat them later

use proc_macro2::LineColumn;
use syn::{Block, Expr, File, Item, Macro, Stmt};

type CollectedMacro<'a> = &'a Macro;

pub fn collect_from_file<'a>(file: &'a File, macros: &mut Vec<CollectedMacro<'a>>) {
    for item in file.items.iter() {
        collect_from_item(item, macros);
    }
}

pub fn collect_from_item<'a>(item: &'a Item, macros: &mut Vec<CollectedMacro<'a>>) {
    match item {
        Item::Fn(f) => collect_from_block(&f.block, macros),

        // Ignore macros if they're not rsx or render
        Item::Macro(macro_) => {
            if macro_.mac.path.segments[0].ident == "rsx"
                || macro_.mac.path.segments[0].ident == "render"
            {
                macros.push(&macro_.mac);
            }
        }

        // Currently disabled since we're not focused on autoformatting these
        Item::Impl(_imp) => {}
        Item::Trait(_) => {}

        // Global-ish things
        Item::Static(f) => collect_from_expr(&f.expr, macros),
        Item::Const(f) => collect_from_expr(&f.expr, macros),
        Item::Mod(s) => {
            if let Some((_, block)) = &s.content {
                for item in block {
                    collect_from_item(item, macros);
                }
            }
        }

        // None of these we can really do anything with at the item level
        Item::Macro2(_)
        | Item::Enum(_)
        | Item::ExternCrate(_)
        | Item::ForeignMod(_)
        | Item::TraitAlias(_)
        | Item::Type(_)
        | Item::Struct(_)
        | Item::Union(_)
        | Item::Use(_)
        | Item::Verbatim(_) => {}
        _ => {}
    }
}

pub fn collect_from_block<'a>(block: &'a Block, macros: &mut Vec<CollectedMacro<'a>>) {
    for stmt in &block.stmts {
        match stmt {
            Stmt::Item(item) => collect_from_item(item, macros),
            Stmt::Local(local) => {
                if let Some((_eq, init)) = &local.init {
                    collect_from_expr(init, macros);
                }
            }
            Stmt::Expr(exp) | Stmt::Semi(exp, _) => collect_from_expr(exp, macros),
        }
    }
}

pub fn collect_from_expr<'a>(expr: &'a Expr, macros: &mut Vec<CollectedMacro<'a>>) {
    // collect an expr from the exprs, descending into blocks
    match expr {
        Expr::Macro(macro_) => {
            if macro_.mac.path.segments[0].ident == "rsx"
                || macro_.mac.path.segments[0].ident == "render"
            {
                macros.push(&macro_.mac);
            }
        }

        Expr::MethodCall(e) => {
            collect_from_expr(&e.receiver, macros);
            for expr in e.args.iter() {
                collect_from_expr(expr, macros);
            }
        }
        Expr::Assign(exp) => {
            collect_from_expr(&exp.left, macros);
            collect_from_expr(&exp.right, macros);
        }

        Expr::Async(b) => collect_from_block(&b.block, macros),
        Expr::Block(b) => collect_from_block(&b.block, macros),
        Expr::Closure(c) => collect_from_expr(&c.body, macros),
        Expr::Let(l) => collect_from_expr(&l.expr, macros),
        Expr::Unsafe(u) => collect_from_block(&u.block, macros),
        Expr::Loop(l) => collect_from_block(&l.body, macros),

        Expr::Call(c) => {
            collect_from_expr(&c.func, macros);
            for expr in c.args.iter() {
                collect_from_expr(expr, macros);
            }
        }

        Expr::ForLoop(b) => {
            collect_from_expr(&b.expr, macros);
            collect_from_block(&b.body, macros);
        }
        Expr::If(f) => {
            collect_from_expr(&f.cond, macros);
            collect_from_block(&f.then_branch, macros);
            if let Some((_, else_branch)) = &f.else_branch {
                collect_from_expr(else_branch, macros);
            }
        }
        Expr::Yield(y) => {
            if let Some(expr) = &y.expr {
                collect_from_expr(expr, macros);
            }
        }

        Expr::Return(r) => {
            if let Some(expr) = &r.expr {
                collect_from_expr(expr, macros);
            }
        }

        Expr::Match(l) => {
            collect_from_expr(&l.expr, macros);
            for arm in l.arms.iter() {
                if let Some((_, expr)) = &arm.guard {
                    collect_from_expr(expr, macros);
                }

                collect_from_expr(&arm.body, macros);
            }
        }

        Expr::While(w) => {
            collect_from_expr(&w.cond, macros);
            collect_from_block(&w.body, macros);
        }

        // don't both formatting these for now
        Expr::Array(_)
        | Expr::AssignOp(_)
        | Expr::Await(_)
        | Expr::Binary(_)
        | Expr::Box(_)
        | Expr::Break(_)
        | Expr::Cast(_)
        | Expr::Continue(_)
        | Expr::Field(_)
        | Expr::Group(_)
        | Expr::Index(_)
        | Expr::Lit(_)
        | Expr::Paren(_)
        | Expr::Path(_)
        | Expr::Range(_)
        | Expr::Reference(_)
        | Expr::Repeat(_)
        | Expr::Struct(_)
        | Expr::Try(_)
        | Expr::TryBlock(_)
        | Expr::Tuple(_)
        | Expr::Type(_)
        | Expr::Unary(_)
        | Expr::Verbatim(_) => {}

        _ => {},
    };
}

pub fn byte_offset(input: &str, location: LineColumn) -> usize {
    let mut offset = 0;
    for _ in 1..location.line {
        offset += input[offset..].find('\n').unwrap() + 1;
    }
    offset
        + input[offset..]
            .chars()
            .take(location.column)
            .map(char::len_utf8)
            .sum::<usize>()
}

#[test]
fn parses_file_and_collects_rsx_macros() {
    let contents = include_str!("../tests/samples/long.rsx");
    let parsed = syn::parse_file(contents).unwrap();
    let mut macros = vec![];
    collect_from_file(&parsed, &mut macros);
    assert_eq!(macros.len(), 3);
}

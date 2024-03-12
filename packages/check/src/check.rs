use std::path::PathBuf;

use syn::{spanned::Spanned, visit::Visit, Pat};

use crate::{
    issues::{Issue, IssueReport},
    metadata::{
        AnyLoopInfo, ClosureInfo, ComponentInfo, ConditionalInfo, FnInfo, ForInfo, HookInfo,
        IfInfo, LoopInfo, MatchInfo, Span, WhileInfo,
    },
};

struct VisitHooks {
    issues: Vec<Issue>,
    context: Vec<Node>,
}

impl VisitHooks {
    const fn new() -> Self {
        Self {
            issues: vec![],
            context: vec![],
        }
    }
}

/// Checks a Dioxus file for issues.
pub fn check_file(path: PathBuf, file_content: &str) -> IssueReport {
    let file = syn::parse_file(file_content).unwrap();
    let mut visit_hooks = VisitHooks::new();
    visit_hooks.visit_file(&file);
    IssueReport::new(
        path,
        std::env::current_dir().unwrap_or_default(),
        file_content.to_string(),
        visit_hooks.issues,
    )
}

#[derive(Debug, Clone)]
enum Node {
    If(IfInfo),
    Match(MatchInfo),
    For(ForInfo),
    While(WhileInfo),
    Loop(LoopInfo),
    Closure(ClosureInfo),
    ComponentFn(ComponentInfo),
    HookFn(HookInfo),
    OtherFn(FnInfo),
}

fn returns_element(ty: &syn::ReturnType) -> bool {
    match ty {
        syn::ReturnType::Default => false,
        syn::ReturnType::Type(_, ref ty) => {
            if let syn::Type::Path(ref path) = **ty {
                if let Some(segment) = path.path.segments.last() {
                    if segment.ident == "Element" {
                        return true;
                    }
                }
            }
            false
        }
    }
}

fn is_hook_ident(ident: &syn::Ident) -> bool {
    ident.to_string().starts_with("use_")
}

fn is_component_fn(item_fn: &syn::ItemFn) -> bool {
    returns_element(&item_fn.sig.output)
}

fn get_closure_hook_body(local: &syn::Local) -> Option<&syn::Expr> {
    if let Pat::Ident(ident) = &local.pat {
        if is_hook_ident(&ident.ident) {
            if let Some((_, expr)) = &local.init {
                if let syn::Expr::Closure(closure) = &**expr {
                    return Some(&closure.body);
                }
            }
        }
    }

    None
}

fn fn_name_and_name_span(item_fn: &syn::ItemFn) -> (String, Span) {
    let name = item_fn.sig.ident.to_string();
    let name_span = item_fn.sig.ident.span().into();
    (name, name_span)
}

impl<'ast> syn::visit::Visit<'ast> for VisitHooks {
    fn visit_expr_call(&mut self, i: &'ast syn::ExprCall) {
        if let syn::Expr::Path(ref path) = *i.func {
            if let Some(segment) = path.path.segments.last() {
                if is_hook_ident(&segment.ident) {
                    let hook_info = HookInfo::new(
                        i.span().into(),
                        segment.ident.span().into(),
                        segment.ident.to_string(),
                    );
                    let mut container_fn: Option<Node> = None;
                    for node in self.context.iter().rev() {
                        match node {
                            Node::If(if_info) => {
                                let issue = Issue::HookInsideConditional(
                                    hook_info.clone(),
                                    ConditionalInfo::If(if_info.clone()),
                                );
                                self.issues.push(issue);
                            }
                            Node::Match(match_info) => {
                                let issue = Issue::HookInsideConditional(
                                    hook_info.clone(),
                                    ConditionalInfo::Match(match_info.clone()),
                                );
                                self.issues.push(issue);
                            }
                            Node::For(for_info) => {
                                let issue = Issue::HookInsideLoop(
                                    hook_info.clone(),
                                    AnyLoopInfo::For(for_info.clone()),
                                );
                                self.issues.push(issue);
                            }
                            Node::While(while_info) => {
                                let issue = Issue::HookInsideLoop(
                                    hook_info.clone(),
                                    AnyLoopInfo::While(while_info.clone()),
                                );
                                self.issues.push(issue);
                            }
                            Node::Loop(loop_info) => {
                                let issue = Issue::HookInsideLoop(
                                    hook_info.clone(),
                                    AnyLoopInfo::Loop(loop_info.clone()),
                                );
                                self.issues.push(issue);
                            }
                            Node::Closure(closure_info) => {
                                let issue = Issue::HookInsideClosure(
                                    hook_info.clone(),
                                    closure_info.clone(),
                                );
                                self.issues.push(issue);
                            }
                            Node::ComponentFn(_) | Node::HookFn(_) | Node::OtherFn(_) => {
                                container_fn = Some(node.clone());
                                break;
                            }
                        }
                    }

                    if let Some(Node::OtherFn(_)) = container_fn {
                        let issue = Issue::HookOutsideComponent(hook_info);
                        self.issues.push(issue);
                    }
                }
            }
        }
    }

    fn visit_item_fn(&mut self, i: &'ast syn::ItemFn) {
        let (name, name_span) = fn_name_and_name_span(i);
        if is_component_fn(i) {
            self.context.push(Node::ComponentFn(ComponentInfo::new(
                i.span().into(),
                name,
                name_span,
            )));
        } else if is_hook_ident(&i.sig.ident) {
            self.context.push(Node::HookFn(HookInfo::new(
                i.span().into(),
                i.sig.ident.span().into(),
                name,
            )));
        } else {
            self.context
                .push(Node::OtherFn(FnInfo::new(i.span().into(), name, name_span)));
        }
        syn::visit::visit_item_fn(self, i);
        self.context.pop();
    }

    fn visit_local(&mut self, i: &'ast syn::Local) {
        if let Some(body) = get_closure_hook_body(i) {
            // if the closure is a hook, we only visit the body of the closure.
            // this prevents adding a ClosureInfo node to the context
            syn::visit::visit_expr(self, body);
        } else {
            // otherwise visit the whole local
            syn::visit::visit_local(self, i);
        }
    }

    fn visit_expr_if(&mut self, i: &'ast syn::ExprIf) {
        self.context.push(Node::If(IfInfo::new(
            i.span().into(),
            i.if_token
                .span()
                .join(i.cond.span())
                .unwrap_or_else(|| i.span())
                .into(),
        )));
        syn::visit::visit_expr_if(self, i);
        self.context.pop();
    }

    fn visit_expr_match(&mut self, i: &'ast syn::ExprMatch) {
        self.context.push(Node::Match(MatchInfo::new(
            i.span().into(),
            i.match_token
                .span()
                .join(i.expr.span())
                .unwrap_or_else(|| i.span())
                .into(),
        )));
        syn::visit::visit_expr_match(self, i);
        self.context.pop();
    }

    fn visit_expr_for_loop(&mut self, i: &'ast syn::ExprForLoop) {
        self.context.push(Node::For(ForInfo::new(
            i.span().into(),
            i.for_token
                .span()
                .join(i.expr.span())
                .unwrap_or_else(|| i.span())
                .into(),
        )));
        syn::visit::visit_expr_for_loop(self, i);
        self.context.pop();
    }

    fn visit_expr_while(&mut self, i: &'ast syn::ExprWhile) {
        self.context.push(Node::While(WhileInfo::new(
            i.span().into(),
            i.while_token
                .span()
                .join(i.cond.span())
                .unwrap_or_else(|| i.span())
                .into(),
        )));
        syn::visit::visit_expr_while(self, i);
        self.context.pop();
    }

    fn visit_expr_loop(&mut self, i: &'ast syn::ExprLoop) {
        self.context
            .push(Node::Loop(LoopInfo::new(i.span().into())));
        syn::visit::visit_expr_loop(self, i);
        self.context.pop();
    }

    fn visit_expr_closure(&mut self, i: &'ast syn::ExprClosure) {
        self.context
            .push(Node::Closure(ClosureInfo::new(i.span().into())));
        syn::visit::visit_expr_closure(self, i);
        self.context.pop();
    }
}

#[cfg(test)]
mod tests {
    use crate::metadata::{
        AnyLoopInfo, ClosureInfo, ConditionalInfo, ForInfo, HookInfo, IfInfo, LineColumn, LoopInfo,
        MatchInfo, Span, WhileInfo,
    };
    use indoc::indoc;
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn test_no_hooks() {
        let contents = indoc! {r#"
            fn App() -> Element {
                rsx! {
                    p { "Hello World" }
                }
            }
        "#};

        let report = check_file("app.rs".into(), contents);

        assert_eq!(report.issues, vec![]);
    }

    #[test]
    fn test_hook_correctly_used_inside_component() {
        let contents = indoc! {r#"
            fn App() -> Element {
                let count = use_signal(|| 0);
                rsx! {
                    p { "Hello World: {count}" }
                }
            }
        "#};

        let report = check_file("app.rs".into(), contents);

        assert_eq!(report.issues, vec![]);
    }

    #[test]
    fn test_hook_correctly_used_inside_hook_fn() {
        let contents = indoc! {r#"
            fn use_thing() -> UseState<i32> {
                use_signal(|| 0)
            }
        "#};

        let report = check_file("use_thing.rs".into(), contents);

        assert_eq!(report.issues, vec![]);
    }

    #[test]
    fn test_hook_correctly_used_inside_hook_closure() {
        let contents = indoc! {r#"
            fn App() -> Element {
                let use_thing = || {
                    use_signal(|| 0)
                };
                let count = use_thing();
                rsx! {
                    p { "Hello World: {count}" }
                }
            }
        "#};

        let report = check_file("app.rs".into(), contents);

        assert_eq!(report.issues, vec![]);
    }

    #[test]
    fn test_conditional_hook_if() {
        let contents = indoc! {r#"
            fn App() -> Element {
                if you_are_happy && you_know_it {
                    let something = use_signal(|| "hands");
                    println!("clap your {something}")
                }
            }
        "#};

        let report = check_file("app.rs".into(), contents);

        assert_eq!(
            report.issues,
            vec![Issue::HookInsideConditional(
                HookInfo::new(
                    Span::new_from_str(
                        r#"use_signal(|| "hands")"#,
                        LineColumn { line: 3, column: 24 },
                    ),
                    Span::new_from_str(
                        r#"use_signal"#,
                        LineColumn { line: 3, column: 24 },
                    ),
                    "use_signal".to_string()
                ),
                ConditionalInfo::If(IfInfo::new(
                    Span::new_from_str(
                        "if you_are_happy && you_know_it {\n        let something = use_signal(|| \"hands\");\n        println!(\"clap your {something}\")\n    }",
                        LineColumn { line: 2, column: 4 },
                    ),
                    Span::new_from_str(
                        "if you_are_happy && you_know_it",
                        LineColumn { line: 2, column: 4 }
                    )
                ))
            )],
        );
    }

    #[test]
    fn test_conditional_hook_match() {
        let contents = indoc! {r#"
            fn App() -> Element {
                match you_are_happy && you_know_it {
                    true => {
                        let something = use_signal(|| "hands");
                        println!("clap your {something}")
                    }
                    false => {}
                }
            }
        "#};

        let report = check_file("app.rs".into(), contents);

        assert_eq!(
            report.issues,
            vec![Issue::HookInsideConditional(
                HookInfo::new(
                    Span::new_from_str(r#"use_signal(|| "hands")"#, LineColumn { line: 4, column: 28 }),
                    Span::new_from_str(r#"use_signal"#, LineColumn { line: 4, column: 28 }),
                    "use_signal".to_string()
                ),
                ConditionalInfo::Match(MatchInfo::new(
                    Span::new_from_str(
                        "match you_are_happy && you_know_it {\n        true => {\n            let something = use_signal(|| \"hands\");\n            println!(\"clap your {something}\")\n        }\n        false => {}\n    }",
                        LineColumn { line: 2, column: 4 },
                    ),
                    Span::new_from_str("match you_are_happy && you_know_it", LineColumn { line: 2, column: 4 })
                ))
            )]
        );
    }

    #[test]
    fn test_for_loop_hook() {
        let contents = indoc! {r#"
            fn App() -> Element {
                for _name in &names {
                    let is_selected = use_signal(|| false);
                    println!("selected: {is_selected}");
                }
            }
        "#};

        let report = check_file("app.rs".into(), contents);

        assert_eq!(
            report.issues,
            vec![Issue::HookInsideLoop(
                HookInfo::new(
                    Span::new_from_str(
                        "use_signal(|| false)",
                        LineColumn { line: 3, column: 26 },
                    ),
                    Span::new_from_str(
                        "use_signal",
                        LineColumn { line: 3, column: 26 },
                    ),
                    "use_signal".to_string()
                ),
                AnyLoopInfo::For(ForInfo::new(
                    Span::new_from_str(
                        "for _name in &names {\n        let is_selected = use_signal(|| false);\n        println!(\"selected: {is_selected}\");\n    }",
                        LineColumn { line: 2, column: 4 },
                    ),
                    Span::new_from_str(
                        "for _name in &names",
                        LineColumn { line: 2, column: 4 },
                    )
                ))
            )]
        );
    }

    #[test]
    fn test_while_loop_hook() {
        let contents = indoc! {r#"
            fn App() -> Element {
                while true {
                    let something = use_signal(|| "hands");
                    println!("clap your {something}")
                }
            }
        "#};

        let report = check_file("app.rs".into(), contents);

        assert_eq!(
            report.issues,
            vec![Issue::HookInsideLoop(
                HookInfo::new(
                    Span::new_from_str(
                        r#"use_signal(|| "hands")"#,
                        LineColumn { line: 3, column: 24 },
                    ),
                    Span::new_from_str(
                        "use_signal",
                        LineColumn { line: 3, column: 24 },
                    ),
                    "use_signal".to_string()
                ),
                AnyLoopInfo::While(WhileInfo::new(
                    Span::new_from_str(
                        "while true {\n        let something = use_signal(|| \"hands\");\n        println!(\"clap your {something}\")\n    }",
                        LineColumn { line: 2, column: 4 },
                    ),
                    Span::new_from_str(
                        "while true",
                        LineColumn { line: 2, column: 4 },
                    )
                ))
            )],
        );
    }

    #[test]
    fn test_loop_hook() {
        let contents = indoc! {r#"
            fn App() -> Element {
                loop {
                    let something = use_signal(|| "hands");
                    println!("clap your {something}")
                }
            }
        "#};

        let report = check_file("app.rs".into(), contents);

        assert_eq!(
            report.issues,
            vec![Issue::HookInsideLoop(
                HookInfo::new(
                    Span::new_from_str(
                        r#"use_signal(|| "hands")"#,
                        LineColumn { line: 3, column: 24 },
                    ),
                    Span::new_from_str(
                        "use_signal",
                        LineColumn { line: 3, column: 24 },
                    ),
                    "use_signal".to_string()
                ),
                AnyLoopInfo::Loop(LoopInfo::new(Span::new_from_str(
                    "loop {\n        let something = use_signal(|| \"hands\");\n        println!(\"clap your {something}\")\n    }",
                    LineColumn { line: 2, column: 4 },
                )))
            )],
        );
    }

    #[test]
    fn test_conditional_okay() {
        let contents = indoc! {r#"
            fn App() -> Element {
                let something = use_signal(|| "hands");
                if you_are_happy && you_know_it {
                    println!("clap your {something}")
                }
            }
        "#};

        let report = check_file("app.rs".into(), contents);

        assert_eq!(report.issues, vec![]);
    }

    #[test]
    fn test_closure_hook() {
        let contents = indoc! {r#"
            fn App() -> Element {
                let _a = || {
                    let b = use_signal(|| 0);
                    b.get()
                };
            }
        "#};

        let report = check_file("app.rs".into(), contents);

        assert_eq!(
            report.issues,
            vec![Issue::HookInsideClosure(
                HookInfo::new(
                    Span::new_from_str(
                        "use_signal(|| 0)",
                        LineColumn {
                            line: 3,
                            column: 16
                        },
                    ),
                    Span::new_from_str(
                        "use_signal",
                        LineColumn {
                            line: 3,
                            column: 16
                        },
                    ),
                    "use_signal".to_string()
                ),
                ClosureInfo::new(Span::new_from_str(
                    "|| {\n        let b = use_signal(|| 0);\n        b.get()\n    }",
                    LineColumn {
                        line: 2,
                        column: 13
                    },
                ))
            )]
        );
    }

    #[test]
    fn test_hook_outside_component() {
        let contents = indoc! {r#"
            fn not_component_or_hook() {
                let _a = use_signal(|| 0);
            }
        "#};

        let report = check_file("app.rs".into(), contents);

        assert_eq!(
            report.issues,
            vec![Issue::HookOutsideComponent(HookInfo::new(
                Span::new_from_str(
                    "use_signal(|| 0)",
                    LineColumn {
                        line: 2,
                        column: 13
                    }
                ),
                Span::new_from_str(
                    "use_signal",
                    LineColumn {
                        line: 2,
                        column: 13
                    },
                ),
                "use_signal".to_string()
            ))]
        );
    }

    #[test]
    fn test_hook_inside_hook() {
        let contents = indoc! {r#"
            fn use_thing() {
                let _a = use_signal(|| 0);
            }
        "#};

        let report = check_file("app.rs".into(), contents);

        assert_eq!(report.issues, vec![]);
    }
}
